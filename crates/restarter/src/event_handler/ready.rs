use anyhow::Result;
use serenity::all::{Context, Ready};

use super::Handler;

pub(crate) async fn handle(handler: &Handler, _ctx: Context, ready: Ready) -> Result<()> {
    tracing::info!("{} is ready", ready.user.name);

    handler.restarter.wait().await;

    Ok(())
}
