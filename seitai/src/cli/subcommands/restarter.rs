use anyhow::Result;
use clap::Parser;

use crate::cli::args::discord::DiscordConfig;

#[derive(Debug, Parser)]
#[command(about = "Launches client to restart voicevox statefulset")]
pub struct Restarter {
    #[command(flatten)]
    pub discord_args: DiscordConfig,

    #[arg(long, default_value_t = 300, help = "secs until this client restarts voicevox after leaving from voice channel")]
    pub duration: u64,
}

impl Restarter {
    pub async fn run(&self) -> Result<()> {
        let token = self.discord_args.discord_token.clone();

        restarter::Client::start(token, self.duration).await?;

        Ok(())
    }
}
