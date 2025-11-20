use anyhow::{Context as _, Result};
use serenity::all::{CommandInteraction, Context, Interaction};
use songbird::input::Input;

use crate::{audio::AudioRepository, commands, event_handler::Handler};

struct InteractionCreateHandler<'a, Repository> {
    event_handler: &'a Handler<Repository>,
    context: Context,
    interaction: Interaction,
}

impl<'a, Repository> InteractionCreateHandler<'a, Repository>
where
    Repository: AudioRepository<Input = Input> + Send + Sync,
{
    fn new(event_handler: &'a Handler<Repository>, context: Context, interaction: Interaction) -> Self {
        Self { event_handler, context, interaction }
    }

    async fn handle_command(&self, command: &CommandInteraction) -> Result<()> {
         match command.data.name.as_str() {
            "dictionary" => commands::dictionary::run(&self.context, &self.event_handler.audio_repository, command).await,
            "help" => commands::help::run(&self.context, command).await,
            "join" => {
                commands::join::run(&self.context, &self.event_handler.audio_repository, &mut *self.event_handler.connections.lock().await, command).await
            },
            "leave" => commands::leave::run(&self.context, command).await,
            "voice" => commands::voice::run(&self.context, command, &self.event_handler.database, &self.event_handler.speaker).await,
            "soundsticker" => commands::soundsticker::run(&self.context, command, &self.event_handler.database).await,
            _ => Ok(()),
        }
            .with_context(|| format!("failed to execute /{}", command.data.name))
    }

    async fn handle_autocomplete(&self, command: &CommandInteraction) -> Result<()> {
        match command.data.name.as_str() {
            "voice" => commands::voice::autocomplete(&self.context, command, &self.event_handler.speaker).await,
            "soundsticker" => commands::soundsticker::autocomplete(&self.context, command).await,
            _ => Ok(()),
        }
            .with_context(|| format!("failed to autocomplete /{}", command.data.name))
    }

    async fn handle(&self) -> Result<()> {
        match &self.interaction {
            Interaction::Command(command) => self.handle_command(command).await?,
            Interaction::Autocomplete(command) => self.handle_autocomplete(command).await?,
            _ => {},
        }

        Ok(())
    }
}

pub(crate) async fn handle<Repository>(event_handler: &Handler<Repository>, context: Context, interaction: Interaction) -> Result<()>
where
    Repository: AudioRepository<Input = Input> + Send + Sync,
{
    let handler = InteractionCreateHandler::new(event_handler, context, interaction);
    handler.handle().await
}
