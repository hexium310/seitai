use anyhow::{Context as _, Result};
use regex_lite::Regex;
use serenity::{
    all::{ChannelId as SerenityChannelId, UserId as SerenityUserId, VoiceState},
    async_trait,
    client::{Context, EventHandler},
    model::{application::Interaction, channel::Message, gateway::Ready},
};
use songbird::{input::Input, Call};
use tracing::{debug, error, info};

use crate::{
    commands,
    utils::{get_cached_audio, get_manager, get_voicevox, normalize},
};

#[derive(Debug)]
pub struct Handler;

enum Replacing {
    General(std::result::Result<Regex, regex_lite::Error>, String),
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, context: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let result = match command.data.name.as_str() {
                "dictionary" => commands::dictionary::run(&context, &command).await,
                "help" => commands::help::run(&context, &command).await,
                "join" => commands::join::run(&context, &command).await,
                "leave" => commands::leave::run(&context, &command).await,
                _ => Ok(()),
            }
            .with_context(|| format!("failed to execute /{}", command.data.name));

            if let Err(error) = result {
                error!("error handling slash command\nError: {error:?}");
            }
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
                error!("{error:?}");
                return;
            },
        };
        let call = manager.get_or_insert(guild_id);
        let mut call = call.lock().await;

        if call.current_connection().is_none() {
            return;
        };

        let speaker = "1";

        for text in replace_message(&context, &message).split('\n') {
            let text = text.trim();
            if text.is_empty() {
                continue;
            }

            match get_audio_source(&context, text, speaker).await {
                Ok(input) => {
                    call.enqueue_input(input).await;
                },
                Err(error) => {
                    error!("failed to get audio source\nError: {error:?}");
                },
            };
        }
    }

    #[tracing::instrument(skip(self, context))]
    async fn ready(&self, context: Context, ready: Ready) {
        info!("{} is ready", ready.user.name);

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
                    ],
                )
                .await;

            if let Err(error) = commands {
                error!("failed to regeister slash commands\nError: {error:?}");
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
                error!("{error:?}");
                return;
            },
        };
        let call = manager.get_or_insert(guild_id);
        let mut call = call.lock().await;

        let Some(connection) = call.current_connection() else {
            return;
        };

        let channel_id_bot_at = connection
            .channel_id
            .map(|channel_id| SerenityChannelId::from(channel_id.0));
        let bot_id = SerenityUserId::from(connection.user_id.0);

        let is_disconnected = new_state.channel_id.is_none();
        let newly_connected = match &old_state {
            Some(old_state) => old_state.channel_id != new_state.channel_id,
            None => true,
        };
        let is_bot_connected = new_state.user_id == bot_id;
        let is_connected_bot_at = new_state.channel_id == channel_id_bot_at;

        if !is_disconnected && newly_connected && is_bot_connected || is_connected_bot_at {
            handle_connect(&context, &new_state, &mut call, is_bot_connected).await;
            return;
        }
    }
}

async fn get_audio_source(context: &Context, text: &str, speaker: &str) -> Result<Input> {
    let audio_generator = {
        let voicevox = get_voicevox(context).await.context("failed to get voicevox client")?;
        let voicevox = voicevox.lock().await;
        voicevox.audio_generator.clone()
    };

    match text {
        "{{seitai::replacement::CODE}}" => get_cached_audio(context, "CODE")
            .await
            .context("failed to get chaced audio: CODE"),
        "{{seitai::replacement::URL}}" => get_cached_audio(context, "URL")
            .await
            .context("failed to get chaced audio: URL"),
        _ => {
            let audio = audio_generator
                .generate(speaker, text)
                .await
                .with_context(|| format!("failed to generate audio with {text}"))?;
            Ok(Input::from(audio))
        },
    }
}

fn replace_message(context: &Context, message: &Message) -> String {
    let Some(guild_id) = message.guild_id else {
        return message.content.clone();
    };

    let replacings = vec![
        Replacing::General(
            Regex::new(r"(?:`[^`]+`|```[^`]+```)"),
            "\n{{seitai::replacement::CODE}}\n".to_string(),
        ),
        Replacing::General(
            Regex::new(r"[[:alpha:]][[:alnum:]+\-.]*?://[^\s]+"),
            "\n{{seitai::replacement::URL}}\n".to_string(),
        ),
        Replacing::General(Regex::new(r"[wｗ]$"), "ワラ".to_string()),
        Replacing::General(Regex::new(r"[wｗ]{2,}"), "ワラワラ".to_string()),
        Replacing::General(Regex::new(r"。"), "。\n".to_string()),
        Replacing::General(Regex::new(r"<:([\w_]+):\d+>"), ":$1:".to_string()),
    ];

    let text = normalize(context, &guild_id, &message.mentions, &message.content);
    replacings.iter().fold(text, |accumulator, replacing| match replacing {
        Replacing::General(regex, replacement) => {
            let regex = match regex {
                Ok(regex) => regex,
                Err(error) => {
                    debug!("error regex\nError: {error:?}");
                    return accumulator;
                },
            };
            regex.replace_all(&accumulator, replacement).to_string()
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
        let name = member
            .nick
            .clone()
            .or(user.global_name.clone())
            .unwrap_or(user.name.clone());
        let text = format!("{name}さんが");

        let audio_generator = {
            let Some(voicevox) = get_voicevox(context).await else {
                error!("failed to get voicevox client");
                return None;
            };
            let voicevox = voicevox.lock().await;
            voicevox.audio_generator.clone()
        };
        let speaker = "1";
        let audio = match audio_generator.generate(speaker, &text).await {
            Ok(audio) => audio,
            Err(error) => {
                error!("failed to generate audio\nError: {error:?}");
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
