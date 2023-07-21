use std::sync::Arc;

use anyhow::Result;
use k8s_openapi::api::apps::v1::StatefulSet;
use kube::{Api, Client};
use serenity::{
    all::VoiceState,
    async_trait,
    client::{Context, EventHandler},
    futures::lock::Mutex,
    model::gateway::Ready,
};
use tokio::sync::Notify;

use crate::Data;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, context: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let data = get_data(&context).await;
        data.lock().await.bot_id = ready.user.id;
    }

    async fn voice_state_update(&self, context: Context, _old: Option<VoiceState>, new: VoiceState) {
        let data = get_data(&context).await;
        let mut data = data.lock().await;
        let bot_id = data.bot_id;

        if new.user_id == bot_id {
            // When bot joined a voice channel
            if let (Some(channel_id), Some(guild_id)) = (new.channel_id, new.guild_id) {
                println!("{:#?}", data.cancellation);
                data.connected_channels.insert(guild_id, channel_id);
                data.cancellation.notify_one();
                println!("{:#?}", data.cancellation);
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

async fn get_data(context: &Context) -> Arc<Mutex<Data>> {
    let data = context.data.read().await;
    data.get::<crate::Data>().unwrap().clone()
}

async fn wait_restart(context: &Context) {
    let data = get_data(context).await;

    tokio::spawn(async move {
        let cancellation = {
            let mut data = data.lock().await;
            data.cancellation = Arc::new(Notify::new());
            data.cancellation.clone()
        };

        println!("waiting start");
        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(300)) => {
                if let Err(why) = restart().await {
                    println!("Restarting voicevox failed: {why}");
                    return;
                }
                println!("voicevox restarted");
            },
            _ = cancellation.notified() => {
                println!("Restarting voicevox is cancelled");
            },
        }
    });
}

async fn restart() -> Result<()> {
    let client = Client::try_default().await?;
    let stateful_sets: Api<StatefulSet> = Api::default_namespaced(client);
    stateful_sets.restart("voicevox").await?;

    Ok(())
}
