use anyhow::Result;
use clap::{error::ErrorKind, Parser};
use subcommands::Subcommand;

use crate::start_bot;

mod args;
mod subcommands;

pub struct Application;

#[derive(clap::Parser)]
#[command(arg_required_else_help = false)]
#[command(subcommand_required = false)]
pub struct Cli {
    #[command(subcommand)]
    subcommand: Subcommand,
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
                help.print()?;
                return Ok(());
            },
        };

        match cli.subcommand {
            Subcommand::Migration(migration) => migration.run().await?,
            Subcommand::Restarter(restarter) => restarter.run().await?,
        }

        Ok(())
    }
}
