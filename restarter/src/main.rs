use std::{collections::HashMap, env, process::exit, sync::Arc};

use futures::lock::Mutex;
use logging::initialize_logging;
use serenity::{
    all::{ChannelId, GuildId, UserId},
    client::Client,
    model::gateway::GatewayIntents,
    prelude::TypeMapKey,
};
use tokio::{
    signal::unix::{signal, SignalKind},
    sync::Notify,
};

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
    initialize_logging();

    let token = match env::var("DISCORD_TOKEN") {
        Ok(token) => token,
        Err(error) => {
            tracing::error!("failed to fetch environment variable DISCORD_TOKEN\nError: {error:?}");
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
            tracing::error!("failed to build serenity client\nError: {error:?}");
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

    tokio::spawn(async move {
        if let Err(error) = client.start().await {
            tracing::error!("failed to start client\nError: {error:?}");
            exit(1);
        }
    });

    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();

    tokio::select! {
        _ = sigint.recv() => {
            tracing::info!("received SIGINT, shutting down");
            exit(130);
        },
        _ = sigterm.recv() => {
            tracing::info!("received SIGTERM, shutting down");
            exit(143);
        },
    }
}
