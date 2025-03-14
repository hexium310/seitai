use futures::future::BoxFuture;
use sea_query::{ColumnDef, Index, PostgresQueryBuilder, Table};
use sqlx::{PgConnection, Postgres};
use sqlx_migrator::{operation::Operation, vec_box};

use crate::{speaker::DatabaseSpeaker, user::DatabaseUser};

pub(crate) struct CreateTableOperation;
pub(crate) struct CreateIndexOperation;

pub(crate) struct V1Migration;

impl Operation<Postgres> for CreateTableOperation {
    fn up<'a, 'b, 'async_trait>(&'a self, connection: &'b mut PgConnection) -> BoxFuture<'async_trait, Result<(), sqlx_migrator::error::Error>>
    where
        Self: 'async_trait,
        'a: 'async_trait,
        'b: 'async_trait,
    {
        Box::pin(async {
            let sql = Table::create()
                .if_not_exists()
                .table(DatabaseUser::Table)
                .col(ColumnDef::new(DatabaseUser::Id).big_integer().not_null().unique_key())
                .col(ColumnDef::new(DatabaseUser::SpeakerId).integer().not_null())
                .build(PostgresQueryBuilder);

            sqlx::query(&sql).execute(&mut *connection).await?;

            let sql = Table::create()
                .if_not_exists()
                .table(DatabaseSpeaker::Table)
                .col(ColumnDef::new(DatabaseSpeaker::Id).integer().not_null().primary_key())
                .col(ColumnDef::new(DatabaseSpeaker::Speed).float())
                .build(PostgresQueryBuilder);

            sqlx::query(&sql).execute(&mut *connection).await?;

            Ok(())
        })
    }

    fn down<'a, 'b, 'async_trait>(&'a self, connection: &'b mut PgConnection) -> BoxFuture<'async_trait, Result<(), sqlx_migrator::error::Error>>
    where
        Self: 'async_trait,
        'a: 'async_trait,
        'b: 'async_trait,
    {
        Box::pin(async {
            let sql = Table::drop()
                .table(DatabaseUser::Table)
                .build(PostgresQueryBuilder);

            sqlx::query(&sql).execute(&mut *connection).await?;

            let sql = Table::drop()
                .table(DatabaseSpeaker::Table)
                .build(PostgresQueryBuilder);

            sqlx::query(&sql).execute(&mut *connection).await?;

            Ok(())
        })
    }
}

impl Operation<Postgres> for CreateIndexOperation {
    fn up<'a, 'b, 'async_trait>(&'a self, connection: &'b mut PgConnection) -> BoxFuture<'async_trait, Result<(), sqlx_migrator::error::Error>>
    where
        Self: 'async_trait,
        'a: 'async_trait,
        'b: 'async_trait,
    {
        Box::pin(async {
            let sql = Index::create()
                .if_not_exists()
                .name("users_id_idx")
                .table(DatabaseUser::Table)
                .col(DatabaseUser::Id)
                .build(PostgresQueryBuilder);

            sqlx::query(&sql).execute(&mut *connection).await?;

            let sql = Index::create()
                .if_not_exists()
                .name("speakers_id_idx")
                .table(DatabaseSpeaker::Table)
                .col(DatabaseSpeaker::Id)
                .build(PostgresQueryBuilder);

            sqlx::query(&sql).execute(&mut *connection).await?;

            Ok(())
        })
    }

    fn down<'a, 'b, 'async_trait>(&'a self, connection: &'b mut PgConnection) -> BoxFuture<'async_trait, Result<(), sqlx_migrator::error::Error>>
    where
        Self: 'async_trait,
        'a: 'async_trait,
        'b: 'async_trait,
    {
        Box::pin(async {
            let sql = Index::drop()
                .name("users_id_idx")
                .build(PostgresQueryBuilder);

            sqlx::query(&sql).execute(&mut *connection).await?;

            let sql = Index::drop()
                .name("speakers_id_idx")
                .build(PostgresQueryBuilder);

            sqlx::query(&sql).execute(&mut *connection).await?;

            Ok(())
        })
    }
}

sqlx_migrator::migration!(
    sqlx::Postgres,
    V1Migration,
    "seitai",
    "create tables",
    vec_box![],
    vec_box![
        CreateTableOperation,
        CreateIndexOperation,
    ]
);
