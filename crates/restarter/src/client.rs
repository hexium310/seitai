use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use futures::lock::Mutex;
use serenity::{all::GatewayIntents, Client as SerenityClient};
use tokio::{signal::unix::{self, SignalKind}, sync::Notify, task::JoinHandle};

use crate::{event_handler::Handler, restarter::Restarter};

pub struct Client;

impl Client {
    #[tracing::instrument(skip_all)]
    pub async fn start(token: String) -> Result<()> {
        enable_graceful_shutdown();

        let intents = GatewayIntents::GUILD_VOICE_STATES;
        let mut client = SerenityClient::builder(token, intents)
            .event_handler(Handler {
                connected_channels: Arc::new(Mutex::new(HashMap::new())),
                abort_controller: Arc::new(Notify::default()),
                restarter: Restarter::new(),
            })
            .await?;

        tracing::debug!("seitai client starts");
        if let Err(err) = client.start().await {
            tracing::error!("failed to start client\nError: {err:?}");
            return Err(err.into());
        }

        Ok(())
    }
}

fn enable_graceful_shutdown() -> JoinHandle<Result<()>> {
    tokio::spawn(async move {
        let mut sigint = unix::signal(SignalKind::interrupt())?;
        let mut sigterm = unix::signal(SignalKind::terminate())?;

        tokio::select! {
            _ = sigint.recv() => {
                tracing::info!("received SIGINT, shutting down");
                std::process::exit(130);
            },
            _ = sigterm.recv() => {
                tracing::info!("received SIGTERM, shutting down");
                std::process::exit(143);
            },
        }
    })
}
