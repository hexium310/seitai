use anyhow::Result;
use serenity::all::{Context, Ready};

use super::Handler;

pub(crate) async fn handle(_handler: &Handler, _ctx: Context, ready: Ready) -> Result<()> {
    tracing::info!("{} is ready", ready.user.name);

    Ok(())
}
