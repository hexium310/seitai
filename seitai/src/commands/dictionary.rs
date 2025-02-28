use anyhow::{bail, Context as _, Result};
use futures::{future, stream, StreamExt};
use hashbrown::HashMap;
use indexmap::IndexMap;
use ordered_float::NotNan;
use serenity::{
    all::{CommandDataOptionValue, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponseMessage},
    client::Context,
    model::{application::CommandInteraction, Colour},
};
use songbird::input::Input;
use uuid::Uuid;
use voicevox::dictionary::{
    response::{DeleteUserDictWordResult, GetUserDictResult, PostUserDictWordResult, PutUserDictWordResult},
    Dictionary,
};

use crate::{
    audio::{cache::PredefinedUtterance, Audio, AudioRepository},
    character_converter::{to_full_width, to_half_width, to_katakana},
    regex,
    speaker::Speaker,
    utils::{get_manager, get_voicevox, normalize, respond},
};

use super::subcommand::Subcommand;

const SYSTEM_SPEAKER: &str = "1";

pub(crate) async fn run<Repository>(
    context: &Context,
    audio_repository: &Repository,
    interaction: &CommandInteraction,
) -> Result<()>
where
    Repository: AudioRepository<Input = Input> + Send + Sync,
{
    let Some(guild_id) = interaction.guild_id else {
        return Ok(());
    };
    let users = guild_id
        .members(&context.http, None, None)
        .await
        .with_context(|| format!("failed to get list of guild ({guild_id}) members"))?
        .iter()
        .map(|member| member.user.clone())
        .collect::<Vec<_>>();
    let dictionary = {
        let voicevox = get_voicevox(context)
            .await
            .context("failed to get voicevox client for /dictionary command")?;
        let voicevox = voicevox.lock().await;
        voicevox.dictionary.clone()
    };

    for option in &interaction.data.options {
        let subcommand = Subcommand::from_command_data_option(option).unwrap_or_default();
        let mut subcommand_options = subcommand
            .options
            .into_iter()
            .map(|(k, v)| {
                match v {
                    CommandDataOptionValue::String(v) => (k, v.to_string()),
                    CommandDataOptionValue::Integer(v) => (k, v.to_string()),
                    _ => unreachable!(),
                }
            })
            .collect::<HashMap<_, _>>();
        subcommand_options
            .entry("surface")
            .and_replace_entry_with(|_key, word| {
                let text = normalize(context, &guild_id, &users, &word);
                Some(regex::EMOJI.replace_all(&text, ":$1:").into_owned())
            });

        match option.name.as_str() {
            "add" => {
                subcommand_options.entry("accent_type").or_insert("0".to_string());
                subcommand_options.entry("priority").or_insert("10".to_string());
                subcommand_options
                    .entry("pronunciation")
                    .and_replace_entry_with(|_key, pronunciation| Some(to_katakana(&*pronunciation).into_owned()));

                let word = subcommand_options
                    .get("surface")
                    .context("there is no surface to make sure whether word is regsitered")?;
                let uuid = match get_regsiterd(&dictionary, word).await {
                    Ok(uuid) => uuid,
                    Err(error) => {
                        tracing::error!("failed to register {word} into dictionary\nError: {error:?}");
                        let message = CreateInteractionResponseMessage::new().embed(
                            CreateEmbed::new()
                                .title("単語の登録に失敗しました。")
                                .field("詳細", format!("```\n{}\n```", error), false)
                                .colour(Colour::RED),
                        );
                        respond(context, interaction, &message).await?;
                        continue;
                    },
                };

                if let Some(uuid) = uuid {
                    update_word(context, interaction, &dictionary, &uuid, &subcommand_options).await?;
                } else {
                    register_word(context, interaction, &dictionary, &subcommand_options).await?;
                }

                let manager = get_manager(context).await?;
                let call = manager.get_or_insert(guild_id);
                let mut call = call.lock().await;

                if call.current_connection().is_none() {
                    continue;
                };

                let inputs = stream::iter([word, PredefinedUtterance::Registered.as_ref()])
                    .map(async |text| {
                        let audio = Audio {
                            text: text.to_string(),
                            speaker: SYSTEM_SPEAKER.to_string(),
                            speed: NotNan::new(Speaker::default_speed()).unwrap(),
                        };
                        match audio_repository.get(audio).await {
                            Ok(input) => Some(input),
                            Err(error) => {
                                tracing::error!("failed to get audio source\nError: {error:?}");
                                None
                            },
                        }
                    })
                    .collect::<Vec<_>>()
                    .await;

                for input in future::join_all(inputs).await.into_iter().flatten() {
                    call.enqueue_input(input).await;
                }
            },
            // TODO: Paginate
            "list" => {
                let response = match dictionary.list().await {
                    Ok(response) => response,
                    Err(error) => {
                        let message = CreateInteractionResponseMessage::new().embed(
                            CreateEmbed::new()
                                .title("単語の更新に失敗しました。")
                                .field("詳細", format!("```\n{}\n```", error), false)
                                .colour(Colour::RED),
                        );
                        respond(context, interaction, &message).await?;
                        bail!("failed to get dictionary for /dictionary list command\nError: {error:?}");
                    },
                };

                let GetUserDictResult::Ok(list) = response;
                let words = list
                    .values()
                    .map(|item| format!("{} -> {}", to_half_width(&item.surface), item.pronunciation))
                    .collect::<Vec<_>>();
                let message = CreateInteractionResponseMessage::new().embed(
                    CreateEmbed::new()
                        .title("単語一覧")
                        .description(format!("```\n{}\n```", words.join("\n")))
                        .colour(Colour::FOOYOO),
                );
                respond(context, interaction, &message).await?;
            },
            "delete" => {
                let word = subcommand_options
                    .get("surface")
                    .context("there is no surface to delete word")?;
                let uuid = match get_regsiterd(&dictionary, word).await {
                    Ok(uuid) => uuid,
                    Err(error) => {
                        tracing::error!("failed to delete {word} in dictionary\nError: {error:?}");
                        let message = CreateInteractionResponseMessage::new().embed(
                            CreateEmbed::new()
                                .title("単語の削除に失敗しました。")
                                .field("詳細", format!("```\n{}\n```", error), false)
                                .colour(Colour::RED),
                        );
                        respond(context, interaction, &message).await?;
                        continue;
                    },
                };

                if let Some(uuid) = uuid {
                    delete_word(context, interaction, &dictionary, &uuid, word).await?;
                    continue;
                }

                let message = CreateInteractionResponseMessage::new().embed(
                    CreateEmbed::new()
                        .title("単語は登録されていません。")
                        .field("単語", format!("```\n{}\n```", word), false)
                        .colour(Colour::RED),
                );
                respond(context, interaction, &message).await?;
            },
            _ => {
                unreachable!();
            },
        };
    }
    Ok(())
}

#[rustfmt::skip]
pub fn register() -> CreateCommand {
    let add = {
        let word = CreateCommandOption::new(CommandOptionType::String, "surface", "Word to be registered")
            .name_localized("ja", "単語")
            .description_localized("ja", "登録する単語")
            .required(true);
        let pronunciation = CreateCommandOption::new(CommandOptionType::String, "pronunciation", "Pronunciation")
            .name_localized("ja", "ヨミ")
            .description_localized("ja", "単語のヨミ（カタカナ）")
            .required(true);
        let accent_type = CreateCommandOption::new(CommandOptionType::Integer, "accent_type", "Position of accent core. See link on `/help directory`")
            .name_localized("ja", "音が下がる位置")
            .description_localized("ja", "音が下がる拍の位置。0 は途中で音が下がらず、n（n > 0）は n 拍目の直後に音が下がる。詳細は `/help dictionary` のリンクへ")
            .min_int_value(0);
        let word_type = CreateCommandOption::new(CommandOptionType::String, "word_type", "Category of word")
            .name_localized("ja", "単語の種類")
            .description_localized("ja", "単語の種類")
            .add_string_choice_localized("PROPER_NOUN", "PROPER_NOUN", [("ja", "固有名詞")])
            .add_string_choice_localized("COMMON_NOUN", "COMMON_NOUN", [("ja", "一般名詞")])
            .add_string_choice_localized("VERB", "VERB", [("ja", "動詞")])
            .add_string_choice_localized("ADJECTIVE", "ADJECTIVE", [("ja", "形容詞")])
            .add_string_choice_localized("SUFFIX", "SUFFIX", [("ja", "接尾辞")]);
        let priority = CreateCommandOption::new(CommandOptionType::Integer, "priority", "The higher number, the higher priority of word (0 - 10. default: 10)")
            .name_localized("ja", "優先度")
            .description_localized("ja", "数字が大きいほど優先度が高くなる（0 〜 10。デフォルトは 10）")
            .min_int_value(0)
            .max_int_value(10);
        CreateCommandOption::new(CommandOptionType::SubCommand, "add", "Registers word into dictionary")
            .description_localized("ja", "単語を辞書に登録します")
            .add_sub_option(word)
            .add_sub_option(pronunciation)
            .add_sub_option(accent_type)
            .add_sub_option(word_type)
            .add_sub_option(priority)
    };
    let list = CreateCommandOption::new(CommandOptionType::SubCommand, "list", "List registered words")
        .description_localized("ja", "登録されている単語を表示します。");
    let delete = {
        let word = CreateCommandOption::new(CommandOptionType::String, "surface", "Word to be registered")
            .name_localized("ja", "単語")
            .description_localized("ja", "削除する単語")
            .required(true);
        CreateCommandOption::new(CommandOptionType::SubCommand, "delete", "Delete word in dictionary")
            .description_localized("ja", "単語を辞書から削除します")
            .add_sub_option(word)
    };

    CreateCommand::new("dictionary")
        .description("Dictionary")
        .set_options(vec![add, list, delete])
}

async fn get_regsiterd(dictionary: &Dictionary, word: &str) -> Result<Option<Uuid>> {
    let GetUserDictResult::Ok(list) = dictionary
        .list()
        .await
        .context("failed to get dictionary to register word")?;
    let uuids = list
        .into_iter()
        .filter(|(_uuid, item)| item.surface == to_full_width(word))
        .collect::<IndexMap<_, _>>();
    if uuids.len() > 1 {
        bail!("{word} is registered in more than one");
    }

    Ok(uuids.into_keys().next())
}

async fn register_word(
    context: &Context,
    interaction: &CommandInteraction,
    dictionary: &Dictionary,
    property: &HashMap<&str, String>,
) -> Result<()> {
    let word = property
        .get("surface")
        .context("there is no surface to register word")?;
    let pronunciation = property
        .get("pronunciation")
        .context("there is no pronunciation to register word")?;
    let parameters = property
        .iter()
        .map(|(&key, value)| (key, value.as_str()))
        .collect::<Vec<_>>();

    let response = match dictionary.register_word(&parameters).await {
        Ok(response) => response,
        Err(error) => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("単語の登録に失敗しました。")
                    .field("詳細", format!("```\n{}\n```", error), false)
                    .colour(Colour::RED),
            );
            respond(context, interaction, &message).await?;
            bail!("failed to register {word} into dictionary\nError: {error:?}");
        },
    };

    match response {
        PostUserDictWordResult::Ok(_id) => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("単語を登録しました。")
                    .field("単語", format!("```\n{}\n```", word), false)
                    .field("ヨミ", format!("```\n{}\n```", pronunciation), false)
                    .colour(Colour::FOOYOO),
            );
            respond(context, interaction, &message).await?;
        },
        PostUserDictWordResult::UnprocessableEntity(error) => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("単語の登録に失敗しました。")
                    .field("詳細", format!("```\n{}\n```", error.detail), false)
                    .colour(Colour::RED),
            );
            respond(context, interaction, &message).await?;
            bail!("failed to register {word} into dictionary\nError: {error:?}");
        },
    };

    Ok(())
}

async fn update_word(
    context: &Context,
    interaction: &CommandInteraction,
    dictionary: &Dictionary,
    uuid: &Uuid,
    property: &HashMap<&str, String>,
) -> Result<()> {
    let word = property.get("surface").context("there is no surface to update word")?;
    let pronunciation = property
        .get("pronunciation")
        .context("there is no pronunciation to update word")?;
    let parameters = property
        .iter()
        .map(|(&key, value)| (key, value.as_str()))
        .collect::<Vec<_>>();

    let response = match dictionary.update_word(uuid, &parameters).await {
        Ok(response) => response,
        Err(error) => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("単語の更新に失敗しました。")
                    .field("詳細", format!("```\n{}\n```", error), false)
                    .colour(Colour::RED),
            );
            respond(context, interaction, &message).await?;
            bail!("failed to update {word} in dictionary\nError: {error:?}");
        },
    };

    match response {
        PutUserDictWordResult::NoContent => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("単語を更新しました。")
                    .field("単語", format!("```\n{}\n```", word), false)
                    .field("ヨミ", format!("```\n{}\n```", pronunciation), false)
                    .colour(Colour::FOOYOO),
            );
            respond(context, interaction, &message).await?;
        },
        PutUserDictWordResult::UnprocessableEntity(error) => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("単語の更新に失敗しました。")
                    .field("詳細", format!("```\n{}\n```", error.detail), false)
                    .colour(Colour::RED),
            );
            respond(context, interaction, &message).await?;
            bail!("failed to update {word} in dictionary\nError: {error:?}");
        },
    };

    Ok(())
}

async fn delete_word(
    context: &Context,
    interaction: &CommandInteraction,
    dictionary: &Dictionary,
    uuid: &Uuid,
    word: &str,
) -> Result<()> {
    let response = match dictionary.delete_word(uuid).await {
        Ok(response) => response,
        Err(error) => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("単語の削除に失敗しました。")
                    .field("詳細", format!("```\n{}\n```", error), false)
                    .colour(Colour::RED),
            );
            respond(context, interaction, &message).await?;
            bail!("failed to delete {word} in dictionary\nError: {error:?}");
        },
    };

    match response {
        DeleteUserDictWordResult::NoContent => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("単語を削除しました。")
                    .field("単語", format!("```\n{}\n```", word), false)
                    .colour(Colour::FOOYOO),
            );
            respond(context, interaction, &message).await?;
        },
        DeleteUserDictWordResult::UnprocessableEntity(error) => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("単語の削除に失敗しました。")
                    .field("詳細", format!("```\n{}\n```", error.detail), false)
                    .colour(Colour::RED),
            );
            respond(context, interaction, &message).await?;
            bail!("failed to delete {word} in dictionary\nError: {error:?}");
        },
    };

    Ok(())
}
