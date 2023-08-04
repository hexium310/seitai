use std::sync::Arc;

use anyhow::{Result, bail};
use k8s_openapi::api::apps::v1::StatefulSet;
use kube::{Api, Client, Config};
use serenity::{
    all::VoiceState,
    async_trait,
    client::{Context, EventHandler},
    futures::lock::Mutex,
    model::gateway::Ready,
};
use tokio::sync::Notify;
use tracing::{instrument, info, error};

use crate::Data;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    #[instrument(skip(self, context))]
    async fn ready(&self, context: Context, ready: Ready) {
        info!("{} is ready", ready.user.name);

        let Some(data) =  get_data(&context).await else{
            error!("failed to get data");
            return;
        };
        data.lock().await.bot_id = ready.user.id;
    }

    async fn voice_state_update(&self, context: Context, _old: Option<VoiceState>, new: VoiceState) {
        let Some(data) =  get_data(&context).await else{
            error!("failed to get data");
            return;
        };
        let mut data = data.lock().await;
        let bot_id = data.bot_id;

        if new.user_id == bot_id {
            // When bot joined a voice channel
            if let (Some(channel_id), Some(guild_id)) = (new.channel_id, new.guild_id) {
                data.connected_channels.insert(guild_id, channel_id);
                data.cancellation.notify_one();
            // When bot left a voice channel
            } else if let (None, Some(guild_id)) = (new.channel_id, new.guild_id) {
                data.connected_channels.remove(&guild_id);
                if data.connected_channels.is_empty() {
                    wait_restart(&context).await;
                }
            };
        }
    }
}

async fn get_data(context: &Context) -> Option<Arc<Mutex<Data>>> {
    let data = context.data.read().await;
    data.get::<crate::Data>().cloned()
}

async fn wait_restart(context: &Context) {
    let Some(data) =  get_data(context).await else{
        error!("failed to get data");
        return;
    };

    tokio::spawn(async move {
        let cancellation = {
            let mut data = data.lock().await;
            data.cancellation = Arc::new(Notify::new());
            data.cancellation.clone()
        };

        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(300)) => {
                if let Err(error) = restart().await {
                    error!("failed to restart statefulsets/voicevox\nError: {error:?}");
                    return;
                }
                info!("succeeded in restarting statefulsets/voicevox");
            },
            _ = cancellation.notified() => {
                info!("canceled restarting statefulsets/voicevox");
            },
        }
    });
}

async fn restart() -> Result<()> {
    let config = match Config::incluster() {
        Ok(config) => config,
        Err(_) => {
            bail!("error: this app is not running in cluster of Kubernetes");
        },
    };
    let client = Client::try_from(config)?;
    let stateful_sets: Api<StatefulSet> = Api::default_namespaced(client);
    stateful_sets.restart("voicevox").await?;

    Ok(())
}
