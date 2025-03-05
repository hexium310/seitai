use std::time::Duration;

use anyhow::Result;
use k8s_openapi::api::apps::v1::StatefulSet;
use kube::{Api, Client, Config};
use tokio::sync::Notify;
use tracing::Instrument;

#[derive(Debug, Clone)]
pub(crate) struct Restarter {
    interval: u64,
}

impl Restarter {
    pub(crate) fn new(interval: u64) -> Self {
        Self { interval }
    }

    #[tracing::instrument]
    pub(crate) async fn restart() -> Result<()> {
        let config = match Config::incluster() {
            Ok(config) => config,
            Err(_) => {
                tracing::warn!("This app doesn't running in Kubernetes, so it didn't restart voicevox.");
                return Ok(())
            },
        };
        let client = Client::try_from(config)?;
        let stateful_sets: Api<StatefulSet> = Api::default_namespaced(client);

        stateful_sets.restart("voicevox").await?;

        tracing::info!("succeeded in restarting statefulsets/voicevox");

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub(crate) fn wait(&self) {
        let interval = Duration::from_secs(self.interval);

        tokio::spawn(async move {
            // Resets notify waiters because notified() is immediately received notify by notify_one() called before starting waiting.
            let notify = Notify::new();

            tokio::select! {
                _   = tokio::time::sleep(interval) => {
                    if let Err(err) = Self::restart().await {
                        tracing::error!("failed to restart statefulsets/voicevox\nError: {err:?}");
                    }
                },
                _ = notify.notified() => {
                    tracing::info!("canceled waiting for restarting statefulsets/voicevox");
                },
            }
        }.in_current_span());
    }

    pub(crate) fn abort(&self) {
        Notify::new().notify_one();
    }
}
