use std::{sync::Arc, time::Duration};

use anyhow::Result;
use futures::lock::Mutex;
use k8s_openapi::api::apps::v1::StatefulSet;
use kube::{Api, Client, Config};
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

impl<Restart: self::Restart + Clone + Send + 'static> Restarter<Restart> {
    pub(crate) fn new(duration: Duration, restart: Restart) -> Self {
        Self {
            duration,
            waiting: Arc::new(Mutex::new(false)),
            restart,
            connection_count: Arc::new(Mutex::new(0)),
        }
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn wait(&self) {
        let waiting = self.waiting.clone();

        if *waiting.lock().await {
            return;
        }

        let duration = self.duration;
        let restart = self.restart.clone();
        let connection_count = self.connection_count.clone();

        tracing::info!("statefulsets/voicevox is going to restart in {} secs", duration.as_secs());

        tokio::spawn(async move {
            *waiting.lock().await = true;

            tokio::time::sleep(duration).await;

            *waiting.lock().await = false;

            if *connection_count.lock().await != 0 {
                tracing::info!("cancelled restarting statefulsets/voicevox");

                return;
            }

            if let Err(err) = restart.restart().await {
                tracing::error!("failed to restart statefulsets/voicevox\nError: {err:?}");
            }
        }.in_current_span());
    }

    pub(crate) async fn set_connection_count(&self, count: usize) {
        *self.connection_count.lock().await = count;
    }
}
