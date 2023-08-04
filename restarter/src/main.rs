use std::{collections::HashMap, env, process::exit, sync::Arc};

use serenity::{
    all::{ChannelId, GuildId, UserId},
    client::Client,
    futures::lock::Mutex,
    model::gateway::GatewayIntents,
    prelude::TypeMapKey,
};
use tokio::sync::Notify;
use tracing::error;

use crate::logging::initialize_logging;

mod event_handler;
mod logging;

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
    initialize_logging();

    let token = match env::var("DISCORD_TOKEN") {
        Ok(token) => token,
        Err(error) => {
            error!("failed to fetch environment variable DISCORD_TOKEN\nError: {error:?}");
            exit(1);
        },
    };

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = match Client::builder(token, intents)
        .event_handler(event_handler::Handler)
        .await
    {
        Ok(client) => client,
        Err(error) => {
            error!("error creating serenity client\nError: {error:?}");
            exit(1);
        },
    };

    {
        let mut data = client.data.write().await;

        data.insert::<Data>(Arc::new(Mutex::new(Data {
            bot_id: UserId::default(),
            connected_channels: HashMap::new(),
            cancellation: Arc::new(Notify::default()),
        })));
    }

    if let Err(error) = client.start().await {
        error!("error starting client\nError: {error:?}");
        exit(1);
    }
}
