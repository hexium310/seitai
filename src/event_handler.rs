use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{application::Interaction, channel::Message, gateway::Ready},
};
use songbird::input::Input;

use crate::{
    commands,
    utils::get_manager,
    voicevox::{generate_audio, generate_audio_query},
};

pub struct Handler;

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
        let call = manager.get_or_insert(guild_id);
        let mut call = call.lock().await;

        let speaker = "1";
        let text = message.content;
        let json = match generate_audio_query(speaker, &text).await {
            Ok(json) => json,
            Err(why) => {
                println!("Generating audio query with `{text}` failed because of `{why}`.");
                return;
            },
        };
        let audio = match generate_audio(speaker, &json).await {
            Ok(audio) => audio,
            Err(why) => {
                println!("Generating audio failed because of `{why}`. The audio query used is {json}.");
                return;
            },
        };

        call.enqueue_input(Input::from(audio)).await;
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
