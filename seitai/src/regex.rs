use lazy_regex::{lazy_regex, Lazy, Regex};

pub(crate) static CODE: Lazy<Regex> = lazy_regex!(r"(?:`[^`]+`|```[^`]+```)");
pub(crate) static EMOJI: Lazy<Regex> = lazy_regex!(r"<(?:a)?:([[:word:]]+):\d+>");
pub(crate) static FULL_GRAPHICAL_AND_IDEOGRAPHIC_SPACE: Lazy<Regex> = lazy_regex!(r"[\u3000！-～]+");
pub(crate) static HALF_GRAPHICAL: Lazy<Regex> = lazy_regex!(r"[!-~]+");
pub(crate) static HIRAGANA: Lazy<Regex> = lazy_regex!(r"[ぁ-ゖ]+");
pub(crate) static IDEOGRAPHIC_FULL_STOP: Lazy<Regex> = lazy_regex!(r"。");
pub(crate) static MENTION_CHANNEL: Lazy<Regex> = lazy_regex!(r"<[@#].+>");
pub(crate) static SOUNDMOJI: Lazy<Regex> = lazy_regex!(r"<sound:(?<guild_id>\d+):(?<sound_id>\d+)>");
pub(crate) static URL: Lazy<Regex> = lazy_regex!(r"[[:alpha:]][[:alnum:]+\-.]*?://[^\s]+");
pub(crate) static W: Lazy<Regex> = lazy_regex!(r"([^ｗ[:word:]]|^)[wｗ]([^ｗ[:word:]]|$)");
pub(crate) static WW: Lazy<Regex> = lazy_regex!(r"([^ｗ[:word:]]|^)[wｗ]{2,}([^ｗ[:word:]]|$)");
pub(crate) static WORD: Lazy<Regex> = lazy_regex!(r"[[:alpha:]'-]{2,}");
