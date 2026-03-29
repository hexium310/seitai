use std::sync::Arc;

use anyhow::{Context as _, Result};
use serenity::all::{Context, GuildId};
use songbird::{Call, Songbird};
use tokio::sync::Mutex;

#[derive(Clone)]
pub(crate) struct SongbirdManager<'a> {
    context: &'a Context,
}

impl<'a> SongbirdManager<'a> {
    pub(crate) fn new(context: &'a Context) -> Self {
        Self { context }
    }

    pub(crate) async fn manager(&self) -> Result<Arc<Songbird>> {
        songbird::get(self.context)
            .await
            .context("failed to get songbird voice client")
    }

    pub(crate) async fn call(&self, guild_id: impl Into<GuildId>) -> Result<Arc<Mutex<Call>>> {
        let manager = self.manager().await?;
        Ok(manager.get_or_insert(guild_id.into()))
    }
}
