use regex_lite::Regex;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{application::Interaction, channel::Message, gateway::Ready},
};
use songbird::input::Input;

use crate::{
    commands,
    utils::{get_manager, get_sound_store},
    voicevox::{generate_audio, generate_audio_query},
};

pub struct Handler;

const DEFAULT_SPEED: f32 = 1.2;

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
        let manager = get_manager(&context).await.unwrap();

        // Returns when the bot is not connected to a voice channel
        let call = match manager.get(guild_id) {
            Some(call) => call,
            None => {
                return;
            },
        };
        let mut call = call.lock().await;

        let speaker = "1";

        for text in replace_message(&message.content).split('\n') {
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
}

async fn get_audio_source(context: &Context, text: &str, speaker: &str) -> Option<Input> {
    match text {
        "{{seitai::replacement::CODE}}" => {
            let sound_store = get_sound_store(context).await;
            let sound_store = sound_store.lock().await;
            let source = sound_store.get("CODE").unwrap();
            Some(source.new_handle().into())
        },
        "{{seitai::replacement::URL}}" => {
            let sound_store = get_sound_store(context).await;
            let sound_store = sound_store.lock().await;
            let source = sound_store.get("URL").unwrap();
            Some(source.new_handle().into())
        },
        _ => {
            let mut audio_query = match generate_audio_query(speaker, text).await {
                Ok(audio_query) => audio_query,
                Err(why) => {
                    println!("Generating audio query with `{text}` failed because of `{why}`.");
                    return None;
                },
            };

            // TODO: Truncate message too long
            audio_query.speed_scale = DEFAULT_SPEED + (text.len() / 50) as f32 * 0.1;

            let json = serde_json::to_string(&audio_query).unwrap();
            let audio = match generate_audio(speaker, &json).await {
                Ok(audio) => audio,
                Err(why) => {
                    println!("Generating audio failed because of `{why}`. The audio query used is {json}.");
                    return None;
                },
            };
            Some(Input::from(audio))
        },
    }
}

fn replace_message(message: &str) -> String {
        let replacings = vec![
            (Regex::new(r"(?:`[^`]+`|```[^`]+```)").unwrap(), "{{seitai::replacement::CODE}}"),
            (Regex::new(r"[[:alpha:]][[:alnum:]+\-.]*?://[^\s]+").unwrap(), "{{seitai::replacement::URL}}"),
        ];

        replacings.iter().fold(message.to_string(), |accumulator, replacing| {
            replacing.0.split(&accumulator).collect::<Vec<_>>().join(&format!("\n{}\n", replacing.1))
        })
}
