use anyhow::{Context as _, Result};
use sea_query::{Iden, OnConflict, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
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
use sqlx::{PgPool, Row};
use voicevox::speaker::response::Speaker;

use crate::utils::respond;

#[derive(Iden)]
enum Users {
    Table,
    Id,
    SpeakerId,
}

pub(crate) async fn run(
    context: &Context,
    interaction: &CommandInteraction,
    database: &PgPool,
    speakers: &[Speaker],
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

            let (sql, values) = Query::insert()
                .into_table(Users::Table)
                .columns([Users::Id, Users::SpeakerId])
                .values_panic([interaction.user.id.get().into(), speaker_id.into()])
                .on_conflict(OnConflict::column(Users::Id).update_column(Users::SpeakerId).to_owned())
                .returning_col(Users::SpeakerId)
                .build_sqlx(PostgresQueryBuilder);
            let mut connection = database.acquire().await?;
            let row = sqlx::query_with(&sql, values)
                .fetch_one(&mut *connection)
                .await
                .with_context(|| format!("failed to execute `{sql}`"))?;
            let speaker_id = u16::try_from(
                row.try_get::<i32, _>("speaker_id")
                    .context("failed to get speaker_id as i32 in users")?,
            )
            .context("failed to convert speaker_id to u16")?;
            let speaker_tuples = to_speaker_tuples(speakers);
            let speaker_name = &speaker_tuples.iter().find(|(_, id)| id == &speaker_id).context("")?.0;

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
    CreateCommand::new("voice")
        .description("ボイスの設定を行います。")
        .set_options(vec![r#use])
}

pub async fn autocomplete(context: &Context, interaction: &CommandInteraction, speakers: &[Speaker]) -> Result<()> {
    let subcommand = interaction.data.options.first().context("cannot get subcommand")?;
    let speaker = get_subcommand_option(&subcommand.value, "speaker").context("cannot get speaker from argument")?;

    if let CommandDataOptionValue::Autocomplete { value, .. } = &speaker {
        let choices = to_speaker_tuples(speakers)
            .into_iter()
            .filter_map(|(name, id)| name.contains(value).then_some(Some(AutocompleteChoice::new(name, id))))
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

fn to_speaker_tuples(speakers: &[Speaker]) -> Vec<(String, u16)> {
    speakers
        .iter()
        .flat_map(|speaker| {
            speaker.styles.iter().map(|style| {
                let name = format!("{}（{}）", speaker.name, style.name);
                (name, style.id)
            })
        })
        .collect::<Vec<_>>()
}
