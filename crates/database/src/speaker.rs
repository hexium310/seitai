use std::fmt::Debug;

use anyhow::{Error, Result};
use sea_query::{Iden, OnConflict, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::{FromRow, PgPool};

#[derive(Iden)]
pub(crate) enum DatabaseSpeaker {
    #[iden = "speakers"]
    Table,
    Id,
    Speed,
}

#[derive(Debug, FromRow)]
pub struct Speaker {
    pub id: i32,
    pub speed: f32,
}

pub async fn create<Id, Speed>(database: &PgPool, id: Id, speed: Speed) -> Result<Speaker>
where
    Id: TryInto<u16>,
    Speed: Into<f64>,
    <Id as TryInto<u16>>::Error: std::error::Error + Send + Sync + 'static,
{
    let (sql, values) = Query::insert()
        .into_table(DatabaseSpeaker::Table)
        .columns([DatabaseSpeaker::Id, DatabaseSpeaker::Speed])
        .values_panic([id.try_into()?.into(), speed.into().into()])
        .on_conflict(
            OnConflict::column(DatabaseSpeaker::Id)
                .update_column(DatabaseSpeaker::Speed)
                .to_owned(),
        )
        .returning(Query::returning().columns([DatabaseSpeaker::Id, DatabaseSpeaker::Speed]))
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_as_with::<_, Speaker, _>(&sql, values)
        .fetch_one(&mut *database.acquire().await?)
        .await
        .map_err(Error::msg)
}
