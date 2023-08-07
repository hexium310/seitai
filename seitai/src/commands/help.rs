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
            "command" => match response_option.value.as_str().unwrap_or_default() {
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
                "dictionary" => Some(
                    CreateEmbed::new()
                        .title("/dictionary")
                        .description("辞書関連のコマンドです。")
                        .fields([
                            (
                                "add",
                                format!(
                                    "{}\n{}",
                                    "単語を辞書に追加します。任意で指定できる`音が下がる位置`については次のリンクを参照してください。",
                                    "https://tdmelodic.readthedocs.io/ja/latest/pages/introduction.html#representation-of-accent-nuclei-by-digits"
                                ).as_str(),
                                false
                            ),
                            ("list", "単語一覧を表示します。", true),
                            ("delete", "単語を削除します。", true),
                        ]),
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
                .fields([
                    ("/join", "ボイスチャンネルに接続します。", true),
                    ("/leave", "ボイスチャンネルから切断します。", true),
                    (
                        "/dictionary add",
                        &format!(
                            "{}\n{}",
                            "単語を辞書に追加します。任意で指定できる`音が下がる位置`については次のリンクを参照してください。",
                            "https://tdmelodic.readthedocs.io/ja/latest/pages/introduction.html#representation-of-accent-nuclei-by-digits"
                        ),
                        false,
                    ),
                    ("/dictionary list", "単語一覧を表示します。", true),
                    ("/dictionary delete", "単語を削除します。", true),
                ]),
        );
    }

    let message = CreateInteractionResponseMessage::new().embeds(embeds);
    respond(context, interaction, message).await?;
    Ok(())
}

pub fn register() -> CreateCommand {
    let channels = CreateCommandOption::new(CommandOptionType::String, "command", "chose command")
        .add_string_choice("join", "join")
        .add_string_choice("leave", "leave")
        .add_string_choice("dictionary", "dictionary");

    CreateCommand::new("help")
        .description("Specific command to show help about")
        .set_options(vec![channels])
}
