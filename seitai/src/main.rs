use std::{env, process::exit, sync::Arc};

use anyhow::Result;
use hashbrown::HashMap;
use logging::initialize_logging;
use serenity::{client::Client, futures::lock::Mutex, model::gateway::GatewayIntents, prelude::TypeMapKey};
use songbird::{
    driver::Bitrate,
    input::{cached::Compressed, File},
    SerenityInit,
};
use tokio::signal::unix::{signal, SignalKind};
use voicevox::Voicevox;

mod character_converter;
mod commands;
mod event_handler;
mod regex;
mod utils;

struct SoundStore;

impl TypeMapKey for SoundStore {
    type Value = Arc<Mutex<HashMap<String, Compressed>>>;
}

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

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = match Client::builder(token, intents)
        .event_handler(event_handler::Handler)
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

        let mut audio_map = HashMap::new();

        let resources = vec![
            ("CODE", "resources/code.wav"),
            ("URL", "resources/url.wav"),
            ("connected", "resources/connected.wav"),
        ];
        for resource in resources {
            let key = resource.0;
            let path = resource.1;

            let audio = match set_up_audio(path).await {
                Ok(audio) => audio,
                Err(error) => {
                    tracing::error!("failed to set up audio {path}\nError: {error:?}");
                    continue;
                },
            };
            audio_map.insert(key.into(), audio);
        }

        data.insert::<SoundStore>(Arc::new(Mutex::new(audio_map)));

        let voicevox_host = match env::var("VOICEVOX_HOST") {
            Ok(voicevox_host) => voicevox_host,
            Err(error) => {
                tracing::error!("failed to fetch environment variable VOICEVOX_HOST\nError: {error:?}");
                exit(1);
            },
        };
        let voicevox = match Voicevox::build(&voicevox_host) {
            Ok(voicevox) => voicevox,
            Err(error) => {
                tracing::error!("failed to build voicevox client\nError: {error:?}");
                exit(1);
            },
        };
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

async fn set_up_audio(path: &'static str) -> Result<Compressed> {
    let url = Compressed::new(File::new(path).into(), Bitrate::BitsPerSecond(128_000)).await?;
    let _ = url.raw.spawn_loader();

    Ok(url)
}
