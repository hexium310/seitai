use std::env;

use serenity::{client::Client, model::gateway::GatewayIntents};
use songbird::SerenityInit;

mod commands;
mod event_handler;
mod utils;

pub struct Data {}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(event_handler::Handler)
        .register_songbird()
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
