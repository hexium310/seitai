use anyhow::{Error, Result};
use futures::TryStreamExt;
use sea_query::{Expr, Iden, JoinType, OnConflict, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::{FromRow, PgPool};

use crate::speaker::DatabaseSpeaker;

#[derive(Iden)]
pub(crate) enum DatabaseUser {
    #[iden = "users"]
    Table,
    Id,
    SpeakerId,
}

#[derive(Debug, FromRow)]
pub struct User {
    pub id: i64,
    pub speaker_id: i32,
}

#[derive(Debug, FromRow)]
pub struct UserSpeaker {
    pub id: i64,
    pub speaker_id: i32,
    pub speed: Option<f32>,
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

pub async fn create(database: &PgPool, user_id: u64, speaker_id: u16) -> Result<User> {
    let (sql, values) = Query::insert()
        .into_table(DatabaseUser::Table)
        .columns([DatabaseUser::Id, DatabaseUser::SpeakerId])
        .values_panic([user_id.into(), speaker_id.into()])
        .on_conflict(
            OnConflict::column(DatabaseUser::Id)
                .update_column(DatabaseUser::SpeakerId)
                .to_owned(),
        )
        .returning(Query::returning().columns([DatabaseUser::Id, DatabaseUser::SpeakerId]))
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_as_with::<_, User, _>(&sql, values)
        .fetch_one(&mut *database.acquire().await?)
        .await
        .map_err(Error::msg)
}

pub async fn fetch_by_ids(database: &PgPool, ids: &[i64]) -> Result<Vec<User>> {
    let (sql, values) = Query::select()
        .columns([DatabaseUser::Id, DatabaseUser::SpeakerId])
        .from(DatabaseUser::Table)
        .and_where(Expr::col(DatabaseUser::Id).is_in(ids.iter().cloned()))
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_as_with::<_, User, _>(&sql, values)
        .fetch(&mut *database.acquire().await?)
        .try_collect()
        .await
        .map_err(Error::msg)
}

pub async fn fetch_with_speaker_by_ids(database: &PgPool, ids: &[i64]) -> Result<Vec<UserSpeaker>> {
    let (sql, values) = Query::select()
        .columns([
            (DatabaseUser::Table, DatabaseUser::Id),
            (DatabaseUser::Table, DatabaseUser::SpeakerId),
        ])
        .columns([DatabaseSpeaker::Speed])
        .from(DatabaseUser::Table)
        .join(
            JoinType::LeftJoin,
            DatabaseSpeaker::Table,
            Expr::col((DatabaseUser::Table, DatabaseUser::SpeakerId))
                .equals((DatabaseSpeaker::Table, DatabaseSpeaker::Id)),
        )
        .and_where(Expr::col((DatabaseUser::Table, DatabaseUser::Id)).is_in(ids.iter().cloned()))
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_as_with::<_, UserSpeaker, _>(&sql, values)
        .fetch(&mut *database.acquire().await?)
        .try_collect()
        .await
        .map_err(Error::msg)
}
