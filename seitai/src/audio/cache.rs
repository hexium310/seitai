use std::{marker::PhantomData, str::FromStr};

use strum::{AsRefStr, EnumString};

#[derive(Debug, Clone, EnumString, AsRefStr)]
pub(crate) enum PredefinedUtterance {
    #[strum(serialize = "コード省略")]
    Code,
    #[strum(serialize = "URL")]
    Url,
    #[strum(serialize = "接続しました")]
    Connected,
    #[strum(serialize = "添付ファイル")]
    Attachment,
    #[strum(serialize = "を登録しました")]
    Registered,
}

pub(crate) struct ConstCacheable<Utterance> {
    _marker: PhantomData<fn() -> Utterance>,
}

#[cfg_attr(test, mockall::automock)]
pub(crate) trait Cacheable {
    fn should_cache(&self, text: &str) -> bool;
}

impl<Utterance> ConstCacheable<Utterance> {
    pub(crate) fn new() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<Utterance> Cacheable for ConstCacheable<Utterance>
where
    Utterance: FromStr,
{
    fn should_cache(&self, text: &str) -> bool {
        Utterance::from_str(text).is_ok()
    }
}
