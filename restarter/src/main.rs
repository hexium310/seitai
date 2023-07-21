use std::{collections::HashMap, env, sync::Arc};

use serenity::{
    all::{ChannelId, GuildId, UserId},
    client::Client,
    futures::lock::Mutex,
    model::gateway::GatewayIntents,
    prelude::TypeMapKey,
};
use tokio::sync::Notify;

mod event_handler;

struct Data {
    bot_id: UserId,
    connected_channels: HashMap<GuildId, ChannelId>,
    cancellation: Arc<Notify>,
}

impl TypeMapKey for Data {
    type Value = Arc<Mutex<Data>>;
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("`DISCORD_TOKEN` is not set.");

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(event_handler::Handler)
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;

        data.insert::<Data>(Arc::new(Mutex::new(Data {
            bot_id: UserId::default(),
            connected_channels: HashMap::new(),
            cancellation: Arc::new(Notify::default()),
        })));
    }

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
