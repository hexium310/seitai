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

use crate::{utils::respond, voicevox};

pub(crate) async fn run(context: &Context, interaction: &CommandInteraction) -> Result<()> {
    for option in &interaction.data.options {
        match option.name.as_str() {
            "add" => {
                let mut subcommand_options = to_option_map(&option.value).unwrap_or_default();
                subcommand_options
                    .entry("accent_type".to_string())
                    .or_insert("0".to_string());
                let word = subcommand_options.get("surface").unwrap();
                let uuid = match get_regsiterd(word).await {
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
                    }
                };

                if let Some(uuid) = uuid {
                    update_word(context, interaction, &uuid, &subcommand_options).await?;
                    continue;
                }

                register_word(context, interaction, &subcommand_options).await?;
            },
            // TODO: Paginate
            "list" => {
                let dictionary = voicevox::get_dictionary().await?;
                let words = dictionary.values().map(|item| format!("{} -> {}", to_half_width(&item.surface), item.pronunciation)).collect::<Vec<_>>();
                let message = CreateInteractionResponseMessage::new().embed(
                    CreateEmbed::new()
                        .title("単語一覧")
                        .description(format!("```\n{}\n```", words.join("\n")))
                        .colour(Colour::FOOYOO),
                );
                respond(context, interaction, message).await?;
            }
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

    CreateCommand::new("dictionary")
        .description("Dictionary")
        .set_options(vec![add, list])
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

async fn get_regsiterd(word: &str) -> Result<Option<Uuid>> {
    let dictionary = voicevox::get_dictionary().await?;
    let uuids = dictionary
        .into_iter()
        .filter(|(_uuid, item)| item.surface == to_full_width(word))
        .collect::<IndexMap<_, _>>();
    if uuids.len() > 1 {
        bail!("`{word}` is registered in more than one.");
    }

    Ok(uuids.into_keys().next())
}

async fn register_word(context: &Context, interaction: &CommandInteraction, property: &HashMap<String, String>) -> Result<()> {
    let word = property.get("surface").unwrap();
    let pronunciation = property.get("pronunciation").unwrap();

    match voicevox::register_word(property.iter()).await? {
        voicevox::PostUserDictWordResponse::Ok(_id) => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("単語を登録しました。")
                    .field("単語", format!("```\n{}\n```", word), false)
                    .field("ヨミ", format!("```\n{}\n```", pronunciation), false)
                    .colour(Colour::FOOYOO),
            );
            respond(context, interaction, message).await?;
        },
        voicevox::PostUserDictWordResponse::UnprocessableEntity(error) => {
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

async fn update_word(context: &Context, interaction: &CommandInteraction, uuid: &Uuid, property: &HashMap<String, String>) -> Result<()> {
    let word = property.get("surface").unwrap();
    let pronunciation = property.get("pronunciation").unwrap();

    match voicevox::update_word(uuid, property.iter()).await? {
        voicevox::PutUserDictWordResponse::NoContent => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .title("単語を更新しました。")
                    .field("単語", format!("```\n{}\n```", word), false)
                    .field("ヨミ", format!("```\n{}\n```", pronunciation), false)
                    .colour(Colour::FOOYOO),
            );
            respond(context, interaction, message).await?;
        },
        voicevox::PutUserDictWordResponse::UnprocessableEntity(error) => {
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

fn to_full_width(text: &str) -> String {
    Regex::new(r"\s")
        .unwrap()
        .replace_all(text, "\u{3000}")
        .chars()
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
