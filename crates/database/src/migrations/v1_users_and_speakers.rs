use futures::future::BoxFuture;
use sea_query::{ColumnDef, Index, PostgresQueryBuilder, Table};
use sqlx::{PgConnection, Postgres};
use sqlx_migrator::{migration::Migration, operation::Operation, vec_box};

use crate::{speaker::DatabaseSpeaker, user::DatabaseUser};

pub(crate) struct CreateSchemaOperation;
pub(crate) struct CreateTableOperation;
pub(crate) struct CreateIndexOperation;

pub(crate) struct V1Migration;

impl Operation<Postgres> for CreateSchemaOperation {
    fn up<'a, 'b, 'async_trait>(&'a self, connection: &'b mut PgConnection) -> BoxFuture<'async_trait, Result<(), sqlx_migrator::error::Error>>
    where
        Self: 'async_trait,
        'a: 'async_trait,
        'b: 'async_trait,
    {
        Box::pin(async {
            sqlx::query("CREATE SCHEMA IF NOT EXISTS seitai AUTHORIZATION seitai").execute(&mut *connection).await?;

            Ok(())
        })
    }

    fn down<'a, 'b, 'async_trait>(&'a self, _connection: &'b mut PgConnection) -> BoxFuture<'async_trait, Result<(), sqlx_migrator::error::Error>>
    where
        Self: 'async_trait,
        'a: 'async_trait,
        'b: 'async_trait,
    {
        Box::pin(async {
            // The `seitai` schema is used as the _sqlx_migrator_migrations schema.
            // If dropping the `seitai` schema with `CASCADE`, _sqlx_migrator_migrations is also dropped, so reverting migration fails by sqlx_migrator.
            //sqlx::query("DROP SCHEMA seitai").execute(&mut *connection).await?;

            Ok(())
        })
    }
}

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

// TODO: use macro after new sqlx_migrator version is releaseed
impl Migration<Postgres> for V1Migration {
    fn app(&self) -> &str {
        "seitai"
    }

    fn name(&self) -> &str {
        "create tables"
    }

    fn parents(&self) -> Vec<Box<dyn Migration<Postgres>>> {
        vec_box![]
    }

    fn operations(&self) -> Vec<Box<dyn Operation<Postgres>>> {
        vec_box![
            CreateSchemaOperation,
            CreateTableOperation,
            CreateIndexOperation,
        ]
    }
}
