use anyhow::{Error, Result};
use futures::TryStreamExt;
use sea_query::{Expr, JoinType, OnConflict, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::{FromRow, PgPool};

use super::identifier;

#[derive(Debug, FromRow)]
#[allow(dead_code)]
pub(crate) struct User {
    pub(crate) id: i64,
    pub(crate) speaker_id: i32,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
pub(crate) struct UserSpeaker {
    pub(crate) id: i64,
    pub(crate) speaker_id: i32,
    pub(crate) speed: Option<f32>,
}

impl Default for User {
    fn default() -> Self {
        Self { id: 0, speaker_id: 1 }
    }
}

impl Default for UserSpeaker {
    fn default() -> Self {
        Self {
            id: 0,
            speaker_id: 1,
            speed: Some(1.2),
        }
    }
}

pub(crate) async fn create(database: &PgPool, user_id: u64, speaker_id: u16) -> Result<User> {
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
        .fetch_one(&mut *database.acquire().await?)
        .await
        .map_err(Error::msg)
}

pub(crate) async fn fetch_by_ids(database: &PgPool, ids: &[i64]) -> Result<Vec<User>> {
    let (sql, values) = Query::select()
        .columns([identifier::User::Id, identifier::User::SpeakerId])
        .from(identifier::User::Table)
        .and_where(Expr::col(identifier::User::Id).is_in(ids.iter().cloned()))
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_as_with::<_, User, _>(&sql, values)
        .fetch(&mut *database.acquire().await?)
        .try_collect()
        .await
        .map_err(Error::msg)
}

pub(crate) async fn fetch_with_speaker_by_ids(database: &PgPool, ids: &[i64]) -> Result<Vec<UserSpeaker>> {
    let (sql, values) = Query::select()
        .columns([
            (identifier::User::Table, identifier::User::Id),
            (identifier::User::Table, identifier::User::SpeakerId),
        ])
        .columns([identifier::Speaker::Speed])
        .from(identifier::User::Table)
        .join(
            JoinType::LeftJoin,
            identifier::Speaker::Table,
            Expr::col((identifier::User::Table, identifier::User::SpeakerId))
                .equals((identifier::Speaker::Table, identifier::Speaker::Id)),
        )
        .and_where(Expr::col((identifier::User::Table, identifier::User::Id)).is_in(ids.iter().cloned()))
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_as_with::<_, UserSpeaker, _>(&sql, values)
        .fetch(&mut *database.acquire().await?)
        .try_collect()
        .await
        .map_err(Error::msg)
}
