use sea_query::Iden;
use sqlx::prelude::FromRow;
use uuid::Uuid;

#[derive(Iden)]
pub(crate) enum DatabaseSound {
    #[iden = "sounds"]
    Table,
    Id,
    Name,
    SoundId,
    GuildId,
}

#[derive(Debug, Default, FromRow)]
pub(crate) struct  DatabaseSoundRow {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) sound_id: i64,
    pub(crate) guild_id: Option<i64>,
}

#[derive(Debug, Default)]
pub struct Sound {
    pub id: Uuid,
    pub name: String,
    pub sound_id: u64,
    pub guild_id: Option<u64>,
}

impl From<DatabaseSoundRow> for Sound {
    fn from(value: DatabaseSoundRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            sound_id: value.sound_id as u64,
            guild_id: value.guild_id.map(|v| v as u64),
        }
    }
}
