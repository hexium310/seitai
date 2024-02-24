use std::{borrow::Cow, future::Future, pin::Pin, sync::Arc};

use anyhow::Context as _;
use hashbrown::HashMap;
use lazy_regex::Regex;
use ordered_float::NotNan;
use serenity::{
    all::{ChannelId as SerenityChannelId, ChannelType, GuildId, VoiceState},
    client::{Context, EventHandler},
    futures::{future::join_all, lock::Mutex, StreamExt},
    model::{application::Interaction, channel::Message, gateway::Ready},
};
use songbird::{input::Input, Call};
use sqlx::PgPool;
use tracing::instrument;

use crate::{
    audio::{cache::PredefinedUtterance, Audio, AudioRepository},
    commands,
    database,
    regex,
    speaker::Speaker,
    utils::{get_manager, normalize},
};

#[derive(Debug)]
pub(crate) struct Handler<Repository> {
    pub(crate) database: PgPool,
    pub(crate) speaker: Speaker,
    pub(crate) audio_repository: Repository,
    pub(crate) connections: Arc<Mutex<HashMap<GuildId, SerenityChannelId>>>,
}

enum Replacement {
    General(&'static Regex, &'static str),
}

const SYSTEM_SPEAKER: &str = "1";

impl<Repository> EventHandler for Handler<Repository>
where
    Repository: AudioRepository<Input = Input> + Send + Sync,
{
    fn interaction_create<'s, 'async_trait>(
        &'s self,
        context: Context,
        interaction: Interaction,
    ) -> Pin<Box<(dyn Future<Output = ()> + Send + 'async_trait)>>
    where
        Self: 'async_trait,
        's: 'async_trait,
    {
        Box::pin(async move {
            match interaction {
                Interaction::Command(command) => {
                    let result = match command.data.name.as_str() {
                        "dictionary" => commands::dictionary::run(&context, &self.audio_repository, &command).await,
                        "help" => commands::help::run(&context, &command).await,
                        "join" => {
                            let mut connections = self.connections.lock().await;
                            commands::join::run(&context, &self.audio_repository, &mut connections, &command).await
                        },
                        "leave" => commands::leave::run(&context, &command).await,
                        "voice" => commands::voice::run(&context, &command, &self.database, &self.speaker).await,
                        _ => Ok(()),
                    }
                    .with_context(|| format!("failed to execute /{}", command.data.name));

                    if let Err(error) = result {
                        tracing::error!("failed to handle slash command\nError: {error:?}");
                    }
                },
                Interaction::Autocomplete(command) => {
                    let result = match command.data.name.as_str() {
                        "voice" => commands::voice::autocomplete(&context, &command, &self.speaker).await,
                        _ => Ok(()),
                    }
                    .with_context(|| format!("failed to autocomplete /{}", command.data.name));

                    if let Err(error) = result {
                        tracing::error!("failed to handle autocomplete of slash command\nError: {error:?}");
                    }
                },
                _ => {},
            }
        })
    }

    fn message<'s, 'async_trait>(
        &'s self,
        context: Context,
        message: Message,
    ) -> Pin<Box<(dyn Future<Output = ()> + Send + 'async_trait)>>
    where
        Self: 'async_trait,
        's: 'async_trait,
    {
        Box::pin(async move {
            if message.author.bot {
                return;
            }

            let Some(guild_id) = message.guild_id else {
                return;
            };

            let manager = match get_manager(&context).await {
                Ok(manager) => manager,
                Err(error) => {
                    tracing::error!("{error:?}");
                    return;
                },
            };
            let call = manager.get_or_insert(guild_id);
            let mut call = call.lock().await;

            let (Some(_), Some(channel_id_bot_at)) = (call.current_connection(), call.current_channel()) else {
                return;
            };
            let channel_id_bot_at = SerenityChannelId::from(channel_id_bot_at.0);

            let is_voice_channel_bot_at = {
                let connections = self.connections.lock().await;
                connections
                    .get(&guild_id)
                    .is_some_and(|channel_id| &message.channel_id == channel_id)
            };
            let is_text_channel_binded_to_bot = message.channel_id == channel_id_bot_at;

            if !is_voice_channel_bot_at && !is_text_channel_binded_to_bot {
                return;
            }

            let channel_bot_at = match channel_id_bot_at.to_channel(&context.http).await {
                Ok(channel_bot_at) => channel_bot_at,
                Err(error) => {
                    tracing::error!("failed to get channel: {channel_id_bot_at:?}\nError: {error:?}");
                    return;
                },
            };

            let serenity::all::Channel::Guild(channel_bot_at) = channel_bot_at else {
                return;
            };

            let members = match channel_bot_at.members(&context.cache) {
                Ok(members) => members,
                Err(error) => {
                    tracing::error!("failed to get members in channel: {channel_bot_at:?}\nError: {error:?}");
                    return;
                },
            };
            if !members
                .into_iter()
                .map(|member| member.user)
                .any(|user| message.author == user)
            {
                return;
            }

            let ids: Vec<i64> = vec![message.author.id.into()];
            let speaker = match database::user::fetch_by_ids(&self.database, &ids).await {
                Ok(users) => users
                    .first()
                    .unwrap_or(&database::User::default())
                    .speaker_id
                    .to_string(),
                Err(error) => {
                    tracing::error!("failed to fetch users by ids: {ids:?}\nError: {error:?}");
                    return;
                },
            };

            let default = database::UserSpeaker::default();
            let speed =
                match database::user::fetch_with_speaker_by_ids(&self.database, &[message.author.id.into()]).await {
                    Ok(speakers) => speakers
                        .first()
                        .unwrap_or(&default)
                        .speed
                        .or(default.speed)
                        .unwrap_or(1.2),
                    Err(error) => {
                        tracing::error!("failed to fetch speakers\nError: {error:?}");
                        return;
                    },
                };

            {
                for text in replace_message(&context, &message).split('\n') {
                    let text = text.trim();
                    if text.is_empty() {
                        continue;
                    }

                    let audio = Audio {
                        text: text.to_string(),
                        speaker: speaker.clone(),
                        speed: NotNan::new(speed).or(NotNan::new(Speaker::default_speed())).unwrap(),
                    };
                    match self.audio_repository.get(audio).await {
                        Ok(input) => {
                            call.enqueue_input(input).await;
                        },
                        Err(error) => {
                            tracing::error!("failed to get audio source\nError: {error:?}");
                        },
                    };
                }

                if !message.attachments.is_empty() {
                    let audio = Audio {
                        text: PredefinedUtterance::Attachment.as_ref().to_string(),
                        speaker: speaker.clone(),
                        speed: NotNan::new(speed).or(NotNan::new(Speaker::default_speed())).unwrap(),
                    };
                    match self.audio_repository.get(audio).await {
                        Ok(input) => {
                            call.enqueue_input(input).await;
                        },
                        Err(error) => {
                            tracing::error!("failed to get audio source\nError: {error:?}");
                        },
                    };
                }
            }
        })
    }

    #[instrument(skip(self, context))]
    fn ready<'s, 'async_trait>(
        &'s self,
        context: Context,
        ready: Ready,
    ) -> Pin<Box<(dyn Future<Output = ()> + Send + 'async_trait)>>
    where
        Self: 'async_trait,
        's: 'async_trait,
    {
        tracing::info!("{} is ready", ready.user.name);

        Box::pin(async move {
            for guild in ready.guilds {
                let commands = guild
                    .id
                    .set_commands(
                        &context.http,
                        vec![
                            commands::dictionary::register(),
                            commands::help::register(),
                            commands::join::register(),
                            commands::leave::register(),
                            commands::voice::register(),
                        ],
                    )
                    .await;

                if let Err(error) = commands {
                    tracing::error!("failed to regeister slash commands\nError: {error:?}");
                }
            }
        })
    }

    fn voice_state_update<'s, 'async_trait>(
        &'s self,
        context: Context,
        old_state: Option<VoiceState>,
        new_state: VoiceState,
    ) -> Pin<Box<(dyn Future<Output = ()> + Send + 'async_trait)>>
    where
        Self: 'async_trait,
        's: 'async_trait,
    {
        Box::pin(async move {
            let Some(guild_id) = new_state.guild_id else {
                return;
            };

            let manager = match get_manager(&context).await {
                Ok(manager) => manager,
                Err(error) => {
                    tracing::error!("{error:?}");
                    return;
                },
            };
            let call = manager.get_or_insert(guild_id);
            let mut call = call.lock().await;

            let bot_id = match context.http.get_current_user().await {
                Ok(bot) => bot.id,
                Err(error) => {
                    tracing::error!("failed to get current user on voice state update\nError: {error:?}");
                    return;
                },
            };

            let is_bot = new_state.user_id == bot_id;
            let is_disconnected = new_state.channel_id.is_none();

            if is_bot {
                if is_disconnected {
                    let mut connections = self.connections.lock().await;
                    connections.remove(&guild_id);
                }
                return;
            }

            let channel_id_bot_at = call
                .current_channel()
                .map(|channel_id| SerenityChannelId::from(channel_id.0));
            let newly_connected = match &old_state {
                Some(old_state) => old_state.channel_id != new_state.channel_id,
                None => true,
            };
            let is_connected_bot_at = new_state.channel_id == channel_id_bot_at;

            if !is_disconnected && newly_connected && is_connected_bot_at {
                let mut connections = self.connections.lock().await;
                handle_connect(&self.audio_repository, &new_state, &mut call, is_bot, &mut connections).await;
                return;
            }

            if let Some(channel_id_bot_at) = channel_id_bot_at {
                let channel = match channel_id_bot_at.to_channel(&context.http).await {
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
                let members = match channel.members(&context.cache) {
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
        })
    }
}

fn replace_message<'a>(context: &Context, message: &'a Message) -> Cow<'a, str> {
    let Some(guild_id) = message.guild_id else {
        return Cow::Borrowed(&message.content);
    };

    let replacements = [
        Replacement::General(&regex::CODE, "\nコード省略\n"),
        Replacement::General(&regex::URL, "\nURL\n"),
        Replacement::General(&regex::WW, "$1ワラワラ$2"),
        Replacement::General(&regex::W, "$1ワラ$2"),
        Replacement::General(&regex::IDEOGRAPHIC_FULL_STOP, "。\n"),
        Replacement::General(&regex::EMOJI, ":$1:"),
    ];

    let text = normalize(context, &guild_id, &message.mentions, &message.content);
    replacements
        .into_iter()
        .fold(text, |accumulator, replacement| match replacement {
            Replacement::General(regex, replacer) => match regex.replace_all(&accumulator, replacer) {
                Cow::Borrowed(borrowed) if borrowed.len() == accumulator.len() => accumulator,
                Cow::Borrowed(borrowed) => Cow::Owned(borrowed.to_owned()),
                Cow::Owned(owned) => Cow::Owned(owned),
            },
        })
}

async fn handle_connect<Repository>(
    audio_repository: &Repository,
    state: &VoiceState,
    call: &mut Call,
    is_bot: bool,
    connections: &mut HashMap<GuildId, SerenityChannelId>,
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

    let inputs = serenity::futures::stream::iter([user_is, connected].into_iter().flatten())
        .map(|text| async move {
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

    for input in join_all(inputs).await.into_iter().flatten() {
        call.enqueue_input(input).await;
    }
}
