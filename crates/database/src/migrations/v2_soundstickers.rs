use futures::future::BoxFuture;
use sea_query::{ColumnDef, Expr, ForeignKey, ForeignKeyAction, Index, PgFunc, PostgresQueryBuilder, Table};
use sqlx::{PgConnection, Postgres};
use sqlx_migrator::{operation::Operation, vec_box};

use crate::{sound::DatabaseSound, soundsticker::DatabaseSoundsticker, sticker::DatabaseSticker};

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
                .table(DatabaseSound::Table)
                .col(ColumnDef::new(DatabaseSound::Id).uuid().default(PgFunc::gen_random_uuid()).primary_key())
                .col(ColumnDef::new(DatabaseSound::Name).text())
                .col(
                    ColumnDef::new(DatabaseSound::SoundId)
                        .big_integer()
                        .not_null()
                        .unique_key()
                        .check(Expr::col(DatabaseSound::SoundId).gt(0)),
                )
                .col(ColumnDef::new(DatabaseSound::GuildId).big_integer().check(Expr::col(DatabaseSound::GuildId).gt(0)))
                .build(PostgresQueryBuilder);

            sqlx::query(&sql).execute(&mut *connection).await?;

            let sql = Table::create()
                .if_not_exists()
                .table(DatabaseSticker::Table)
                .col(ColumnDef::new(DatabaseSticker::Id).uuid().default(PgFunc::gen_random_uuid()).primary_key())
                .col(ColumnDef::new(DatabaseSticker::Name).text())
                .col(
                    ColumnDef::new(DatabaseSticker::StickerId)
                        .big_integer()
                        .not_null()
                        .unique_key()
                        .check(Expr::col(DatabaseSticker::StickerId).gt(0)),
                )
                .col(ColumnDef::new(DatabaseSticker::GuildId).big_integer().check(Expr::col(DatabaseSticker::GuildId).gt(0)))
                .build(PostgresQueryBuilder);

            sqlx::query(&sql).execute(&mut *connection).await?;

            let sql = Table::create()
                .if_not_exists()
                .table(DatabaseSoundsticker::Table)
                .col(ColumnDef::new(DatabaseSoundsticker::Id).uuid().default(PgFunc::gen_random_uuid()).primary_key())
                .col(
                    ColumnDef::new(DatabaseSoundsticker::StickerId)
                        .uuid()
                        .not_null()
                        .unique_key()
                )
                .col(ColumnDef::new(DatabaseSoundsticker::SoundId).uuid().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .from(DatabaseSoundsticker::Table, DatabaseSoundsticker::StickerId)
                        .to(DatabaseSticker::Table, DatabaseSticker::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .from(DatabaseSoundsticker::Table, DatabaseSoundsticker::SoundId)
                        .to(DatabaseSound::Table, DatabaseSound::Id)
                        .on_delete(ForeignKeyAction::Cascade),
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
                .table(DatabaseSoundsticker::Table)
                .table(DatabaseSticker::Table)
                .table(DatabaseSound::Table)
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
                .name("sounds_sound_id_idx")
                .table(DatabaseSoundsticker::Table)
                .col(DatabaseSoundsticker::StickerId)
                .build(PostgresQueryBuilder);

            sqlx::query(&sql).execute(&mut *connection).await?;

            let sql = Index::create()
                .if_not_exists()
                .name("stickers_sticker_id_idx")
                .table(DatabaseSoundsticker::Table)
                .col(DatabaseSoundsticker::StickerId)
                .build(PostgresQueryBuilder);

            sqlx::query(&sql).execute(&mut *connection).await?;

            let sql = Index::create()
                .if_not_exists()
                .name("soundstickers_sticker_id_idx")
                .table(DatabaseSoundsticker::Table)
                .col(DatabaseSoundsticker::StickerId)
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
                .name("sounds_sound_id_idx")
                .build(PostgresQueryBuilder);

            sqlx::query(&sql).execute(&mut *connection).await?;

            let sql = Index::drop()
                .name("stickers_sticker_id_idx")
                .build(PostgresQueryBuilder);

            sqlx::query(&sql).execute(&mut *connection).await?;

            let sql = Index::drop()
                .name("soundstickers_sticker_id_idx")
                .build(PostgresQueryBuilder);

            sqlx::query(&sql).execute(&mut *connection).await?;

            Ok(())
        })
    }
}

sqlx_migrator::migration!(
    sqlx::Postgres,
    V2Migration,
    "seitai",
    "create soundstickers",
    vec_box![],
    vec_box![
        CreateTableOperation,
        CreateIndexOperation,
    ]
);
