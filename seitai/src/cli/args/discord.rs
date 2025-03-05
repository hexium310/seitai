use clap::Parser;

#[derive(Debug, Parser)]
pub struct DiscordConfig {
    #[arg(long, env, hide_env_values = true)]
    pub discord_token: String,
}
