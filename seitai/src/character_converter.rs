use crate::regex;

const HALF_GRAPHICAL_BEGIN: u32 = '!' as u32;
const HALF_GRAPHICAL_END: u32 = '~' as u32;
const FULL_GRAPHICAL_BEGIN: u32 = '！' as u32;
const FULL_GRAPHICAL_END: u32 = '～' as u32;
const HALF_FULL_GRAPHICAL_DIFF: u32 = 0xFEE0;
const HIRAGANA_BEGIN: u32 = 'ぁ' as u32;
const HIRAGANA_END: u32 = 'ゖ' as u32;
const HIRAGANA_KATAKANA_DIFF: u32 = 0x60;

pub(crate) fn to_full_width(text: &str) -> String {
    text.chars()
        .map(|char| match u32::from(char) {
            code @ HALF_GRAPHICAL_BEGIN..=HALF_GRAPHICAL_END => {
                char::from_u32(code + HALF_FULL_GRAPHICAL_DIFF).unwrap_or(char)
            },
            _ => char,
        })
        .collect()
}

pub(crate) fn to_half_width(text: &str) -> String {
    regex::IDEOGRAPHIC_SPACE
        .replace_all(text, " ")
        .chars()
        .map(|char| match u32::from(char) {
            code @ FULL_GRAPHICAL_BEGIN..=FULL_GRAPHICAL_END => {
                char::from_u32(code - HALF_FULL_GRAPHICAL_DIFF).unwrap_or(char)
            },
            _ => char,
        })
        .collect()
}

pub(crate) fn to_katakana(text: &str) -> String {
    text.chars()
        .map(|char| match u32::from(char) {
            code @ HIRAGANA_BEGIN..=HIRAGANA_END => char::from_u32(code + HIRAGANA_KATAKANA_DIFF).unwrap_or(char),
            _ => char,
        })
        .collect()
}
