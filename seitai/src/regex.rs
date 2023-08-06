use lazy_regex::{Lazy, lazy_regex, Regex};

pub(crate) static CODE: Lazy<Regex> = lazy_regex!(r"(?:`[^`]+`|```[^`]+```)");
pub(crate) static EMOJI: Lazy<Regex> = lazy_regex!(r"<:([\w_]+):\d+>");
pub(crate) static IDEOGRAPHIC_FULL_STOP: Lazy<Regex> = lazy_regex!(r"。");
pub(crate) static IDEOGRAPHIC_SPACE: Lazy<Regex> = lazy_regex!(r"\u3000");
pub(crate) static URL: Lazy<Regex> = lazy_regex!(r"[[:alpha:]][[:alnum:]+\-.]*?://[^\s]+");
pub(crate) static W: Lazy<Regex> = lazy_regex!(r"[wｗ]{2,}");
pub(crate) static WW: Lazy<Regex> = lazy_regex!(r"[wｗ]$");
