use std::borrow::Cow;

use regex_lite::Captures;

use crate::regex;

const HALF_GRAPHICAL_BEGIN: u32 = '!' as u32;
const HALF_GRAPHICAL_END: u32 = '~' as u32;
const FULL_GRAPHICAL_BEGIN: u32 = '！' as u32;
const FULL_GRAPHICAL_END: u32 = '～' as u32;
const HALF_FULL_GRAPHICAL_DIFF: u32 = 0xFEE0;
const HIRAGANA_BEGIN: u32 = 'ぁ' as u32;
const HIRAGANA_END: u32 = 'ゖ' as u32;
const HIRAGANA_KATAKANA_DIFF: u32 = 0x60;

pub(crate) fn to_full_width<'a>(text: impl Into<Cow<'a, str>>) -> Cow<'a, str> {
    let text = text.into();
    match regex::HALF_GRAPHICAL.replace_all(&text, |captures: &Captures| {
        captures[0]
            .chars()
            .map(|char| match u32::from(char) {
                code @ HALF_GRAPHICAL_BEGIN..=HALF_GRAPHICAL_END => {
                    char::from_u32(code + HALF_FULL_GRAPHICAL_DIFF).unwrap_or(char)
                },
                _ => char,
            })
            .collect::<String>()
    }) {
        Cow::Borrowed(borrowed) if borrowed.len() == text.len() => text,
        Cow::Borrowed(borrowed) => Cow::Owned(borrowed.to_owned()),
        Cow::Owned(owned) => Cow::Owned(owned),
    }
}

pub(crate) fn to_half_width<'a>(text: impl Into<Cow<'a, str>>) -> Cow<'a, str> {
    let text = text.into();
    match regex::FULL_GRAPHICAL_AND_IDEOGRAPHIC_SPACE.replace_all(&text, |captures: &Captures| {
        captures[0]
            .chars()
            .map(|char| match u32::from(char) {
                0x3000 => ' ',
                code @ FULL_GRAPHICAL_BEGIN..=FULL_GRAPHICAL_END => {
                    char::from_u32(code - HALF_FULL_GRAPHICAL_DIFF).unwrap_or(char)
                },
                _ => char,
            })
            .collect::<String>()
    }) {
        Cow::Borrowed(borrowed) if borrowed.len() == text.len() => text,
        Cow::Borrowed(borrowed) => Cow::Owned(borrowed.to_owned()),
        Cow::Owned(owned) => Cow::Owned(owned),
    }
}

pub(crate) fn to_katakana<'a>(text: impl Into<Cow<'a, str>>) -> Cow<'a, str> {
    let text = text.into();
    match regex::HIRAGANA.replace_all(&text, |captures: &Captures| {
        captures[0]
            .chars()
            .map(|char| match u32::from(char) {
                code @ HIRAGANA_BEGIN..=HIRAGANA_END => char::from_u32(code + HIRAGANA_KATAKANA_DIFF).unwrap_or(char),
                _ => char,
            })
            .collect::<String>()
    }) {
        Cow::Borrowed(borrowed) if borrowed.len() == text.len() => text,
        Cow::Borrowed(borrowed) => Cow::Owned(borrowed.to_owned()),
        Cow::Owned(owned) => Cow::Owned(owned),
    }
}
