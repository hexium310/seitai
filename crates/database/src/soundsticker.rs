use anyhow::{Error, Result};
use futures::{TryFutureExt, TryStreamExt};
use sea_query::{Expr, Iden, OnConflict, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::{prelude::FromRow, PgPool};
use uuid::Uuid;

#[derive(Iden)]
pub(crate) enum DatabaseSoundticker {
    #[iden = "soundstickers"]
    Table,
    Id,
    StickerId,
    SoundId,
}

#[derive(Debug, Default, FromRow)]
struct DatabaseSoundstickerRow {
    pub id: Uuid,
    pub sticker_id: i64,
    pub sound_id: i64,
}

#[derive(Debug, Default)]
pub struct Soundsticker {
    pub id: Uuid,
    pub sticker_id: u64,
    pub sound_id: u64,
}

impl From<DatabaseSoundstickerRow> for Soundsticker {
    fn from(value: DatabaseSoundstickerRow) -> Self {
        Self {
            id: value.id,
            sticker_id: value.sticker_id as u64,
            sound_id: value.sound_id as u64,
        }
    }
}

pub async fn create(database: &PgPool, sticker_id: u64, sound_id: u64) -> Result<Soundsticker> {
    let (sql, values) = Query::insert()
        .into_table(DatabaseSoundticker::Table)
        .columns([
            DatabaseSoundticker::StickerId,
            DatabaseSoundticker::SoundId,
        ])
        .values_panic([sticker_id.into(), sound_id.into()])
        .on_conflict(
            OnConflict::column(DatabaseSoundticker::StickerId)
                .update_column(DatabaseSoundticker::SoundId)
                .to_owned()
        )
        .returning(Query::returning().columns([
            DatabaseSoundticker::Id,
            DatabaseSoundticker::StickerId,
            DatabaseSoundticker::SoundId,
        ]))
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_as_with::<_, DatabaseSoundstickerRow, _>(&sql, values)
        .fetch_one(&mut *database.acquire().await?)
        .map_ok(Into::into)
        .await
        .map_err(Error::msg)
}

pub async fn fetch_by_ids<T>(database: &PgPool, sticker_ids: T) -> Result<Vec<Soundsticker>>
where
    T: IntoIterator<Item = u64> + Send + Sync + 'static,
{
    let (sql, values) = Query::select()
        .columns([
            DatabaseSoundticker::Id,
            DatabaseSoundticker::StickerId,
            DatabaseSoundticker::SoundId,
        ])
        .from(DatabaseSoundticker::Table)
        .and_where(Expr::col(DatabaseSoundticker::StickerId).is_in(sticker_ids))
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_as_with::<_, DatabaseSoundstickerRow, _>(&sql, values)
        .fetch(&mut *database.acquire().await?)
        .map_ok(Into::into)
        .try_collect()
        .await
        .map_err(Error::msg)
}
