use sea_query::Iden;
use sqlx::prelude::FromRow;
use uuid::Uuid;

#[derive(Iden)]
pub(crate) enum DatabaseSticker {
    #[iden = "stickers"]
    Table,
    Id,
    Name,
    StickerId,
    GuildId,
}

#[derive(Debug, Default, FromRow)]
pub(crate) struct DatabaseStickerRow {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) sticker_id: i64,
    pub(crate) guild_id: Option<i64>,
}

#[derive(Debug, Default)]
pub struct Sticker {
    pub id: Uuid,
    pub name: String,
    pub sticker_id: u64,
    pub guild_id: Option<u64>,
}

impl From<DatabaseStickerRow> for Sticker {
    fn from(value: DatabaseStickerRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            sticker_id: value.sticker_id as u64,
            guild_id: value.guild_id.map(|v| v as u64),
        }
    }
}
