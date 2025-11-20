use anyhow::{Context as _, Result};
use serenity::all::{Context, CreateCommand, Ready};
use songbird::input::Input;

use crate::{audio::AudioRepository, commands, event_handler::Handler};

#[allow(unused)]
struct ReadyHandler<'a, Repository> {
    event_handler: &'a Handler<Repository>,
    context: Context,
    ready: Ready,
}

impl<'a, Repository> ReadyHandler<'a, Repository>
where
    Repository: AudioRepository<Input = Input> + Send + Sync,
{
    fn new(event_handler: &'a Handler<Repository>, context: Context, ready: Ready) -> Self {
        Self { event_handler, context, ready }
    }

    async fn set_commands(&self, commands: &[CreateCommand]) -> Result<()> {
        for guild in &self.ready.guilds {
            guild
                .id
                .set_commands(&self.context.http, commands.to_vec())
            .await
            .context("failed to register slash commands")?;
        }

        Ok(())
    }

    async fn handle(&self) -> Result<()> {
        tracing::info!("{} is ready", self.ready.user.name);

        let commands = &[
            commands::dictionary::register(),
            commands::help::register(),
            commands::join::register(),
            commands::leave::register(),
            commands::voice::register(),
            commands::soundsticker::register(),
        ];

        self.set_commands(commands).await?;

        Ok(())
    }
}

pub(crate) async fn handle<Repository>(event_handler: &Handler<Repository>, context: Context, ready: Ready) -> Result<()>
where
    Repository: AudioRepository<Input = Input> + Send + Sync,
{
    let handler = ReadyHandler::new(event_handler, context, ready);
    handler.handle().await
}
