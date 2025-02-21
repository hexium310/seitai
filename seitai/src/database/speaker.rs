use std::fmt::Debug;

use anyhow::{Error, Result};
use sea_query::{OnConflict, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::{FromRow, PgPool};

use super::identifier;

#[derive(Debug, FromRow)]
#[allow(dead_code)]
pub(crate) struct Speaker {
    pub(crate) id: i32,
    pub(crate) speed: f32,
}

pub(crate) async fn create<Id, Speed>(database: &PgPool, id: Id, speed: Speed) -> Result<Speaker>
where
    Id: TryInto<u16>,
    Speed: Into<f64>,
    <Id as TryInto<u16>>::Error: std::error::Error + Send + Sync + 'static,
{
    let (sql, values) = Query::insert()
        .into_table(identifier::Speaker::Table)
        .columns([identifier::Speaker::Id, identifier::Speaker::Speed])
        .values_panic([id.try_into()?.into(), speed.into().into()])
        .on_conflict(
            OnConflict::column(identifier::Speaker::Id)
                .update_column(identifier::Speaker::Speed)
                .to_owned(),
        )
        .returning(Query::returning().columns([identifier::Speaker::Id, identifier::Speaker::Speed]))
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_as_with::<_, Speaker, _>(&sql, values)
        .fetch_one(&mut *database.acquire().await?)
        .await
        .map_err(Error::msg)
}
