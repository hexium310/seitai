use std::convert::Infallible;

use anyhow::{Context as _, Result};
use ordered_float::NotNan;
use serenity::all::{ChannelType, Context, GuildId, VoiceState};
use songbird::input::Input;

use crate::{
    audio::{Audio, AudioRepository, cache::PredefinedUtterance},
    bot::Bot,
    event_handler::Handler,
    songbird_manager::SongbirdManager,
    speaker::Speaker,
};

#[allow(unused)]
struct VoiceStateUpdateHandler<'a, Repository> {
    event_handler: &'a Handler<Repository>,
    context: &'a Context,
    old_state: &'a Option<VoiceState>,
    new_state: &'a VoiceState,
    observer: VoiceStateObserver<'a>,
    change: VoiceStateChange<'a>,
}

#[derive(Clone)]
struct VoiceStateObserver<'a> {
    context: &'a Context,
    new_state: &'a VoiceState,
    songbird_manager: SongbirdManager<'a>,
}

struct VoiceStateChange<'a> {
    old_state: &'a Option<VoiceState>,
    new_state: &'a VoiceState,
    observer: VoiceStateObserver<'a>,
}

#[derive(Debug)]
enum VoiceStateKind {
    Joined,
    Left,
    Moved,
    Other,
    Unreachable,
}

const SYSTEM_SPEAKER: &str = "1";

impl<'a, Repository> VoiceStateUpdateHandler<'a, Repository>
where
    Repository: AudioRepository<Input = Input> + Send + Sync,
{
    fn new(event_handler: &'a Handler<Repository>, context: &'a Context, old_state: &'a Option<VoiceState>, new_state: &'a VoiceState) -> Self {
        let observer = VoiceStateObserver::new(context, new_state);
        let change = VoiceStateChange::new(old_state, new_state, observer.clone());

        Self { event_handler, context, old_state, new_state, observer, change }
    }

    async fn handle_joined(&self) -> Result<()> {
        let (Some(guild_id), Some(channel_id)) = (self.new_state.guild_id, self.new_state.channel_id) else {
            return Ok(());
        };

        let is_bot = self.change.is_bot().await;

        if is_bot {
            let mut connections = self.event_handler.connections.lock().await;
            connections.insert(guild_id, channel_id);
        }

        let user_is = (!is_bot)
            .then(|| {
                let member = self.new_state.member.as_ref()?;
                let user = &member.user;
                let name = member.nick.as_ref().or(user.global_name.as_ref()).unwrap_or(&user.name);
                Some(format!("{name}さんが"))
            })
            .flatten();
        let connected = Some(PredefinedUtterance::Connected.as_ref().to_string());

        for text in [user_is, connected].into_iter().flatten() {
            let audio = Audio {
                text,
                speaker: SYSTEM_SPEAKER.to_string(),
                speed: NotNan::new(Speaker::default_speed()).unwrap(),
            };

            self.observer.enqueue(audio, &self.event_handler.audio_repository).await?;
        }

        Ok(())
    }

    async fn handle_bot_left(&self) -> std::result::Result<(), Infallible> {
        let Some(guild_id) = self.new_state.guild_id else {
            return Ok(());
        };

        let mut connections = self.event_handler.connections.lock().await;
        connections.remove(&guild_id);

        Ok(())
    }

    async fn handle_left(&self) -> Result<()> {
        let Some(channel) = self.observer.channel().await? else {
            return Ok(());
        };

        if channel.kind != ChannelType::Voice {
            return Ok(());
        }

        let members = channel
            .members(&self.context.cache)
            .with_context(|| format!("failed to get members in channel {}", channel.id))?;
        let is_alone = members.iter().map(|v| v.user.id).eq([self.observer.id().await?].into_iter());

        if is_alone {
            let call = self.observer.call().await?;
            call.lock().await.leave().await.context("failed to leave when bot is alone in voice channel")?;
        }

        Ok(())
    }

    async fn handle(&self) -> Result<()> {
        match self.change.kind() {
            VoiceStateKind::Joined if self.new_state.channel_id == self.observer.channel_id().await? && !self.change.is_bot().await => self.handle_joined().await?,
            VoiceStateKind::Left if self.change.is_bot().await => self.handle_bot_left().await?,
            VoiceStateKind::Left => self.handle_left().await?,
            _ => (),
        }

        Ok(())
    }
}

impl<'a> VoiceStateChange<'a> {
    fn new(old_state: &'a Option<VoiceState>, new_state: &'a VoiceState, observer: VoiceStateObserver<'a>) -> Self {
        Self { old_state, new_state, observer }
    }

    #[tracing::instrument(
        level = "debug",
        ret,
        fields(
            user_id = ?self.new_state.user_id,
            guild_id = ?self.new_state.guild_id,
            channel_id = ?self.new_state.channel_id,
        ),
        skip_all,
    )]
    fn kind(&self) -> VoiceStateKind {
        match &self.old_state {
            Some(old_state) => match (old_state.channel_id, self.new_state.channel_id) {
                (Some(old_channel_id), Some(new_channel_id)) if old_channel_id != new_channel_id => VoiceStateKind::Moved,
                (Some(_), None) => VoiceStateKind::Left,
                _ => VoiceStateKind::Other,
            },
            None => match self.new_state.channel_id {
                Some(_) => VoiceStateKind::Joined,
                None => VoiceStateKind::Unreachable,
            },
        }
    }

    async fn is_bot(&self) -> bool {
        self.observer.id().await.is_ok_and(|bot_id| bot_id == self.new_state.user_id)
    }
}

impl<'a> VoiceStateObserver<'a> {
    fn new(context: &'a Context, new_state: &'a VoiceState) -> Self {
        Self {
            context,
            new_state,
            songbird_manager: SongbirdManager::new(context),
        }
    }
}

impl<'a> Bot for VoiceStateObserver<'a> {
    fn context(&self) -> &Context {
        self.context
    }

    fn guild_id(&self) -> Option<GuildId> {
        self.new_state.guild_id
    }

    fn songbird_manager(&self) -> &SongbirdManager<'_> {
        &self.songbird_manager
    }
}

pub(crate) async fn handle<Repository>(event_handler: &Handler<Repository>, context: &Context, old_state: &Option<VoiceState>, new_state: &VoiceState) -> Result<()>
where
    Repository: AudioRepository<Input = Input> + Send + Sync,
{
    let handle = VoiceStateUpdateHandler::new(event_handler, context, old_state, new_state);
    handle.handle().await
}
