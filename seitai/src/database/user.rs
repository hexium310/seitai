use anyhow::{Error, Result};
use sea_query::{Expr, OnConflict, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use serenity::futures::TryStreamExt;
use sqlx::{FromRow, PgPool};

use super::identifier;

#[derive(Debug, FromRow)]
#[allow(dead_code)]
pub(crate) struct User {
    pub(crate) id: i64,
    pub(crate) speaker_id: i32,
}

impl Default for User {
    fn default() -> Self {
        Self { id: 0, speaker_id: 1 }
    }
}

pub(crate) async fn create(database: &PgPool, user_id: u64, speaker_id: u16) -> Result<User> {
    let mut connection = database.acquire().await?;

    let (sql, values) = Query::insert()
        .into_table(identifier::User::Table)
        .columns([identifier::User::Id, identifier::User::SpeakerId])
        .values_panic([user_id.into(), speaker_id.into()])
        .on_conflict(
            OnConflict::column(identifier::User::Id)
                .update_column(identifier::User::SpeakerId)
                .to_owned(),
        )
        .returning(Query::returning().columns([identifier::User::Id, identifier::User::SpeakerId]))
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_as_with::<_, User, _>(&sql, values)
        .fetch_one(&mut *connection)
        .await
        .map_err(Error::msg)
}

pub(crate) async fn fetch_by_ids(database: &PgPool, ids: &[i64]) -> Result<Vec<User>> {
    let mut connection = database.acquire().await?;

    let (sql, values) = Query::select()
        .columns([identifier::User::Id, identifier::User::SpeakerId])
        .from(identifier::User::Table)
        .and_where(Expr::col(identifier::User::Id).is_in(ids.iter().cloned()))
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_as_with::<_, User, _>(&sql, values)
        .fetch(&mut *connection)
        .try_collect()
        .await
        .map_err(Error::msg)
}
