use std::{collections::HashMap, pin::Pin, sync::Arc};

use futures::{lock::Mutex, FutureExt};
use serenity::{
    all::{ChannelId, GuildId, VoiceState},
    client::{Context, EventHandler},
    model::gateway::Ready,
};
use tokio::sync::Notify;
use tracing::instrument;

use crate::{event_handler, restarter::Restarter};

mod ready;
mod voice_state_update;

pub struct Handler {
    pub connected_channels: Arc<Mutex<HashMap<GuildId, ChannelId>>>,
    pub abort_controller: Arc<Notify>,
    pub restarter: Restarter,
}

impl EventHandler for Handler {
    #[instrument(skip_all)]
    fn ready<'s, 'async_trait>(&'s self, ctx: Context, ready: Ready) -> Pin<Box<(dyn Future<Output = ()> + Send + 'async_trait)>>
    where
        Self: 'async_trait,
        's: 'async_trait,
    {
        async move {
            if let Err(err) = event_handler::ready::handle(self, ctx, ready).await {
                tracing::error!("failed to handle ready event\nError: {err:?}");
            }
        }.boxed()
    }

    #[instrument(skip_all)]
    fn voice_state_update<'s, 'async_trait>(
        &'s self,
        context: Context,
        old_state: Option<VoiceState>,
        new_state: VoiceState,
    ) -> Pin<Box<(dyn Future<Output = ()> + Send + 'async_trait)>>
    where
        Self: 'async_trait,
        's: 'async_trait,
    {
        async move {
            if let Err(err) = event_handler::voice_state_update::handle(self, context, old_state, new_state).await {
                tracing::error!("failed to handle voice state update event\nError: {err:?}");
            }
        }.boxed()
    }
}
