use anyhow::{Context as _, Result};
use database::{soundsticker, PgPool};
use serenity::{
    all::{CommandDataOptionValue, CommandOptionType, Sticker, StickerId},
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
use soundboard::{sound::SoundId, Soundboard, SoundboardExt};

use crate::{regex::SOUNDMOJI, utils::respond};

use super::subcommand::Subcommand;

pub(crate) async fn run(context: &Context, interaction: &CommandInteraction, database: &PgPool) -> Result<()> {
    let Some(guild_id) = interaction.guild_id else {
        return Ok(());
    };
    let subcommand = interaction.data.options.first().context("cannot get /soundsticker subcommand")?;
    let subcommand = Subcommand::from_command_data_option(subcommand).unwrap_or_default();

    let soundsticker = match subcommand.name {
        "set" => {
            let sticker_id = subcommand
                .options
                .get("sticker")
                .and_then(|v| v.as_str())
                .map(|v| v.parse::<u64>())
                .transpose()?
                .context("no sticker option")?;
            let sound_id = subcommand
                .options
                .get("sound")
                .and_then(|v| v.as_str())
                .map(|v| SOUNDMOJI.replace(v, "$1").parse::<u64>())
                .transpose()?
                .context("no sound option")?;

            soundsticker::create(database, sticker_id, sound_id).await?
        },
        _ => todo!(),
    };

    let sticker = StickerId::new(soundsticker.sticker_id).to_sticker(&context.http).await?;
    let sound = SoundId::new(soundsticker.sound_id).to_soundboard_sound(&context, guild_id).await?;
    let message = CreateInteractionResponseMessage::new().embed(
        CreateEmbed::new()
            .description(format!("ステッカー`{}`とサウンド`{}`を紐づけました。", sticker.name, sound.name))
            .colour(Colour::FOOYOO),
    );
    respond(context, interaction, &message).await?;

    Ok(())
}

pub fn register() -> CreateCommand {
    let set = {
        let sticker = CreateCommandOption::new(CommandOptionType::String, "sticker", "Sticker name")
            .name_localized("ja", "スタンプ")
            .description_localized("ja", "スタンプの名前")
            .set_autocomplete(true)
            .required(true);
        let sound = CreateCommandOption::new(CommandOptionType::String, "sound", "Sound name or Soundmoji")
            .name_localized("ja", "サウンド")
            .description_localized("ja", "サウンドの名前または Soundmoji")
            .set_autocomplete(true)
            .required(true);
        CreateCommandOption::new(CommandOptionType::SubCommand, "set", "Links sound to sticker")
            .description_localized("ja", "スタンプにサウンドを紐づけます")
            .add_sub_option(sticker)
            .add_sub_option(sound)
    };

    CreateCommand::new("soundsticker")
        .description("スタンプを投稿したときに再生されるサウンドボードのサウンドを管理します。")
        .set_options(vec![set])
}

pub(crate) async fn autocomplete(context: &Context, interaction: &CommandInteraction) -> Result<()> {
    let subcommand = interaction.data.options.first().context("cannot get /soundsticker subcommand")?;

    let options = match &subcommand.value {
        CommandDataOptionValue::SubCommand(options) => options,
        _ => return Ok(()),
    };

    let guild_id = interaction.guild_id.context("")?;

    for option in options {
        let value = match &option.value {
            CommandDataOptionValue::Autocomplete { value, .. } => value,
            _ => continue,
        };

        let autocomplete = match option.name.as_str() {
            "sticker" => {
                let stickers = guild_id.stickers(&context.http).await?;
                sticker_autocomplete(&stickers, value)
            },
            "sound" => {
                let soundboard = guild_id.soundboards(&context.http).await?;
                soundboard_autocomplete(soundboard, value)
            },
            _ => continue,
        };

        let error = format!("failed to create interaction response as autocomplete: {autocomplete:?}");
        interaction
            .create_response(&context.http, autocomplete)
            .await
            .context(error)?;
    }

    Ok(())
}

fn sticker_autocomplete(stickers: &[Sticker], value: &str) -> CreateInteractionResponse {
    let choices = stickers
        .iter()
        .filter_map(|sticker| {
            sticker
                .name
                .contains(value)
                .then_some(Some(AutocompleteChoice::new(sticker.name.clone(), sticker.id.to_string())))
        })
        .flatten()
        .take(25)
        .collect::<Vec<_>>();

    CreateInteractionResponse::Autocomplete(CreateAutocompleteResponse::new().set_choices(choices))
}

fn soundboard_autocomplete(soundboard: Soundboard, value: &str) -> CreateInteractionResponse {
    let choices = soundboard
        .items
        .iter()
        .filter_map(|sound| {
            sound
                .name
                .contains(value)
                .then_some(Some(AutocompleteChoice::new(sound.name.clone(), sound.sound_id.get().to_string())))
        })
        .flatten()
        .take(25)
        .collect::<Vec<_>>();

    CreateInteractionResponse::Autocomplete(CreateAutocompleteResponse::new().set_choices(choices))
}
