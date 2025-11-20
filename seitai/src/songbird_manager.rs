use std::sync::Arc;

use anyhow::{Context as _, Result};
use serenity::all::{Context, GuildId};
use songbird::{Call, Songbird};
use tokio::sync::Mutex;

pub(crate) struct SongbirdManager;

impl SongbirdManager {
    pub(crate) async fn manager(&self, context: &Context) -> Result<Arc<Songbird>> {
        songbird::get(context)
            .await
            .context("failed to get songbird voice client")
    }

    pub(crate) async fn get_call(&self, context: &Context, guild_id: impl Into<GuildId>) -> Result<Arc<Mutex<Call>>> {
        let manager = self.manager(context).await?;
        Ok(manager.get_or_insert(guild_id.into()))
    }
}
