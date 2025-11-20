use futures::{StreamExt, future, stream};
use hashbrown::HashMap;
use ordered_float::NotNan;
use serenity::all::{ChannelId, ChannelType, Context, GuildId, VoiceState};
use songbird::{Call, input::Input};

use crate::{
    audio::{Audio, AudioRepository, cache::PredefinedUtterance},
    event_handler::Handler,
    speaker::Speaker,
    utils,
};

struct VoiceStateUpdateHandler<'a, Repository> {
    event_handler: &'a Handler<Repository>,
    context: Context,
    old_state: Option<VoiceState>,
    new_state: VoiceState,
}

const SYSTEM_SPEAKER: &str = "1";

impl<'a, Repository> VoiceStateUpdateHandler<'a, Repository>
where
    Repository: AudioRepository<Input = Input> + Send + Sync,
{
    fn new(event_handler: &'a Handler<Repository>, context: Context, old_state: Option<VoiceState>, new_state: VoiceState) -> Self {
        Self { event_handler, context, old_state, new_state }
    }

    async fn handle(&self) {
        let Some(guild_id) = self.new_state.guild_id else {
            return;
        };

        let manager = match utils::get_manager(&self.context).await {
            Ok(manager) => manager,
            Err(error) => {
                tracing::error!("{error:?}");
                return;
            },
        };
        let call = manager.get_or_insert(guild_id);
        let mut call = call.lock().await;

        let bot_id = match self.context.http.get_current_user().await {
            Ok(bot) => bot.id,
            Err(error) => {
                tracing::error!("failed to get current user on voice state update\nError: {error:?}");
                return;
            },
        };

        let is_bot = self.new_state.user_id == bot_id;
        let is_disconnected = self.new_state.channel_id.is_none();

        if is_bot {
            if is_disconnected {
                let mut connections = self.event_handler.connections.lock().await;
                connections.remove(&guild_id);
            }
            return;
        }

        let channel_id_bot_at = call
            .current_channel()
            .map(|channel_id| ChannelId::from(channel_id.0));
        let newly_connected = match &self.old_state {
            Some(old_state) => old_state.channel_id != self.new_state.channel_id,
            None => true,
        };
        let is_connected_bot_at = self.new_state.channel_id == channel_id_bot_at;

        if !is_disconnected && newly_connected && is_connected_bot_at {
            let mut connections = self.event_handler.connections.lock().await;
            handle_connect(&self.event_handler.audio_repository, &self.new_state, &mut call, is_bot, &mut connections).await;
            return;
        }

        if let Some(channel_id_bot_at) = channel_id_bot_at {
            let channel = match channel_id_bot_at.to_channel(&self.context.http).await {
                Ok(channel) => channel.guild(),
                Err(error) => {
                    tracing::error!("failed to get channel {channel_id_bot_at} to check alone\nError: {error:?}");
                    return;
                },
            };
            let Some(channel) = channel else {
                return;
            };
            if channel.kind != ChannelType::Voice {
                return;
            }
            let members = match channel.members(&self.context.cache) {
                Ok(members) => members,
                Err(error) => {
                    tracing::error!(
                        "failed to get members in channel {channel_id_bot_at} to check alone\nError: {error:?}"
                    );
                    return;
                },
            };
            let ids = members.iter().map(|v| v.user.id).collect::<Vec<_>>();
            let is_alone = ids != vec![bot_id];
            if is_alone {
                return;
            }

            if let Err(error) = call.leave().await {
                tracing::error!("failed to leave when bot is alone in voice channel\n:Error {error:?}");
            };
        }
    }
}

pub(crate) async fn handle<Repository>(event_handler: &Handler<Repository>, context: Context, old_state: Option<VoiceState>, new_state: VoiceState)
where
    Repository: AudioRepository<Input = Input> + Send + Sync,
{
    let handle = VoiceStateUpdateHandler::new(event_handler, context, old_state, new_state);
    handle.handle().await;
}

async fn handle_connect<Repository>(
    audio_repository: &Repository,
    state: &VoiceState,
    call: &mut Call,
    is_bot: bool,
    connections: &mut HashMap<GuildId, ChannelId>,
) where
    Repository: AudioRepository<Input = Input> + Send + Sync,
{
    if is_bot {
        let (Some(guild_id), Some(channel_id)) = (state.guild_id, state.channel_id) else {
            return;
        };
        connections.insert(guild_id, channel_id);
    }

    let user_is = (!is_bot)
        .then(|| {
            let member = state.member.as_ref()?;
            let user = &member.user;
            let name = member.nick.as_ref().or(user.global_name.as_ref()).unwrap_or(&user.name);
            Some(format!("{name}さんが"))
        })
        .flatten();
    let connected = Some(PredefinedUtterance::Connected.as_ref().to_string());

    let inputs = stream::iter([user_is, connected].into_iter().flatten())
        .map(async |text| {
            let audio = Audio {
                text,
                speaker: SYSTEM_SPEAKER.to_string(),
                speed: NotNan::new(Speaker::default_speed()).unwrap(),
            };
            match audio_repository.get(audio).await {
                Ok(input) => Some(input),
                Err(error) => {
                    tracing::error!("failed to get audio source\nError: {error:?}");
                    None
                },
            }
        })
        .collect::<Vec<_>>()
        .await;

    for input in future::join_all(inputs).await.into_iter().flatten() {
        call.enqueue_input(input).await;
    }
}
