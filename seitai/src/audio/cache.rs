use strum::{AsRefStr, EnumString};

#[derive(Debug, Clone, EnumString, AsRefStr)]
pub(crate) enum CacheTarget {
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
