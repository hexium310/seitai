use anyhow::{Context as _, Result};
use serenity::{
    builder::{CreateCommand, CreateEmbed, CreateInteractionResponseMessage},
    client::Context,
    model::{application::CommandInteraction, Colour},
};

use crate::utils::{get_guild, get_manager, respond};

pub(crate) async fn run(context: &Context, interaction: &CommandInteraction) -> Result<()> {
    let guild = get_guild(context, interaction).context("A guild cannot be get")?;
    let manager = get_manager(context).await?;
    let has_handler = manager.get(guild.id).is_some();

    if !has_handler {
        let message = CreateInteractionResponseMessage::new().embed(
            CreateEmbed::new()
                .description("ボイスチャンネルに接続していません。")
                .colour(Colour::RED),
        );
        respond(context, interaction, message).await?;
        return Ok(());
    }

    manager.remove(guild.id).await?;

    let message = CreateInteractionResponseMessage::new().embed(
        CreateEmbed::new()
            .description("ボイスチャンネルから切断しました。")
            .colour(Colour::FOOYOO),
    );
    respond(context, interaction, message).await?;
    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("leave").description("ボイスチャンネルから切断します。")
}
