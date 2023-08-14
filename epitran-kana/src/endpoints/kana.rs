use anyhow::{bail, Result};
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Json};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::ipa;

#[derive(Deserialize)]
pub(crate) struct GetParams {
    word: String,
}

#[derive(Serialize)]
pub(crate) struct Katakana {
    word: String,
    pronunciation_kana: String,
    pronunciation: String,
}

pub(crate) async fn get(Path(GetParams { word }): Path<GetParams>) -> impl IntoResponse {
    let pronunciation = ipa::transliterate(&word).unwrap();
    let pronunciation_kana = self::transliterate(&pronunciation).unwrap();

    let response = Katakana {
        word,
        pronunciation,
        pronunciation_kana,
    };

    (StatusCode::OK, Json(response))
}

pub(crate) static CONSONANTS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "p",
    "b",
    "t",
    "d",
    "ts",
    "ds",
    "t͡ʃ",
    "d͡ʒ",
    "k",
    "ɡ",
    "f",
    "v",
    "s",
    "z",
    "θ",
    "ð",
    "ʃ",
    "ʒ",
    "h",
    "m",
    "n",
    "ŋ",
    "l",
    "ɹ",
    "w",
    "ʍ",
    "hw",
    "j",
]);

static VOWELS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "æ",
    "ɑ",
    "ɒ",
    "ɔ",
    "ə",
    "ɪ",
    "i",
    "eɪ",
    "ej",
    "ɛ",
    "ʌ",
    "ʊ",
    "u",
    "ju",
    "aɪ",
    "aj",
    "ɔɪ",
    "ɔj",
    "oɪ",
    "oj",
    "oʊ",
    "ow",
    "aʊ",
    "aw",
    "ɪə",
    "ɛə",
    "ɹ̩",
    "jʊ",
]);

fn transliterate(pronunciation: &str) -> Result<String> {
    let mut kana = String::new();
    let mut pronunciation = pronunciation;

    while !pronunciation.is_empty() {
        let (rest, consonant, vowel) = split_into_clusters(pronunciation, &CONSONANTS, &VOWELS);
        pronunciation = rest;

        let syllable = build_syllable(consonant, vowel).unwrap_or_else(|error| {
            tracing::error!("failed to build syllable\nError: {error:?}");
            String::default()
        });

        kana.push_str(&syllable);
    }

    Ok(kana)
}

fn get_cluster<'a>(base: &'a str, list: &'a [&str]) -> (&'a str, Option<&'a str>) {
    list.iter()
        .filter_map(|&cluster| base.strip_prefix(cluster).map(|others| (others, Some(cluster))))
        .max_by_key(|(_, cluster)| cluster.unwrap_or_default().len())
        .unwrap_or((base, None))
}

fn split_into_clusters<'a>(
    base: &'a str,
    consonants: &'a [&str],
    vowels: &'a [&str],
) -> (&'a str, Option<&'a str>, Option<&'a str>) {
    let (base, consonant) = get_cluster(base, consonants);
    let (base, vowel) = get_cluster(base, vowels);

    (base, consonant, vowel)
}

fn build_syllable(consonant: Option<&str>, vowel: Option<&str>) -> Result<String> {
    match (consonant, vowel) {
        (Some(consonant), Some(vowel)) => build_consonant_vowel(consonant, vowel),
        (Some(consonant), None) => build_consonant(consonant),
        (None, Some(vowel)) => build_vowel(vowel),
        (None, None) => {
            bail!("error");
        },
    }
}

const LONG_VOWEL: &str = "ー";
const DIPHTHONG_I: &str = "イ";
const DIPHTHONG_U: &str = "ウ";

fn build_consonant_vowel(consonant: &str, vowel: &str) -> Result<String> {
    let kana_patterns = match consonant {
        "p" => vec!["パ", "ピ", "ピュ", "ペ", "ポ"],
        "b" => vec!["バ", "ビ", "ビュ", "ベ", "ボ"],
        "t" => vec!["タ", "ティ", "トゥ", "テ", "ト"],
        "d" => vec!["ダ", "ディ", "ドゥ", "デ", "ド"],
        "ts" => vec!["タ", "ティ", "ツ", "テ", "ト"],
        "ds" => vec!["ダ", "ディ", "ズ", "デ", "ド"],
        "t͡ʃ" => vec!["チャ", "チ", "チュ", "チェ", "チョ"],
        "d͡ʒ" => vec!["ジャ", "ジ", "ジュ", "ジェ", "ジョ"],
        "k" => vec!["カ", "キ", "ク", "ケ", "コ"],
        "ɡ" => vec!["ガ", "ギ", "グ", "ゲ", "ゴ"],
        "f" => vec!["ファ", "フィ", "フュ", "フェ", "フォ"],
        "v" => vec!["ヴァ", "ヴィ", "ヴ", "ヴェ", "ヴォ"],
        "s" | "θ" => vec!["サ", "シ", "ス", "セ", "ソ"],
        "z" | "ð" => vec!["ザ", "ジ", "ズ", "ゼ", "ゾ"],
        "ʃ" => vec!["シャ", "シ", "シュ", "シェ", "ショ"],
        "ʒ" => vec!["ジャ", "ジ", "ジュ", "ジェ", "ジョ"],
        "h" => vec!["ハ", "ヒ", "ヒュ", "ヘ", "ホ"],
        "m" => vec!["マ", "ミ", "ム", "メ", "モ"],
        "n" => vec!["ナ", "ニ", "ヌ", "ネ", "ノ"],
        "ŋ" => vec!["ンガ", "ンギ", "ング", "ンゲ", "ンゴ"],
        "l" | "ɹ" => vec!["ラ", "リ", "ル", "レ", "ロ"],
        "w" | "ʍ" | "hw" => vec!["ワ", "ウィ", "ウ", "ウェ", "ウォ"],
        "j" => vec!["ヤ", "イ", "ユ", "イェ", "ヨ"],
        _ => bail!("unexpected consonant: {consonant}"),
    };

    let kanas = match vowel {
        "æ" => vec![kana_patterns.first(), None],
        "ɑ" => vec![kana_patterns.first(), None],
        "ɒ" => vec![kana_patterns.last(), None],
        "ɔ" => vec![kana_patterns.last(), Some(&LONG_VOWEL)],
        "ə" => vec![kana_patterns.first(), None],
        "ɪ" => vec![kana_patterns.get(1), None],
        "i" => vec![kana_patterns.get(1), Some(&LONG_VOWEL)],
        "eɪ" | "ej" => vec![kana_patterns.get(3), Some(&DIPHTHONG_I)],
        "ɛ" => vec![kana_patterns.get(3), None],
        "ʌ" => vec![kana_patterns.first(), None],
        "ʊ" => vec![kana_patterns.get(2), None],
        "u" => vec![kana_patterns.get(2), Some(&LONG_VOWEL)],
        "ju" => vec![kana_patterns.get(2), Some(&LONG_VOWEL)],
        "aɪ" | "aj" => vec![kana_patterns.first(), Some(&DIPHTHONG_I)],
        "ɔɪ" | "ɔj" | "oɪ" | "oj" => vec![kana_patterns.last(), Some(&DIPHTHONG_I)],
        "oʊ" | "ow" => vec![kana_patterns.last(), Some(&DIPHTHONG_U)],
        "aʊ" | "aw" => vec![kana_patterns.first(), Some(&DIPHTHONG_U)],
        "ɹ̩" => vec![kana_patterns.first(), Some(&LONG_VOWEL)],
        "jʊ" => vec![kana_patterns.get(2), None],
        _ => bail!("unexpected vowel: {vowel}"),
    };

    Ok(kanas.into_iter().flatten().map(ToOwned::to_owned).collect::<String>())
}

fn build_consonant(consonant: &str) -> Result<String> {
    let kana = match consonant {
        "p" => "プ",
        "b" => "ブ",
        "t" => "ト",
        "d" => "ド",
        "ts" => "ツ",
        "ds" => "ズ",
        "t͡ʃ" => "チ",
        "d͡ʒ" => "ジ",
        "k" => "ク",
        "ɡ" => "グ",
        "f" => "フ",
        "v" => "ヴ",
        "s" | "θ" => "ス",
        "z" | "ð" => "ズ",
        "ʃ" => "シュ",
        "ʒ" => "ジュ",
        "h" => "フ",
        "m" | "n" => "ン",
        "ŋ" => "ング",
        "l" => "ル",
        "ɹ" => "アー",
        "w" | "ʍ" | "hw" => "ウ",
        "j" => "ユ",
        _ => bail!("unexpected consonant: {consonant}"),
    };

    Ok(kana.to_string())
}

fn build_vowel(vowel: &str) -> Result<String> {
    let kana = match vowel {
        "æ" => "ア",
        "ɑ" => "ア",
        "ɒ" => "オ",
        "ɔ" => "オー",
        "ə" => "ア",
        "ɪ" => "イ",
        "i" => "イー",
        "eɪ" | "ej" => "エイ",
        "ɛ" => "エ",
        "ʌ" => "ア",
        "ʊ" => "ウ",
        "u" => "ウー",
        "ju" => "ユー",
        "aɪ" | "aj" => "アイ",
        "ɔɪ" | "ɔj" | "oɪ" | "oj" => "オイ",
        "oʊ" | "ow" => "オウ",
        "aʊ" | "aw" => "アウ",
        "ɹ̩" => "アー",
        "jʊ" => "ユ",
        _ => bail!("unexpected vowel: {vowel}"),
    };

    Ok(kana.to_string())
}
