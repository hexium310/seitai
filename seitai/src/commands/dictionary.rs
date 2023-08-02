use std::collections::HashMap;

use anyhow::{bail, Result};
use indexmap::IndexMap;
use regex_lite::Regex;
use serenity::{
    all::{CommandDataOptionValue, CommandOptionType},
    builder::{CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponseMessage},
    client::Context,
    model::{application::CommandInteraction, Colour},
};
use uuid::Uuid;
use voicevox::dictionary::{
    response::{DeleteUserDictWordResult, GetUserDictResult, PostUserDictWordResult, PutUserDictWordResult},
    Dictionary,
};

use crate::utils::{get_voicevox, normalize, respond};

pub(crate) async fn run(context: &Context, interaction: &CommandInteraction) -> Result<()> {
    let guild_id = interaction.guild_id.unwrap();
    let users = guild_id
        .members(&context.http, None, None)
        .await?
        .iter()
        .map(|member| member.user.clone())
        .collect::<Vec<_>>();
    let dictionary = {
        let voicevox = get_voicevox(context).await;
        let voicevox = voicevox.lock().await;
        voicevox.dictionary.clone()
    };

    for option in &interaction.data.options {
        let mut subcommand_options = to_option_map(&option.value).unwrap_or_default();
        subcommand_options.entry("surface".to_string()).and_modify(|word| {
            let text = normalize(context, &guild_id, &users, word);
            *word = Regex::new(r"<:([\w_]+):\d+>")
                .unwrap()
                .replace_all(&text, ":$1:")
                .to_string();
        });

        match option.name.as_str() {
            "add" => {
                subcommand_options
                    .entry("accent_type".to_string())
                    .or_insert("0".to_string());
                subcommand_options
                    .entry("pronunciation".to_string())
                    .and_modify(|pronunciation| {
                        *pronunciation = to_katakana(pronunciation);
                    });
                let word = subcommand_options.get("surface").unwrap();
                let uuid = match get_regsiterd(&dictionary, word).await {
                    Ok(uuid) => uuid,
                    Err(why) => {
                        let message = CreateInteractionResponseMessage::new().embed(
                            CreateEmbed::new()
                                .title("単語の登録に失敗しました。")
                                .field("詳細", format!("```\n{}\n```", why), false)
                                .colour(Colour::RED),
                        );
                        respond(context, interaction, message).await?;
                        continue;
                    },
                };

                if let Some(uuid) = uuid {
                    update_word(context, interaction, &dictionary, &uuid, &subcommand_options).await?;
                    continue;
                }

                register_word(context, interaction, &dictionary, &subcommand_options).await?;
            },
            // TODO: Paginate
            "list" => {
                let GetUserDictResult::Ok(list) = dictionary.list().await?;
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
                respond(context, interaction, message).await?;
            },
            "delete" => {
                let word = subcommand_options.get("surface").unwrap();
                let uuid = match get_regsiterd(&dictionary, word).await {
                    Ok(uuid) => uuid,
                    Err(why) => {
                        let message = CreateInteractionResponseMessage::new().embed(
                            CreateEmbed::new()
                                .title("単語の削除に失敗しました。")
                                .field("詳細", format!("```\n{}\n```", why), false)
                                .colour(Colour::RED),
                        );
                        respond(context, interaction, message).await?;
                        continue;
                    },
                };

                if let Some(uuid) = uuid {
                    delete_word(context, interaction, &dictionary, &uuid, &subcommand_options).await?;
                    continue;
                }

                let message = CreateInteractionResponseMessage::new().embed(
                    CreateEmbed::new()
                        .title("単語は登録されていません。")
                        .field("単語", format!("```\n{}\n```", word), false)
                        .colour(Colour::RED),
                );
                respond(context, interaction, message).await?;
            },
            _ => {
                unreachable!();
            },
        };
    }
    Ok(())
}

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
        let priority = CreateCommandOption::new(CommandOptionType::Integer, "priority", "The higher number, the higher priority of word (0 - 10. default: 8)")
            .name_localized("ja", "優先度")
            .description_localized("ja", "数字が大きいほど優先度が高くなる（0 〜 10。デフォルトは 8）")
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

fn to_option_map(value: &CommandDataOptionValue) -> Option<HashMap<String, String>> {
    if let CommandDataOptionValue::SubCommand(value) = value {
        let mut subcommand_options = HashMap::new();
        for subcommand_option in value {
            let name = subcommand_option.name.clone();
            match &subcommand_option.value {
                CommandDataOptionValue::String(value) => {
                    subcommand_options.insert(name, value.to_string());
                },
                CommandDataOptionValue::Integer(value) => {
                    subcommand_options.insert(name, value.to_string());
                },
                _ => {},
            }
        }

        Some(subcommand_options)
    } else {
        None
    }
}

async fn get_regsiterd(dictionary: &Dictionary, word: &str) -> Result<Option<Uuid>> {
    let GetUserDictResult::Ok(list) = dictionary.list().await?;
    let uuids = list
        .into_iter()
        .filter(|(_uuid, item)| item.surface == to_full_width(word))
        .collect::<IndexMap<_, _>>();
    if uuids.len() > 1 {
        bail!("`{word}` is registered in more than one.");
    }

    Ok(uuids.into_keys().next())
}

async fn register_word(
    context: &Context,
    interaction: &CommandInteraction,
    dictionary: &Dictionary,
    property: &HashMap<String, String>,
) -> Result<()> {
    let word = property.get("surface").unwrap();
    let pronunciation = property.get("pronunciation").unwrap();
    let parameters = property
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect::<Vec<_>>();

    match dictionary.register_word(&parameters).await? {
        PostUserDictWordResult::Ok(_id) => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("単語を登録しました。")
                    .field("単語", format!("```\n{}\n```", word), false)
                    .field("ヨミ", format!("```\n{}\n```", pronunciation), false)
                    .colour(Colour::FOOYOO),
            );
            respond(context, interaction, message).await?;
        },
        PostUserDictWordResult::UnprocessableEntity(error) => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("単語の登録に失敗しました。")
                    .field("詳細", format!("```\n{}\n```", error.detail), false)
                    .colour(Colour::RED),
            );
            respond(context, interaction, message).await?;
        },
    };

    Ok(())
}

async fn update_word(
    context: &Context,
    interaction: &CommandInteraction,
    dictionary: &Dictionary,
    uuid: &Uuid,
    property: &HashMap<String, String>,
) -> Result<()> {
    let word = property.get("surface").unwrap();
    let pronunciation = property.get("pronunciation").unwrap();
    let parameters = property
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect::<Vec<_>>();

    match dictionary.update_word(uuid, &parameters).await? {
        PutUserDictWordResult::NoContent => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("単語を更新しました。")
                    .field("単語", format!("```\n{}\n```", word), false)
                    .field("ヨミ", format!("```\n{}\n```", pronunciation), false)
                    .colour(Colour::FOOYOO),
            );
            respond(context, interaction, message).await?;
        },
        PutUserDictWordResult::UnprocessableEntity(error) => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("単語の更新に失敗しました。")
                    .field("詳細", format!("```\n{}\n```", error.detail), false)
                    .colour(Colour::RED),
            );
            respond(context, interaction, message).await?;
        },
    };

    Ok(())
}

async fn delete_word(
    context: &Context,
    interaction: &CommandInteraction,
    dictionary: &Dictionary,
    uuid: &Uuid,
    property: &HashMap<String, String>,
) -> Result<()> {
    let word = property.get("surface").unwrap();

    match dictionary.delete_word(uuid).await? {
        DeleteUserDictWordResult::NoContent => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("単語を削除しました。")
                    .field("単語", format!("```\n{}\n```", word), false)
                    .colour(Colour::FOOYOO),
            );
            respond(context, interaction, message).await?;
        },
        DeleteUserDictWordResult::UnprocessableEntity(error) => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("単語の削除に失敗しました。")
                    .field("詳細", format!("```\n{}\n```", error.detail), false)
                    .colour(Colour::RED),
            );
            respond(context, interaction, message).await?;
        },
    };

    Ok(())
}

fn to_full_width(text: &str) -> String {
    text.chars()
        .map(|char| match u32::from(char) {
            code @ 0x21..=0x7E => char::from_u32(code + 0xFEE0).unwrap_or(char),
            _ => char,
        })
        .collect()
}

fn to_half_width(text: &str) -> String {
    Regex::new(r"\u3000")
        .unwrap()
        .replace_all(text, " ")
        .chars()
        .map(|char| match u32::from(char) {
            code @ 0xFF01..=0xFF5E => char::from_u32(code - 0xFEE0).unwrap_or(char),
            _ => char,
        })
        .collect()
}

const HIRAGANA_BEGIN: u32 = 'ぁ' as u32;
const HIRAGANA_END: u32 = 'ゖ' as u32;
const HIRAGANA_KATAKANA_DIFF: u32 = 0x60;

fn to_katakana(text: &str) -> String {
    text.chars()
        .map(|char| match u32::from(char) {
            code @ HIRAGANA_BEGIN..=HIRAGANA_END => char::from_u32(code + HIRAGANA_KATAKANA_DIFF).unwrap_or(char),
            _ => char,
        })
        .collect()
}
