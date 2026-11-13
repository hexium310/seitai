use futures::future::BoxFuture;
use sea_query::{Index, PostgresQueryBuilder};
use sqlx::{AssertSqlSafe, PgConnection, Postgres};
use sqlx_migrator::{operation::Operation, vec_box};

use crate::soundsticker::{DatabaseSoundsticker, DatabaseSoundstickerIndex};

struct CreateIndexOperation;

struct DeleteIndexOperation;

pub(crate) struct V3Migration;

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
                .name(DatabaseSoundstickerIndex::SoundId)
                .table(DatabaseSoundsticker::Table)
                .col(DatabaseSoundsticker::SoundId)
                .build(PostgresQueryBuilder);

            sqlx::query(AssertSqlSafe(sql.as_str())).execute(&mut *connection).await?;

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
                .name(DatabaseSoundstickerIndex::SoundId)
                .build(PostgresQueryBuilder);

            sqlx::query(AssertSqlSafe(sql.as_str())).execute(&mut *connection).await?;

            Ok(())
        })
    }
}

impl Operation<Postgres> for DeleteIndexOperation {
    fn up<'a, 'b, 'async_trait>(&'a self, connection: &'b mut PgConnection) -> BoxFuture<'async_trait, Result<(), sqlx_migrator::error::Error>>
    where
        Self: 'async_trait,
        'a: 'async_trait,
        'b: 'async_trait,
    {
        Box::pin(async {
            let sql = Index::drop()
                .if_exists()
                .name("sounds_sound_id_idx")
                .build(PostgresQueryBuilder);

            sqlx::query(AssertSqlSafe(sql.as_str())).execute(&mut *connection).await?;

            let sql = Index::drop()
                .if_exists()
                .name("stickers_sticker_id_idx")
                .build(PostgresQueryBuilder);

            sqlx::query(AssertSqlSafe(sql.as_str())).execute(&mut *connection).await?;

            let sql = Index::drop()
                .name("soundstickers_sticker_id_idx")
                .build(PostgresQueryBuilder);

            sqlx::query(AssertSqlSafe(sql.as_str())).execute(&mut *connection).await?;

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
            let sql = Index::create()
                .if_not_exists()
                .name("sounds_sound_id_idx")
                .table(DatabaseSoundsticker::Table)
                .col(DatabaseSoundsticker::StickerId)
                .build(PostgresQueryBuilder);

            sqlx::query(AssertSqlSafe(sql.as_str())).execute(&mut *connection).await?;

            let sql = Index::create()
                .if_not_exists()
                .name("stickers_sticker_id_idx")
                .table(DatabaseSoundsticker::Table)
                .col(DatabaseSoundsticker::StickerId)
                .build(PostgresQueryBuilder);

            sqlx::query(AssertSqlSafe(sql.as_str())).execute(&mut *connection).await?;

            let sql = Index::create()
                .if_not_exists()
                .name("soundstickers_sticker_id_idx")
                .table(DatabaseSoundsticker::Table)
                .col(DatabaseSoundsticker::StickerId)
                .build(PostgresQueryBuilder);

            sqlx::query(AssertSqlSafe(sql.as_str())).execute(&mut *connection).await?;

            Ok(())
        })
    }
}

sqlx_migrator::migration!(
    sqlx::Postgres,
    V3Migration,
    "seitai",
    "fix soundstickers index",
    vec_box![],
    vec_box![
        CreateIndexOperation,
        DeleteIndexOperation,
    ]
);

