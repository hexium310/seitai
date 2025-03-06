use std::{sync::Arc, time::Duration};

use anyhow::Result;
use futures::lock::Mutex;
use k8s_openapi::api::apps::v1::StatefulSet;
use kube::{Api, Client, Config};
use tokio::sync::mpsc::{self, error::SendError, Receiver, Sender};
use tokio_util::sync::CancellationToken;
use tracing::Instrument;

#[derive(Debug, Clone)]
pub(crate) struct Restarter<Restart = KubeRestarter> {
    duration: Duration,
    waiting: bool,
    tx: Sender<usize>,
    rx: Arc<Mutex<Receiver<usize>>>,
    restart: Restart,
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
        let (tx, rx) = mpsc::channel(1);

        Self {
            duration,
            waiting: false,
            tx,
            rx: Arc::new(Mutex::new(rx)),
            restart,
        }
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn wait(&self) {
        let duration = self.duration;
        let rx = self.rx.clone();
        let restart = self.restart.clone();
        let mut waiting = self.waiting;
        let cancellation_token = CancellationToken::new();

        tokio::spawn(async move {
            let mut rx = rx.lock().await;

            while let Some(connection_count) = rx.recv().await {
                match (connection_count, waiting) {
                    (0, true) => {
                        tracing::error!("unexpected connection count {connection_count} and waiting {waiting}");
                    },
                    (0, false) => {
                        waiting = true;
                        let cancellation_token = cancellation_token.clone();
                        let restart = restart.clone();

                        tracing::info!("statefulsets/voicevox is going to restart in {} secs", duration.as_secs());

                        tokio::spawn(async move {
                            tokio::select! {
                                _ = cancellation_token.cancelled() => {
                                    tracing::info!("cancelled restarting statefulsets/voicevox");
                                },
                                _ = tokio::time::sleep(duration) => {
                                    if let Err(err) = restart.restart().await {
                                        tracing::error!("failed to restart statefulsets/voicevox\nError: {err:?}");
                                    }
                                },
                            }
                        });
                    },
                    (1.., true) => {
                        waiting = false;
                        cancellation_token.cancel();
                    },
                    (1.., false) => (),
                }
            }
        }.in_current_span());
    }

    pub(crate) async fn send(&self, count: usize) -> Result<(), SendError<usize>> {
        self.tx.send(count).await
    }
}
