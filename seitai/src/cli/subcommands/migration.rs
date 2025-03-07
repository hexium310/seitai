use anyhow::Result;
use clap::Parser;
use database::migrations::{MigrationCommand, Migrator};

use crate::set_up_database;

#[derive(Debug, Parser)]
pub struct Migration {
    #[command(flatten)]
    pub command: MigrationCommand,
}

impl Migration {
    pub async fn run(&self) -> Result<()> {
        let migrator = Migrator::new();
        let pgpool = set_up_database().await?;
        self.command.run(&mut *pgpool.acquire().await?, migrator.into_boxed_inner()).await?;

        Ok(())
    }
}
