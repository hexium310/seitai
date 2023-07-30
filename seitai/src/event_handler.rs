use regex_lite::Regex;
use serenity::{
    all::{VoiceState, GuildId},
    async_trait,
    client::{Context, EventHandler},
    model::{application::Interaction, channel::Message, gateway::Ready},
    utils::{content_safe, ContentSafeOptions},
};
use songbird::input::Input;

use crate::{
    commands,
    utils::{get_cached_audio, get_manager},
    voicevox::generate_audio,
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

    async fn voice_state_update(&self, context: Context, _old: Option<VoiceState>, new: VoiceState) {
        let guild_id = match new.guild_id {
            Some(guild_id) => guild_id,
            None => {
                return;
            },
        };

        if !is_connected(&context, guild_id).await {
            return;
        }

        let bot_id = context.http.get_current_user().await.unwrap().id;
        let manager = get_manager(&context).await.unwrap();
        let call = manager.get_or_insert(guild_id);
        let mut call = call.lock().await;

        if new.channel_id.is_some() {
            let get_user_is = async {
                if new.user_id == bot_id {
                    return None;
                }

                let user = new.user_id.to_user(&context.http).await.unwrap();
                let name = user.nick_in(&context.http, guild_id).await.or(user.global_name).unwrap_or(user.name);
                let text = format!("{name}さんが");

                let speaker = "1";
                let audio = match generate_audio(speaker, &text).await {
                    Ok(audio) => audio,
                    Err(why) => {
                        println!("Generating audio failed because of `{why}`");
                        return None;
                    },
                };
                Some(Input::from(audio))
            };

            let (user_is, connected) = tokio::join!(get_user_is, get_cached_audio(&context, "connected"));
            for audio in [user_is, connected].into_iter().flatten() {
                call.enqueue_input(audio).await;
            }
        }
    }
}

async fn get_audio_source(context: &Context, text: &str, speaker: &str) -> Option<Input> {
    match text {
        "{{seitai::replacement::CODE}}" => get_cached_audio(context, "CODE").await,
        "{{seitai::replacement::URL}}" => get_cached_audio(context, "URL").await,
        _ => {
            let audio = match generate_audio(speaker, text).await {
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
        Replacing::General(Regex::new(r"[wｗ]$").unwrap(), "ワラ".to_string()),
        Replacing::General(Regex::new(r"[wｗ]{2,}").unwrap(), "ワラワラ".to_string()),
        Replacing::General(Regex::new(r"。").unwrap(), "。\n".to_string()),
        Replacing::General(Regex::new(r"<:([\w_]+):\d+>").unwrap(), "$1".to_string()),
    ];

    let guild_id = message.guild_id.unwrap();
    let content_safe_options = ContentSafeOptions::new()
        .clean_role(true)
        .clean_user(true)
        .clean_channel(true)
        .show_discriminator(false)
        .display_as_member_from(guild_id)
        .clean_here(false)
        .clean_everyone(false);

    replacings.iter().fold(
        content_safe(
            &context.cache,
            &message.content,
            &content_safe_options,
            &message.mentions,
        ),
        |accumulator, replacing| match replacing {
            Replacing::General(regex, replacement) => regex.replace_all(&accumulator, replacement).to_string(),
        },
    )
}

async fn is_connected(context: &Context, guild_id: impl Into<GuildId>) -> bool {
    let manager = get_manager(context).await.unwrap();
    manager.get(guild_id.into()).is_some()
}
