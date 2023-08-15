use anyhow::{bail, Result, Context};
use hashbrown::HashMap;
use once_cell::sync::Lazy;

const LONG_VOWEL: &str = "ー";
const DIPHTHONG_I: &str = "イ";
const DIPHTHONG_U: &str = "ウ";

pub(crate) static CONSONANTS: Lazy<Consonants> = Lazy::new(|| {
    [
        (
            "p",
            KanaConsonantPattern {
                with_vowel: ["パ", "ピ", "ピュ", "ペ", "ポ"],
                unit: "プ",
            },
        ),
        (
            "b",
            KanaConsonantPattern {
                with_vowel: ["バ", "ビ", "ビュ", "ベ", "ボ"],
                unit: "ブ",
            },
        ),
        (
            "t",
            KanaConsonantPattern {
                with_vowel: ["タ", "ティ", "トゥ", "テ", "ト"],
                unit: "ト",
            },
        ),
        (
            "d",
            KanaConsonantPattern {
                with_vowel: ["ダ", "ディ", "ドゥ", "デ", "ド"],
                unit: "ド",
            },
        ),
        (
            "ts",
            KanaConsonantPattern {
                with_vowel: ["タ", "ティ", "ツ", "テ", "ト"],
                unit: "ツ",
            },
        ),
        (
            "ds",
            KanaConsonantPattern {
                with_vowel: ["ダ", "ディ", "ズ", "デ", "ド"],
                unit: "ズ",
            },
        ),
        (
            "t͡ʃ",
            KanaConsonantPattern {
                with_vowel: ["チャ", "チ", "チュ", "チェ", "チョ"],
                unit: "チ",
            },
        ),
        (
            "d͡ʒ",
            KanaConsonantPattern {
                with_vowel: ["ジャ", "ジ", "ジュ", "ジェ", "ジョ"],
                unit: "ジ",
            },
        ),
        (
            "k",
            KanaConsonantPattern {
                with_vowel: ["カ", "キ", "ク", "ケ", "コ"],
                unit: "ク",
            },
        ),
        (
            "ɡ",
            KanaConsonantPattern {
                with_vowel: ["ガ", "ギ", "グ", "ゲ", "ゴ"],
                unit: "グ",
            },
        ),
        (
            "f",
            KanaConsonantPattern {
                with_vowel: ["ファ", "フィ", "フュ", "フェ", "フォ"],
                unit: "フ",
            },
        ),
        (
            "v",
            KanaConsonantPattern {
                with_vowel: ["ヴァ", "ヴィ", "ヴ", "ヴェ", "ヴォ"],
                unit: "ヴ",
            },
        ),
        (
            "s",
            KanaConsonantPattern {
                with_vowel: ["サ", "シ", "ス", "セ", "ソ"],
                unit: "ス",
            },
        ),
        (
            "z",
            KanaConsonantPattern {
                with_vowel: ["ザ", "ジ", "ズ", "ゼ", "ゾ"],
                unit: "ズ",
            },
        ),
        (
            "θ",
            KanaConsonantPattern {
                with_vowel: ["サ", "シ", "ス", "セ", "ソ"],
                unit: "ス",
            },
        ),
        (
            "ð",
            KanaConsonantPattern {
                with_vowel: ["ザ", "ジ", "ズ", "ゼ", "ゾ"],
                unit: "ズ",
            },
        ),
        (
            "ʃ",
            KanaConsonantPattern {
                with_vowel: ["シャ", "シ", "シュ", "シェ", "ショ"],
                unit: "シュ",
            },
        ),
        (
            "ʒ",
            KanaConsonantPattern {
                with_vowel: ["ジャ", "ジ", "ジュ", "ジェ", "ジョ"],
                unit: "ジュ",
            },
        ),
        (
            "h",
            KanaConsonantPattern {
                with_vowel: ["ハ", "ヒ", "ヒュ", "ヘ", "ホ"],
                unit: "フ",
            },
        ),
        (
            "m",
            KanaConsonantPattern {
                with_vowel: ["マ", "ミ", "ム", "メ", "モ"],
                unit: "ン",
            },
        ),
        (
            "n",
            KanaConsonantPattern {
                with_vowel: ["ナ", "ニ", "ヌ", "ネ", "ノ"],
                unit: "ン",
            },
        ),
        (
            "ŋ",
            KanaConsonantPattern {
                with_vowel: ["ンガ", "ンギ", "ング", "ンゲ", "ンゴ"],
                unit: "ング",
            },
        ),
        (
            "l",
            KanaConsonantPattern {
                with_vowel: ["ラ", "リ", "ル", "レ", "ロ"],
                unit: "ル",
            },
        ),
        (
            "ɹ",
            KanaConsonantPattern {
                with_vowel: ["ラ", "リ", "ル", "レ", "ロ"],
                unit: "アー",
            },
        ),
        (
            "w",
            KanaConsonantPattern {
                with_vowel: ["ワ", "ウィ", "ウ", "ウェ", "ウォ"],
                unit: "ウ",
            },
        ),
        (
            "ʍ",
            KanaConsonantPattern {
                with_vowel: ["ワ", "ウィ", "ウ", "ウェ", "ウォ"],
                unit: "ウ",
            },
        ),
        (
            "hw",
            KanaConsonantPattern {
                with_vowel: ["ワ", "ウィ", "ウ", "ウェ", "ウォ"],
                unit: "ウ",
            },
        ),
        (
            "j",
            KanaConsonantPattern {
                with_vowel: ["ヤ", "イ", "ユ", "イェ", "ヨ"],
                unit: "ユ",
            },
        ),
    ]
    .into()
});

static VOWELS: Lazy<Vowels> = Lazy::new(|| {
    [
        ("æ", KanaVowelPattern { unit: "ア" }),
        ("ɑ", KanaVowelPattern { unit: "ア" }),
        ("ɒ", KanaVowelPattern { unit: "オ" }),
        ("ɔ", KanaVowelPattern { unit: "オー" }),
        ("ə", KanaVowelPattern { unit: "ア" }),
        ("ɪ", KanaVowelPattern { unit: "イ" }),
        ("i", KanaVowelPattern { unit: "イー" }),
        ("eɪ", KanaVowelPattern { unit: "エイ" }),
        ("ej", KanaVowelPattern { unit: "エイ" }),
        ("ɛ", KanaVowelPattern { unit: "エ" }),
        ("ʌ", KanaVowelPattern { unit: "ア" }),
        ("ʊ", KanaVowelPattern { unit: "ウ" }),
        ("u", KanaVowelPattern { unit: "ウー" }),
        ("ju", KanaVowelPattern { unit: "ユー" }),
        ("aɪ", KanaVowelPattern { unit: "アイ" }),
        ("aj", KanaVowelPattern { unit: "アイ" }),
        ("ɔɪ", KanaVowelPattern { unit: "オイ" }),
        ("ɔj", KanaVowelPattern { unit: "オイ" }),
        ("oɪ", KanaVowelPattern { unit: "オイ" }),
        ("oj", KanaVowelPattern { unit: "オイ" }),
        ("oʊ", KanaVowelPattern { unit: "オウ" }),
        ("ow", KanaVowelPattern { unit: "オウ" }),
        ("aʊ", KanaVowelPattern { unit: "アウ" }),
        ("aw", KanaVowelPattern { unit: "アウ" }),
        ("ɹ̩", KanaVowelPattern { unit: "アー" }),
        ("jʊ", KanaVowelPattern { unit: "ユ" }),
    ]
    .into()
});

type Consonants = HashMap<&'static str, KanaConsonantPattern>;
type Vowels = HashMap<&'static str, KanaVowelPattern>;

pub(crate) struct KanaConsonantPattern {
    pub(crate) with_vowel: [&'static str; 5],
    pub(crate) unit: &'static str,
}

pub(crate) struct KanaVowelPattern {
    pub(crate) unit: &'static str,
}

pub(crate) fn transliterate(pronunciation: &str) -> Result<String> {
    let mut kana = String::new();
    let mut pronunciation = pronunciation;

    while !pronunciation.is_empty() {
        let (rest, consonant, vowel) = split_into_clusters(pronunciation, CONSONANTS.keys(), VOWELS.keys());
        pronunciation = rest;

        let part = build_kana(consonant, vowel).with_context(|| format!("failed to build kana: consonant {consonant:?}, vowel {vowel:?}"))?;

        kana.push_str(&part);
    }

    Ok(kana)
}

fn get_cluster<'a, I, S>(base: &'a str, list: I) -> (&'a str, Option<&'a str>)
where
    I: IntoIterator<Item = &'a S>,
    S: AsRef<str> + 'a,
{
    list.into_iter()
        .filter_map(|cluster| {
            let cluster = cluster.as_ref();
            base.strip_prefix(cluster).map(|others| (others, Some(cluster)))
        })
        .max_by_key(|(_, cluster)| cluster.unwrap_or_default().len())
        .unwrap_or((base, None))
}

fn split_into_clusters<'a, I, J, S1, S2>(
    base: &'a str,
    consonants: I,
    vowels: J,
) -> (&'a str, Option<&'a str>, Option<&'a str>)
where
    I: IntoIterator<Item = &'a S1>,
    S1: AsRef<str> + 'a,
    J: IntoIterator<Item = &'a S2>,
    S2: AsRef<str> + 'a,
{
    let (base, consonant) = get_cluster(base, consonants);
    let (base, vowel) = get_cluster(base, vowels);

    (base, consonant, vowel)
}

fn build_kana(consonant: Option<&str>, vowel: Option<&str>) -> Result<String> {
    match (consonant, vowel) {
        (Some(consonant), Some(vowel)) => build_consonant_vowel(consonant, vowel),
        (Some(consonant), None) => build_consonant(consonant),
        (None, Some(vowel)) => build_vowel(vowel),
        (None, None) => {
            bail!("error");
        },
    }
}

fn build_consonant_vowel(consonant: &str, vowel: &str) -> Result<String> {
    let Some(KanaConsonantPattern { with_vowel: kana_patterns, .. }) = CONSONANTS.get(consonant) else {
        bail!("unexpected consonant: {consonant}");
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
    let Some(KanaConsonantPattern { unit: kana, .. }) = CONSONANTS.get(consonant) else {
        bail!("unexpected consonant: {consonant}");
    };

    Ok(kana.to_string())
}

fn build_vowel(vowel: &str) -> Result<String> {
    let Some(KanaVowelPattern { unit: kana }) = VOWELS.get(vowel) else {
        bail!("unexpected vowel: {vowel}");
    };

    Ok(kana.to_string())
}
