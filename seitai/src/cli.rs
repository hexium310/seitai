use std::{env, process::exit};

use anyhow::Result;
use clap::{error::ErrorKind, Parser};
use database::migrations::{MigrationCommand, Migrator};

use crate::{set_up_database, start_bot};

pub struct Application;

#[derive(clap::Parser)]
#[command(arg_required_else_help = false)]
#[command(subcommand_required = false)]
pub struct Cli {
    #[command(subcommand)]
    subcommand: Subcommand,
}

#[derive(clap::Subcommand)]
enum Subcommand {
    Migration(MigrationCommand),
    Restarter,
}

impl Application {
    pub async fn start() -> Result<()> {
        let cli = match Cli::try_parse() {
            Ok(cli) => cli,
            Err(err) if err.kind() == ErrorKind::MissingSubcommand => {
                start_bot().await;
                return Ok(());
            },
            Err(help) => {
                println!("{help}");
                return Ok(());
            },
        };

        match cli.subcommand {
            Subcommand::Migration(migration) => {
                let migrator = Migrator::new();
                let pgpool = set_up_database().await?;
                migration.run(&mut *pgpool.acquire().await?, migrator.into_boxed_inner()).await?;
            },
            Subcommand::Restarter => {
                let token = match env::var("DISCORD_TOKEN") {
                    Ok(token) => token,
                    Err(error) => {
                        tracing::error!("failed to fetch environment variable DISCORD_TOKEN\nError: {error:?}");
                        exit(1);
                    },
                };

                restarter::Client::start(token).await?;
            },
        }

        Ok(())
    }
}
