use std::sync::Arc;

use anyhow::{Context as _, Result};
use serenity::{
    builder::{CreateInteractionResponse, CreateInteractionResponseMessage},
    client::Context,
    model::{application::CommandInteraction, guild::Guild},
};
use songbird::Songbird;

pub(crate) async fn get_manager(context: &Context) -> Result<Arc<Songbird>> {
    songbird::get(context)
        .await
        .context("Songbird Voice client placed in at initialisation.")
}

pub(crate) fn get_guild(context: &Context, interaction: &CommandInteraction) -> Option<Guild> {
    let guild_id = interaction.guild_id.unwrap();
    guild_id.to_guild_cached(&context.cache).map(|guild| guild.to_owned())
}

pub(crate) async fn respond(
    context: &Context,
    interaction: &CommandInteraction,
    message: CreateInteractionResponseMessage,
) -> Result<()> {
    let builder = CreateInteractionResponse::Message(message);
    interaction.create_response(&context.http, builder).await?;

    Ok(())
}
