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
use soundboard::{Soundboard, SoundboardExt};

use crate::utils::{parse_soundmoji, respond};

use super::subcommand::Subcommand;

#[tracing::instrument(skip_all)]
pub(crate) async fn run(context: &Context, interaction: &CommandInteraction, database: &PgPool) -> Result<()> {
    let Some(guild_id) = interaction.guild_id else {
        return Ok(());
    };
    let subcommand = interaction.data.options.first().context("cannot get /soundsticker subcommand")?;
    let subcommand = Subcommand::from_command_data_option(subcommand).unwrap_or_default();

    let sticker_id = subcommand
        .options
        .get("sticker")
        .and_then(|v| v.as_str())
        .map(|v| v.parse::<u64>())
        .transpose()?
        .map(StickerId::new);
    let sound = subcommand
        .options
        .get("sound")
        .and_then(|v| v.as_str());

    let (sound_id, sound_guild_id) = match sound {
        Some(sound) => match parse_soundmoji(sound)? {
            (None, guild_id) => (Some(sound.parse::<u64>()?.into()), guild_id),
            soundmoji => soundmoji,
        },
        None => (None, None),
    };

    match subcommand.name {
        "link" => {
            let sticker_id = sticker_id.context("no sticker option")?;
            let sound_id = sound_id.context("no sound id")?;
            let sound_guild_id = sound_guild_id.unwrap_or(guild_id);

            let sticker = sticker_id.to_sticker(&context.http).await?;
            let sound = sound_id.to_soundboard_sound(&context.http, sound_guild_id).await?;

            soundsticker::create(
                database,
                &sticker.name,
                sticker.id.get(),
                sticker.guild_id.map(|v| v.get()),
                &sound.name,
                sound.sound_id.get(),
                sound.guild_id.map(|v| v.get()),
            ).await?;

            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .description(format!("スタンプ`{}`とサウンド`{}`を紐づけました。", sticker.name, sound.name))
                    .colour(Colour::FOOYOO),
            );
            respond(context, interaction, &message).await?;
        },
        "list" => {
            let stickers = soundsticker::fetch_all(database)
                .await?
                .into_iter()
                .map(|v| (format!(":frame_photo: {}", v.sticker_name), format!(":sound: {}", v.sound_name), true));

            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .fields(stickers)
                    .colour(Colour::FOOYOO),
            );
            respond(context, interaction, &message).await?;
        },
        "delete" => {
            let sticker_id = sticker_id.context("no sticker option")?;
            let deleted_soundsticker = match soundsticker::delete_by_id(database, sticker_id.get()).await? {
                Some(deleted) => deleted,
                None => return Ok(()),
            };

            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .description(format!(
                        "スタンプ`{}`とサウンド`{}`の紐づけを解除しました。",
                        deleted_soundsticker.sticker_name,
                        deleted_soundsticker.sound_name
                    ))
                    .colour(Colour::FOOYOO),
            );
            respond(context, interaction, &message).await?;
        },
        _ => unreachable!(),
    }

    Ok(())
}

pub fn register() -> CreateCommand {
    let link = {
        let sticker = CreateCommandOption::new(CommandOptionType::String, "sticker", "Sticker name, choose from among autocomplete")
            .name_localized("ja", "スタンプ")
            .description_localized("ja", "スタンプの名前。一覧から選んでください。")
            .set_autocomplete(true)
            .required(true);
        let sound = CreateCommandOption::new(CommandOptionType::String, "sound", "Sound name, choose from among autocomlete or input Soundmoji")
            .name_localized("ja", "サウンド")
            .description_localized("ja", "サウンドの名前。一覧から選んでください。または Soundmoji を入力してください。")
            .set_autocomplete(true)
            .required(true);
        CreateCommandOption::new(CommandOptionType::SubCommand, "link", "Links sound to sticker")
            .description_localized("ja", "スタンプにサウンドを紐づけます。")
            .add_sub_option(sticker)
            .add_sub_option(sound)
    };

    let list = CreateCommandOption::new(CommandOptionType::SubCommand, "list", "List soundstickers")
            .description_localized("ja", "サウンドが紐づいたスタンプの一覧を表示します。");

    let delete = {
        let sticker = CreateCommandOption::new(CommandOptionType::String, "sticker", "Sticker name, choose from among autocomplete")
            .name_localized("ja", "スタンプ")
            .description_localized("ja", "スタンプの名前。一覧から選んでください。")
            .set_autocomplete(true)
            .required(true);
        CreateCommandOption::new(CommandOptionType::SubCommand, "delete", "Links sound to sticker")
            .description_localized("ja", "スタンプとサウンド紐づけを解除します。")
            .add_sub_option(sticker)
    };

    CreateCommand::new("soundsticker")
        .description("スタンプを投稿したときに再生されるサウンドボードのサウンドを管理します。")
        .set_options(vec![link, list, delete])
}

pub(crate) async fn autocomplete(context: &Context, interaction: &CommandInteraction) -> Result<()> {
    let Some(guild_id) = interaction.guild_id else {
        return Ok(());
    };

    let subcommand = interaction.data.options.first().context("cannot get /soundsticker subcommand")?;

    let options = match &subcommand.value {
        CommandDataOptionValue::SubCommand(options) => options,
        _ => return Ok(()),
    };

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
        .filter(|&sticker| sticker.name.contains(value))
        .map(|sticker| AutocompleteChoice::new(sticker.name.clone(), sticker.id.to_string()))
        .take(25)
        .collect::<Vec<_>>();

    CreateInteractionResponse::Autocomplete(CreateAutocompleteResponse::new().set_choices(choices))
}

fn soundboard_autocomplete(soundboard: Soundboard, value: &str) -> CreateInteractionResponse {
    let choices = soundboard
        .items
        .iter()
        .filter(|&sound| sound.name.contains(value))
        .map(|sound| AutocompleteChoice::new(sound.name.clone(), sound.sound_id.get().to_string()))
        .take(25)
        .collect::<Vec<_>>();

    CreateInteractionResponse::Autocomplete(CreateAutocompleteResponse::new().set_choices(choices))
}
