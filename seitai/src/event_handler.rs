use std::{pin::Pin, sync::Arc};

use database::PgPool;
use futures::lock::Mutex;
use hashbrown::HashMap;
use serenity::{
    all::{ChannelId as SerenityChannelId, GuildId, VoiceState},
    client::{Context, EventHandler},
    model::{application::Interaction, channel::Message, gateway::Ready},
};
use songbird::input::Input;
use soundboard::sound::SoundId;
use tracing::instrument;

use crate::{
    audio::AudioRepository,
    time_keeper::TimeKeeper,
    speaker::Speaker,
};

mod message;
mod interaction_create;
mod ready;
mod voice_state_update;

#[derive(Debug)]
pub(crate) struct Handler<Repository> {
    pub(crate) database: PgPool,
    pub(crate) speaker: Speaker,
    pub(crate) audio_repository: Repository,
    pub(crate) connections: Arc<Mutex<HashMap<GuildId, SerenityChannelId>>>,
    pub(crate) time_keeper: Arc<Mutex<TimeKeeper<(GuildId, SoundId)>>>,
    pub(crate) kanatrans_host: String,
    pub(crate) kanatrans_port: u16,
}

impl<Repository> EventHandler for Handler<Repository>
where
    Repository: AudioRepository<Input = Input> + Send + Sync,
{
    fn interaction_create<'s, 'async_trait>(
        &'s self,
        context: Context,
        interaction: Interaction,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'async_trait>>
    where
        Self: 'async_trait,
        's: 'async_trait,
    {
        Box::pin(async move {
            if let Err(err) = interaction_create::handle(self, &context, &interaction).await {
                tracing::error!("{err:?}");
            }
        })
    }

    fn message<'s, 'async_trait>(
        &'s self,
        context: Context,
        message: Message,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'async_trait>>
    where
        Self: 'async_trait,
        's: 'async_trait,
    {
        Box::pin(async move {
            if let Err(err) = message::handle(self, context, message).await {
                tracing::error!("{err:?}");
            }
        })
    }

    #[instrument(skip(self, context))]
    fn ready<'s, 'async_trait>(
        &'s self,
        context: Context,
        ready: Ready,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'async_trait>>
    where
        Self: 'async_trait,
        's: 'async_trait,
    {
        Box::pin(async move {
            if let Err(err) = ready::handle(self, &context, &ready).await {
                tracing::error!("{err:?}");
            }
        })
    }

    fn voice_state_update<'s, 'async_trait>(
        &'s self,
        context: Context,
        old_state: Option<VoiceState>,
        new_state: VoiceState,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'async_trait>>
    where
        Self: 'async_trait,
        's: 'async_trait,
    {
        Box::pin(async move {
            if let Err(err) = voice_state_update::handle(self, &context, &old_state, &new_state).await {
                tracing::error!("{err:?}");
            }
        })
    }
}
