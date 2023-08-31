use std::borrow::Cow;

use anyhow::{Context as _, Result};
use lazy_regex::Regex;
use serenity::{
    all::{ChannelId as SerenityChannelId, ChannelType, VoiceState},
    async_trait,
    client::{Context, EventHandler},
    model::{application::Interaction, channel::Message, gateway::Ready},
};
use songbird::{input::Input, Call};
use sqlx::PgPool;
use tracing::instrument;
use voicevox::audio::AudioGenerator;

use crate::{
    commands,
    database,
    regex,
    speaker::Speaker,
    utils::{get_cached_audio, get_manager, get_voicevox, normalize},
};

#[derive(Debug)]
pub struct Handler {
    pub(crate) database: PgPool,
    pub(crate) speaker: Speaker,
}

enum Replacement {
    General(&'static Regex, &'static str),
}

const SYSTEM_SPEAKER: &str = "1";

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, context: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                let result = match command.data.name.as_str() {
                    "dictionary" => commands::dictionary::run(&context, &command).await,
                    "help" => commands::help::run(&context, &command).await,
                    "join" => commands::join::run(&context, &command).await,
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
    }

    async fn message(&self, context: Context, message: Message) {
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

        if call.current_connection().is_none() {
            return;
        };

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

        let audio_generator = {
            let Some(voicevox) = get_voicevox(&context).await else {
                tracing::error!("failed to get voicevox client to handle message");
                return;
            };
            let voicevox = voicevox.lock().await;
            voicevox.audio_generator.clone()
        };

        for text in replace_message(&context, &message).split('\n') {
            let text = text.trim();
            if text.is_empty() {
                continue;
            }

            match get_audio_source(&context, &audio_generator, text, &speaker).await {
                Ok(input) => {
                    call.enqueue_input(input).await;
                },
                Err(error) => {
                    tracing::error!("failed to get audio source\nError: {error:?}");
                },
            };
        }

        if !message.attachments.is_empty() {
            if let Some(audio) = get_cached_audio(&context, "attachment").await {
                call.enqueue_input(audio).await;
            }
        }
    }

    #[instrument(skip(self, context))]
    async fn ready(&self, context: Context, ready: Ready) {
        tracing::info!("{} is ready", ready.user.name);

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
    }

    async fn voice_state_update(&self, context: Context, old_state: Option<VoiceState>, new_state: VoiceState) {
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
        let is_bot_connected = new_state.user_id == bot_id;
        if is_bot_connected {
            return;
        }

        let channel_id_bot_at = call
            .current_channel()
            .map(|channel_id| SerenityChannelId::from(channel_id.0));
        let is_disconnected = new_state.channel_id.is_none();
        let newly_connected = match &old_state {
            Some(old_state) => old_state.channel_id != new_state.channel_id,
            None => true,
        };
        let is_connected_bot_at = new_state.channel_id == channel_id_bot_at;

        if !is_disconnected && newly_connected && is_connected_bot_at {
            handle_connect(&context, &new_state, &mut call, is_bot_connected).await;
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
                return;
            };
        }
    }
}

async fn get_audio_source(
    context: &Context,
    audio_generator: &AudioGenerator,
    text: &str,
    speaker: &str,
) -> Result<Input> {
    match text {
        "{{seitai::replacement::CODE}}" => get_cached_audio(context, "CODE")
            .await
            .context("failed to get cached audio \"CODE\""),
        "{{seitai::replacement::URL}}" => get_cached_audio(context, "URL")
            .await
            .context("failed to get cached audio \"URL\""),
        _ => {
            let audio = audio_generator
                .generate(speaker, text)
                .await
                .with_context(|| format!("failed to generate audio with {text}"))?;
            Ok(Input::from(audio))
        },
    }
}

fn replace_message<'a>(context: &Context, message: &'a Message) -> Cow<'a, str> {
    let Some(guild_id) = message.guild_id else {
        return Cow::Borrowed(&message.content);
    };

    let replacements = [
        Replacement::General(&regex::CODE, "\n{{seitai::replacement::CODE}}\n"),
        Replacement::General(&regex::URL, "\n{{seitai::replacement::URL}}\n"),
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

async fn handle_connect(context: &Context, state: &VoiceState, call: &mut Call, is_bot_connected: bool) {
    let get_user_is = async {
        if is_bot_connected {
            return None;
        }

        let member = state.member.as_ref()?;
        let user = &member.user;
        let name = member.nick.as_ref().or(user.global_name.as_ref()).unwrap_or(&user.name);
        let text = format!("{name}さんが");

        let audio_generator = {
            let Some(voicevox) = get_voicevox(context).await else {
                tracing::error!("failed to get voicevox client to handle connect");
                return None;
            };
            let voicevox = voicevox.lock().await;
            voicevox.audio_generator.clone()
        };
        let audio = match audio_generator.generate(SYSTEM_SPEAKER, &text).await {
            Ok(audio) => audio,
            Err(error) => {
                tracing::error!("failed to generate audio\nError: {error:?}");
                return None;
            },
        };
        Some(Input::from(audio))
    };

    let (user_is, connected) = tokio::join!(get_user_is, get_cached_audio(context, "connected"));
    for audio in [user_is, connected].into_iter().flatten() {
        call.enqueue_input(audio).await;
    }
}
