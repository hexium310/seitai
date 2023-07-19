use anyhow::Result;
use serenity::{
    builder::{CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponseMessage},
    client::Context,
    model::application::{CommandInteraction, CommandOptionType},
};

use crate::utils::respond;

pub async fn run(context: &Context, interaction: &CommandInteraction) -> Result<()> {
    let mut embeds = interaction
        .data
        .options
        .iter()
        .filter_map(|response_option| match response_option.name.as_str() {
            "command" => match response_option.value.as_str().unwrap() {
                "join" => Some(
                    CreateEmbed::new()
                        .title("/join")
                        .description("ボイスチャンネルに接続します。"),
                ),
                "leave" => Some(
                    CreateEmbed::new()
                        .title("/leave")
                        .description("ボイスチャンネルから切断します。"),
                ),
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>();

    if embeds.is_empty() {
        embeds.push(
            CreateEmbed::new()
                .title("help")
                .field("/join", "ボイスチャンネルに接続します。", true)
                .field("/leave", "ボイスチャンネルから切断します。", false),
        );
    }

    let message = CreateInteractionResponseMessage::new().embeds(embeds);
    respond(context, interaction, message).await?;
    Ok(())
}

pub fn register() -> CreateCommand {
    let channels = CreateCommandOption::new(CommandOptionType::String, "command", "chose command")
        .add_string_choice("join", "join")
        .add_string_choice("leave", "leave");

    CreateCommand::new("help")
        .description("Specific command to show help about")
        .set_options(vec![channels])
}
