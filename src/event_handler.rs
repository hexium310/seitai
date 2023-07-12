use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{application::Interaction, channel::Message, gateway::Ready},
};

use crate::commands;

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

    async fn ready(&self, context: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        for guild in ready.guilds {
            println!("{guild:?}");
            let commands = guild
                .id
                .set_commands(
                    &context.http,
                    vec![
                        commands::help::register(&context, &guild.id).await.unwrap(),
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
