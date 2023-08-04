use regex_lite::Regex;
use serenity::{
    all::{ChannelId as SerenityChannelId, GuildId, UserId as SerenityUserId, VoiceState},
    async_trait,
    client::{Context, EventHandler},
    model::{application::Interaction, channel::Message, gateway::Ready},
};
use songbird::{input::Input, Call};

use crate::{
    commands,
    utils::{get_cached_audio, get_manager, get_voicevox, normalize},
};

pub struct Handler;

enum Replacing {
    General(Regex, String),
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
            };

            if let Err(why) = result {
                println!("{why}");
            }
        }
    }

    async fn message(&self, context: Context, message: Message) {
        if message.author.bot {
            return;
        }

        let guild_id = message.guild_id.unwrap();

        if !is_connected(&context, guild_id).await {
            return;
        }

        let manager = get_manager(&context).await.unwrap();
        let call = manager.get_or_insert(guild_id);
        let mut call = call.lock().await;

        let speaker = "1";

        for text in replace_message(&context, &message).split('\n') {
            let text = text.trim();
            if text.is_empty() {
                continue;
            }

            if let Some(input) = get_audio_source(&context, text, speaker).await {
                call.enqueue_input(input).await;
            }
        }
    }

    async fn ready(&self, context: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

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

            if let Err(why) = commands {
                println!("{why}");
            }
        }
    }

    async fn voice_state_update(&self, context: Context, old_state: Option<VoiceState>, new_state: VoiceState) {
        let Some(guild_id) = new_state.guild_id else {
            return;
        };

        let manager = get_manager(&context).await.unwrap();
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

        if !is_disconnected && newly_connected && (is_bot_connected || is_connected_bot_at) {
            handle_connect(&context, &new_state, &mut call, is_bot_connected).await;
            return;
        }
    }
}

async fn get_audio_source(context: &Context, text: &str, speaker: &str) -> Option<Input> {
    let audio_generator = {
        let voicevox = get_voicevox(context).await;
        let voicevox = voicevox.lock().await;
        voicevox.audio_generator.clone()
    };

    match text {
        "{{seitai::replacement::CODE}}" => get_cached_audio(context, "CODE").await,
        "{{seitai::replacement::URL}}" => get_cached_audio(context, "URL").await,
        _ => {
            let audio = match audio_generator.generate(speaker, text).await {
                Ok(audio) => audio,
                Err(why) => {
                    println!("Generating audio failed because of `{why}`");
                    return None;
                },
            };
            Some(Input::from(audio))
        },
    }
}

fn replace_message(context: &Context, message: &Message) -> String {
    let replacings = vec![
        Replacing::General(
            Regex::new(r"(?:`[^`]+`|```[^`]+```)").unwrap(),
            "\n{{seitai::replacement::CODE}}\n".to_string(),
        ),
        Replacing::General(
            Regex::new(r"[[:alpha:]][[:alnum:]+\-.]*?://[^\s]+").unwrap(),
            "\n{{seitai::replacement::URL}}\n".to_string(),
        ),
        Replacing::General(Regex::new(r"[wｗ]{2,}").unwrap(), "ワラワラ".to_string()),
        Replacing::General(Regex::new(r"[wｗ]$").unwrap(), "ワラ".to_string()),
        Replacing::General(Regex::new(r"。").unwrap(), "。\n".to_string()),
        Replacing::General(Regex::new(r"<:([\w_]+):\d+>").unwrap(), ":$1:".to_string()),
    ];

    let guild_id = message.guild_id.unwrap();
    let text = normalize(context, &guild_id, &message.mentions, &message.content);
    replacings.iter().fold(text, |accumulator, replacing| match replacing {
        Replacing::General(regex, replacement) => regex.replace_all(&accumulator, replacement).to_string(),
    })
}

async fn is_connected(context: &Context, guild_id: impl Into<GuildId>) -> bool {
    let manager = get_manager(context).await.unwrap();
    manager.get(guild_id.into()).is_some()
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
            let voicevox = get_voicevox(context).await;
            let voicevox = voicevox.lock().await;
            voicevox.audio_generator.clone()
        };
        let speaker = "1";
        let audio = match audio_generator.generate(speaker, &text).await {
            Ok(audio) => audio,
            Err(why) => {
                println!("Generating audio failed because of `{why}`");
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
