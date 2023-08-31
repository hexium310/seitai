use anyhow::{Context as _, Result};
use serenity::{
    all::{CommandDataOptionValue, CommandOptionType},
    builder::{
        AutocompleteChoice,
        CreateAutocompleteResponse,
        CreateCommand,
        CreateCommandOption,
        CreateEmbed,
        CreateInteractionResponse,
        CreateInteractionResponseMessage,
    },
    client::Context,
    model::{application::CommandInteraction, Colour},
};
use sqlx::PgPool;

use crate::{database, speaker::Speaker, utils::respond};

pub(crate) async fn run(
    context: &Context,
    interaction: &CommandInteraction,
    database: &PgPool,
    speaker: &Speaker,
) -> Result<()> {
    let subcommand = interaction.data.options.first().context("cannot get subcommand")?;
    match subcommand.name.as_str() {
        "use" => {
            let speaker_id = u16::try_from(
                get_subcommand_option(&subcommand.value, "speaker")
                    .context("cannot get speaker id from `/voice use` argument")?
                    .as_i64()
                    .context(format!("{:?} is not integer", subcommand.value))?,
            )?;

            let speaker_id = u16::try_from(
                database::user::create(database, interaction.user.id.into(), speaker_id)
                    .await?
                    .speaker_id,
            )
            .context("failed to convert speaker_id to u16")?;
            let speaker_name = speaker.get_name(speaker_id)?;

            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("ボイスを変更しました。")
                    .description(speaker_name)
                    .colour(Colour::FOOYOO),
            );
            respond(context, interaction, &message).await?;
        },
        "reset" => {
            let speaker_id = u16::try_from(
                database::user::create(database, interaction.user.id.into(), 1)
                    .await?
                    .speaker_id,
            )
            .context("failed to convert speaker_id to u16")?;
            let speaker_name = speaker.get_name(speaker_id)?;

            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("ボイスを変更しました。")
                    .description(speaker_name)
                    .colour(Colour::FOOYOO),
            );
            respond(context, interaction, &message).await?;
        },
        _ => unreachable!(),
    }

    Ok(())
}

#[rustfmt::skip]
pub fn register() -> CreateCommand {
    let r#use = {
        let voice = CreateCommandOption::new(CommandOptionType::Integer, "speaker", "Voice to be used")
            .name_localized("ja", "ボイス")
            .description_localized("ja", "設定するボイス")
            .set_autocomplete(true)
            .required(true);
        CreateCommandOption::new(CommandOptionType::SubCommand, "use", "Sets voice that read aloud your message.")
            .description_localized("ja", "あなたのメッセージを読み上げるボイスを設定します。")
            .add_sub_option(voice)
    };

    let reset = {
        CreateCommandOption::new(CommandOptionType::SubCommand, "reset", "Resets voice that read aloud your message.")
            .description_localized("ja", "あなたのメッセージを読み上げるボイスをリセットします。")
    };

    CreateCommand::new("voice")
        .description("ボイスの設定を行います。")
        .set_options(vec![r#use, reset])
}

pub(crate) async fn autocomplete(context: &Context, interaction: &CommandInteraction, speaker: &Speaker) -> Result<()> {
    let subcommand = interaction.data.options.first().context("cannot get subcommand")?;
    let speaker_id = get_subcommand_option(&subcommand.value, "speaker").context("cannot get speaker from argument")?;

    if let CommandDataOptionValue::Autocomplete { value, .. } = &speaker_id {
        let choices = speaker
            .pairs()
            .filter_map(|(name_pairs, id)| {
                name_pairs.contains(value).then_some(Some(AutocompleteChoice::new(format!("{name_pairs}"), id)))
            })
            .flatten()
            .take(25)
            .collect::<Vec<_>>();
        let autocomplete =
            CreateInteractionResponse::Autocomplete(CreateAutocompleteResponse::new().set_choices(choices));
        let error = format!("failed to create interaction response as autocomplete: {autocomplete:?}");
        interaction
            .create_response(&context.http, autocomplete)
            .await
            .context(error)?;
    }

    Ok(())
}

fn get_subcommand_option<'a>(value: &'a CommandDataOptionValue, name: &str) -> Option<&'a CommandDataOptionValue> {
    match value {
        CommandDataOptionValue::SubCommand(options) => options
            .iter()
            .find(|option| option.name == name)
            .map(|option| &option.value),
        _ => None,
    }
}
