use futures::future::BoxFuture;
use sea_query::{ColumnDef, Expr, Index, PgFunc, PostgresQueryBuilder, Table};
use sqlx::{PgConnection, Postgres};
use sqlx_migrator::{migration::Migration, operation::Operation, vec_box};

use crate::sticker::DatabaseSoundticker;

pub(crate) struct CreateTableOperation;
pub(crate) struct CreateIndexOperation;

pub(crate) struct V2Migration;

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
                .table(DatabaseSoundticker::Table)
                .col(ColumnDef::new(DatabaseSoundticker::Id).uuid().default(PgFunc::gen_random_uuid()).primary_key())
                .col(
                    ColumnDef::new(DatabaseSoundticker::StickerId)
                        .big_integer()
                        .not_null()
                        .unique_key()
                        .check(Expr::col(DatabaseSoundticker::StickerId).gt(0)),
                )
                .col(
                    ColumnDef::new(DatabaseSoundticker::SoundId)
                        .big_integer()
                        .not_null()
                        .check(Expr::col(DatabaseSoundticker::SoundId).gt(0)),
                )
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
                .table(DatabaseSoundticker::Table)
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
                .name("soundstickers_sticker_id_idx")
                .table(DatabaseSoundticker::Table)
                .col(DatabaseSoundticker::StickerId)
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
                .name("soundstickers_sticker_id_idx")
                .build(PostgresQueryBuilder);

            sqlx::query(&sql).execute(&mut *connection).await?;

            Ok(())
        })
    }
}

// TODO: use macro after new sqlx_migrator version is releaseed
impl Migration<Postgres> for V2Migration {
    fn app(&self) -> &str {
        "seitai"
    }

    fn name(&self) -> &str {
        "create soundstickers table"
    }

    fn parents(&self) -> Vec<Box<dyn Migration<Postgres>>> {
        vec_box![]
    }

    fn operations(&self) -> Vec<Box<dyn Operation<Postgres>>> {
        vec_box![
            CreateTableOperation,
            CreateIndexOperation,
        ]
    }
}
