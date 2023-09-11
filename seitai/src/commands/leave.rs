use anyhow::{Context as _, Result};
use serenity::{
    builder::{CreateCommand, CreateEmbed, CreateInteractionResponseMessage},
    client::Context,
    model::{application::CommandInteraction, Colour},
};

use crate::utils::{get_guild, get_manager, respond};

pub(crate) async fn run(context: &Context, interaction: &CommandInteraction) -> Result<()> {
    let guild = get_guild(context, interaction).context("failed to get guild")?;
    let manager = get_manager(context).await?;
    let call = manager.get_or_insert(guild.id);
    let mut call = call.lock().await;

    if call.current_connection().is_none() {
        let message = CreateInteractionResponseMessage::new().embed(
            CreateEmbed::new()
                .description("ボイスチャンネルに接続していません。")
                .colour(Colour::RED),
        );
        respond(context, interaction, &message).await?;
        return Ok(());
    }

    match call.leave().await {
        Ok(_) => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .description("ボイスチャンネルから切断しました。")
                    .colour(Colour::FOOYOO),
            );
            respond(context, interaction, &message).await?;
        },
        Err(error) => {
            tracing::error!("failed to disconnect from voice channel\nError: {error:?}");
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .description("ボイスチャンネルからの切断に失敗しました。")
                    .field("詳細", format!("```\n{}\n```", error), false)
                    .colour(Colour::RED),
            );
            respond(context, interaction, &message).await?;
        },
    };

    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("leave").description("ボイスチャンネルから切断します。")
}
