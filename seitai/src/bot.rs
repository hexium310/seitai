use std::sync::Arc;

use anyhow::{Context as _, Result};
use serenity::all::{ChannelId, Context, GuildChannel, GuildId, UserId};
use songbird::{Call, input::Input};
use tokio::sync::Mutex;

use crate::{
    audio::{Audio, AudioRepository},
    songbird_manager::SongbirdManager,
};

pub(crate) trait Bot {
    fn context(&self) -> &Context;

    fn songbird_manager(&self) -> &SongbirdManager<'_>;

    fn guild_id(&self) -> Option<GuildId>;

    async fn id(&self) -> Result<UserId> {
        Ok(self.context().http.get_current_user().await.context("failed to get bot user")?.id)
    }

    async fn channel_id(&self) -> Result<Option<ChannelId>> {
        let channel_id = self
            .call()
            .await?
            .lock()
            .await
            .current_channel()
            .map(|channel_id| ChannelId::from(channel_id.0));

        Ok(channel_id)
    }

    async fn channel(&self) -> Result<Option<GuildChannel>> {
        let Some(channel_id) = self.channel_id().await? else {
            return Ok(None);
        };

        let channel = channel_id
            .to_channel(&self.context().http)
            .await
            .with_context(|| format!("failed to get channel {channel_id}"))?
            .guild();

        Ok(channel)
    }

    async fn call(&self) -> Result<Arc<Mutex<Call>>> {
        let guild_id = self.guild_id().ok_or(anyhow::anyhow!("event without guild"))?;

        self.songbird_manager().call(guild_id).await
    }

    async fn enqueue(&self, audio: Audio, audio_repository: &impl AudioRepository<Input = Input>) -> Result<()> {
        let input = audio_repository
            .get(audio)
            .await
            .context("failed to get audio source")?;

        let call = self.call().await?;
        call.lock().await.enqueue_input(input).await;

        Ok(())
    }
}
