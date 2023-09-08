use std::{env, process::exit, sync::Arc, time::Duration};

use anyhow::{Context as _, Error, Result};
use logging::initialize_logging;
use serenity::{client::Client, futures::lock::Mutex, model::gateway::GatewayIntents, prelude::TypeMapKey};
use songbird::SerenityInit;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions,
    PgPool,
};
use tokio::signal::unix::{signal, SignalKind};
use tracing::log::LevelFilter;
use voicevox::Voicevox;

use crate::{audio::{processor::SongbirdAudioProcessor, VoicevoxAudioRepository}, speaker::Speaker};

mod audio;
mod character_converter;
mod commands;
mod database;
mod event_handler;
mod regex;
mod speaker;
mod utils;

struct VoicevoxClient;

impl TypeMapKey for VoicevoxClient {
    type Value = Arc<Mutex<Voicevox>>;
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

    let pool = match set_up_database().await {
        Ok(pool) => pool,
        Err(error) => {
            tracing::error!("failed to set up postgres\nError: {error:?}");
            exit(1);
        },
    };

    let voicevox = match set_up_voicevox().await {
        Ok(voicevox) => voicevox,
        Err(error) => {
            tracing::error!("failed to set up voicevox client\nError: {error:?}");
            exit(1);
        },
    };

    let speaker = match Speaker::build(&voicevox).await {
        Ok(speaker) => speaker,
        Err(error) => {
            tracing::error!("failed to build speaker\nError: {error:?}");
            exit(1);
        },
    };

    let audio_repository = VoicevoxAudioRepository::new(voicevox.audio_generator.clone(), SongbirdAudioProcessor);

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = match Client::builder(token, intents)
        .event_handler(event_handler::Handler {
            database: pool,
            speaker,
            audio_repository,
        })
        .register_songbird()
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

        data.insert::<VoicevoxClient>(Arc::new(Mutex::new(voicevox)));
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

async fn set_up_database() -> Result<PgPool> {
    let pg_options = PgConnectOptions::new()
        .log_statements(LevelFilter::Debug)
        .log_slow_statements(LevelFilter::Warn, Duration::from_millis(500));

    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(pg_options)
        .await
        .map_err(Error::msg)
}

async fn set_up_voicevox() -> Result<Voicevox> {
    let voicevox_host = env::var("VOICEVOX_HOST").context("failed to fetch environment variable VOICEVOX_HOST")?;
    Voicevox::build(&voicevox_host).context("failed to build voicevox client")
}
