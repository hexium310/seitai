use anyhow::Result;
use clap::Parser;

use crate::cli::args::discord::DiscordConfig;

#[derive(Debug, Parser)]
pub struct Restarter {
    #[command(flatten)]
    pub discord_args: DiscordConfig,
}

impl Restarter {
    pub async fn run(&self) -> Result<()> {
        let token = self.discord_args.discord_token.clone();

        restarter::Client::start(token).await?;

        Ok(())
    }
}
