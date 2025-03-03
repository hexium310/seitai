use std::fmt::Debug;

use anyhow::Result;
use futures::TryStreamExt;
use sea_query::{Alias, Expr, Iden, OnConflict, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::{postgres::PgRow, prelude::FromRow, PgPool, Row};
use uuid::Uuid;

use crate::{sound::{DatabaseSound, DatabaseSoundRow}, sticker::{DatabaseSticker, DatabaseStickerRow}};

#[derive(Iden)]
pub(crate) enum DatabaseSoundsticker {
    #[iden = "soundstickers"]
    Table,
    Id,
    StickerId,
    SoundId,
}

#[derive(Debug, Default, FromRow)]
#[allow(dead_code)]
struct DatabaseSoundstickerRow {
    pub(crate) id: Uuid,
    pub(crate) sticker_id: Uuid,
    pub(crate) sound_id: Uuid,
}

#[derive(Debug, Default, Clone)]
pub struct Soundsticker {
    pub id: Uuid,
    pub sticker_name: String,
    pub sticker_id: u64,
    pub sticker_guild_id: Option<u64>,
    pub sound_name: String,
    pub sound_id: u64,
    pub sound_guild_id: Option<u64>,
}

impl FromRow<'_, PgRow> for Soundsticker {
    fn from_row(row: &'_ PgRow) -> std::result::Result<Self, sqlx::Error> {
        let sticker_id: i64 = row.try_get("sound_id")?;
        let sound_id: i64 = row.try_get("sound_id")?;

        Ok(Self {
            id: row.try_get("id")?,
            sticker_name: row.try_get("sticker_name")?,
            sticker_id: sticker_id as u64,
            sticker_guild_id: row.try_get("sticker_guild_id").map(|o: Option<i64>| o.map(|v| v as u64))?,
            sound_name: row.try_get("sound_name")?,
            sound_id: sound_id as u64,
            sound_guild_id: row.try_get("sound_guild_id").map(|o: Option<i64>| o.map(|v| v as u64))?
        })
    }
}

#[tracing::instrument(skip(database))]
pub async fn create(
    database: &PgPool,
    sticker_name: impl Into<String> + Debug,
    sticker_id: u64,
    sticker_guild_id: Option<u64>,
    sound_name: impl Into<String> + Debug,
    sound_id: u64,
    sound_guild_id: Option<u64>,
) -> Result<Soundsticker> {
    let mut tx = database.begin().await?;

    let (sql, values) = Query::insert()
        .into_table(DatabaseSound::Table)
        .columns([
            DatabaseSound::Name,
            DatabaseSound::SoundId,
            DatabaseSound::GuildId,
        ])
        .values_panic([sound_name.into().into(), sound_id.into(), sound_guild_id.into()])
        .on_conflict(OnConflict::column(DatabaseSound::SoundId).update_columns([DatabaseSound::Name]).to_owned())
        .returning(Query::returning().columns([
            DatabaseSound::Id,
            DatabaseSound::Name,
            DatabaseSound::SoundId,
            DatabaseSound::GuildId,
        ]))
        .build_sqlx(PostgresQueryBuilder);

    let sound_row = match sqlx::query_as_with::<_, DatabaseSoundRow, _>(&sql, values)
        .fetch_one(&mut *tx)
        .await
    {
        Ok(sound_row) => sound_row,
        Err(err) => {
            tracing::error!("failed to insert sound\nError: {err:?}");
            return Err(err.into())
        },
    };

    let (sql, values) = Query::insert()
        .into_table(DatabaseSticker::Table)
        .columns([
            DatabaseSticker::Name,
            DatabaseSticker::StickerId,
            DatabaseSticker::GuildId,
        ])
        .values_panic([sticker_name.into().into(), sticker_id.into(), sticker_guild_id.into()])
        .on_conflict(OnConflict::column(DatabaseSticker::StickerId).update_columns([DatabaseSticker::Name]).to_owned())
        .returning(Query::returning().columns([
            DatabaseSticker::Id,
            DatabaseSticker::Name,
            DatabaseSticker::StickerId,
            DatabaseSticker::GuildId,
        ]))
        .build_sqlx(PostgresQueryBuilder);

    let sticker_row = match sqlx::query_as_with::<_, DatabaseStickerRow, _>(&sql, values)
        .fetch_one(&mut *tx)
        .await
    {
        Ok(sticker_row) => sticker_row,
        Err(err) => {
            tracing::error!("failed to insert sticker\nError: {err:?}");
            return Err(err.into())
        },
    };

    let (sql, values) = Query::insert()
        .into_table(DatabaseSoundsticker::Table)
        .columns([
            DatabaseSoundsticker::StickerId,
            DatabaseSoundsticker::SoundId,
        ])
        .values_panic([sticker_row.id.into(), sound_row.id.into()])
        .on_conflict(OnConflict::column(DatabaseSoundsticker::StickerId).update_column(DatabaseSoundsticker::SoundId).to_owned())
        .returning(Query::returning().columns([
            DatabaseSoundsticker::Id,
            DatabaseSoundsticker::StickerId,
            DatabaseSoundsticker::SoundId,
        ]))
        .build_sqlx(PostgresQueryBuilder);

    let soundsticker_row = match sqlx::query_as_with::<_, DatabaseSoundstickerRow, _>(&sql, values)
        .fetch_one(&mut *tx)
        .await
    {
        Ok(soundsticker_row) => soundsticker_row,
        Err(err) => {
            tracing::error!("failed to insert soundsticker\nError: {err:?}");
            return Err(err.into())
        },
    };

    tx.commit().await?;

    Ok(
        Soundsticker {
            id: soundsticker_row.id,
            sticker_name: sticker_row.name,
            sticker_id: sticker_row.sticker_id as u64,
            sticker_guild_id: sticker_row.guild_id.map(|v| v as u64),
            sound_name: sound_row.name,
            sound_id: sound_row.sound_id as u64,
            sound_guild_id: sound_row.guild_id.map(|v| v as u64),
        }
    )
}

#[tracing::instrument(skip(database))]
pub async fn fetch_by_ids<T>(database: &PgPool, sticker_ids: T) -> Result<Vec<Soundsticker>>
where
    T: IntoIterator<Item = u64> + Debug + Send + Sync + 'static,
{
    let (sql, values) = Query::select()
        .column((DatabaseSoundsticker::Table, DatabaseSoundsticker::Id))
        .column((DatabaseSound::Table, DatabaseSound::SoundId))
        .expr_as(Expr::col((DatabaseSound::Table, DatabaseSound::Name)), Alias::new("sound_name"))
        .expr_as(Expr::col((DatabaseSound::Table, DatabaseSound::GuildId)), Alias::new("sound_guild_id"))
        .column((DatabaseSticker::Table, DatabaseSticker::StickerId))
        .expr_as(Expr::col((DatabaseSticker::Table, DatabaseSticker::Name)), Alias::new("sticker_name"))
        .expr_as(Expr::col((DatabaseSticker::Table, DatabaseSticker::GuildId)), Alias::new("sticker_guild_id"))
        .from(DatabaseSoundsticker::Table)
        .inner_join(
            DatabaseSound::Table,
            Expr::col((DatabaseSoundsticker::Table, DatabaseSoundsticker::SoundId))
                .equals((DatabaseSound::Table, DatabaseSound::Id))
        )
        .inner_join(
            DatabaseSticker::Table,
            Expr::col((DatabaseSoundsticker::Table, DatabaseSoundsticker::StickerId))
                .equals((DatabaseSticker::Table, DatabaseSticker::Id))
        )
        .and_where(Expr::col((DatabaseSticker::Table, DatabaseSticker::StickerId)).is_in(sticker_ids))
        .build_sqlx(PostgresQueryBuilder);

    match sqlx::query_as_with::<_, Soundsticker, _>(&sql, values)
        .fetch(&mut *database.acquire().await?)
        .map_ok(Into::into)
        .try_collect()
        .await
    {
        Ok(soundsticker) => Ok(soundsticker),
        Err(err) => {
            tracing::error!("failed to get soundstickers\nError: {err:?}");
            Err(err.into())
        },
    }
}

#[tracing::instrument(skip(database))]
pub async fn fetch_all(database: &PgPool) -> Result<Vec<Soundsticker>> {
    let (sql, values) = Query::select()
        .column((DatabaseSoundsticker::Table, DatabaseSoundsticker::Id))
        .column((DatabaseSound::Table, DatabaseSound::SoundId))
        .expr_as(Expr::col((DatabaseSound::Table, DatabaseSound::Name)), Alias::new("sound_name"))
        .expr_as(Expr::col((DatabaseSound::Table, DatabaseSound::GuildId)), Alias::new("sound_guild_id"))
        .column((DatabaseSticker::Table, DatabaseSticker::StickerId))
        .expr_as(Expr::col((DatabaseSticker::Table, DatabaseSticker::Name)), Alias::new("sticker_name"))
        .expr_as(Expr::col((DatabaseSticker::Table, DatabaseSticker::GuildId)), Alias::new("sticker_guild_id"))
        .from(DatabaseSoundsticker::Table)
        .inner_join(
            DatabaseSound::Table,
            Expr::col((DatabaseSoundsticker::Table, DatabaseSoundsticker::SoundId))
                .equals((DatabaseSound::Table, DatabaseSound::Id))
        )
        .inner_join(
            DatabaseSticker::Table,
            Expr::col((DatabaseSoundsticker::Table, DatabaseSoundsticker::StickerId))
                .equals((DatabaseSticker::Table, DatabaseSticker::Id))
        )
        .build_sqlx(PostgresQueryBuilder);

    match sqlx::query_as_with::<_, Soundsticker, _>(&sql, values)
        .fetch(&mut *database.acquire().await?)
        .map_ok(Into::into)
        .try_collect()
        .await
    {
        Ok(soundsticker) => Ok(soundsticker),
        Err(err) => {
            tracing::error!("failed to get soundsticker\nError: {err:?}");
            Err(err.into())
        },
    }
}

#[tracing::instrument(skip(database))]
pub async fn delete_by_id(database: &PgPool, id: u64) -> Result<Option<Soundsticker>> {
    let soundstickers = fetch_by_ids(database, vec![id]).await?;

    let soundsticker = match soundstickers.first() {
        Some(soundsticker) => soundsticker,
        None => return Ok(None),
    };

    let (sql, values) = Query::delete()
        .from_table(DatabaseSoundsticker::Table)
        .and_where(Expr::col(DatabaseSoundsticker::Id).eq(soundsticker.id))
        .build_sqlx(PostgresQueryBuilder);

    if let Err(err) = sqlx::query_with(&sql, values).execute(&mut *database.acquire().await?).await {
        tracing::error!("failed to get soundsticker\nError: {err:?}");
        return Err(err.into());
    };

    Ok(Some(soundsticker.to_owned()))
}
