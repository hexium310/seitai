use std::{fmt::Debug, sync::Arc, time::Duration};

use anyhow::Result;
use futures::lock::Mutex;
use k8s_openapi::api::apps::v1::StatefulSet;
use kube::{Api, Client, Config};
use tokio::{sync::oneshot, task::JoinHandle};
use tracing::Instrument;

#[derive(Debug, Clone)]
pub(crate) struct Restarter<Restart = KubeRestarter> {
    duration: Duration,
    waiting: Arc<Mutex<bool>>,
    restart: Restart,
    connection_count: Arc<Mutex<usize>>,
}

#[derive(Debug, Clone)]
pub(crate) struct KubeRestarter;

pub(crate) trait Restart: Send {
    fn restart(&self) -> impl Future<Output = Result<()>> + Send;
}

impl Restart for KubeRestarter {
    #[tracing::instrument]
    async fn restart(&self) -> Result<()> {
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
}

impl<Restart: self::Restart + Debug + Clone + Send + Sync + 'static> Restarter<Restart> {
    pub(crate) fn new(duration: Duration, restart: Restart) -> Self {
        Self {
            duration,
            waiting: Arc::new(Mutex::new(false)),
            restart,
            connection_count: Arc::new(Mutex::new(0)),
        }
    }

    #[allow(clippy::async_yields_async)]
    #[tracing::instrument(skip(self), fields(self.duration = ?self.duration, self.restart = ?self.restart))]
    pub(crate) async fn schedule_restart(&self) -> JoinHandle<()> {
        let waiting = self.waiting.clone();

        if *waiting.lock().await {
            return tokio::spawn(async {});
        }

        let duration = self.duration;
        let restart = self.restart.clone();
        let connection_count = self.connection_count.clone();

        tracing::info!("statefulsets/voicevox is going to restart in {} secs", duration.as_secs());

        let (tx, rx) = oneshot::channel();

        let handle = tokio::spawn(async move {
            tx.send(()).expect("failed to send notice that timer task spawned");
            *waiting.lock().await = true;

            tokio::time::sleep(duration).await;

            *waiting.lock().await = false;

            {
                let connection_count = *connection_count.lock().await;
                if connection_count != 0 {
                    tracing::info!("statefulsets/voicevox wasn't restart becase {connection_count} clients are connected");

                    return;
                }
            }

            if let Err(err) = restart.restart().await {
                tracing::error!("failed to restart statefulsets/voicevox\nError: {err:?}");
            }
        }.in_current_span());

        rx.await.expect("failed to receive notice that timer task spawned");
        handle
    }

    pub(crate) async fn set_connection_count(&self, count: usize) {
        *self.connection_count.lock().await = count;
    }
}

#[cfg(test)]
mod tests;
