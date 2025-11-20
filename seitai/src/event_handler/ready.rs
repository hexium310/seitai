use serenity::all::{Context, Ready};
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

    async fn handle(&self) {
        for guild in &self.ready.guilds {
            let commands = guild
                .id
                .set_commands(
                    &self.context.http,
                    vec![
                        commands::dictionary::register(),
                        commands::help::register(),
                        commands::join::register(),
                        commands::leave::register(),
                        commands::voice::register(),
                        commands::soundsticker::register(),
                    ],
                )
            .await;

            if let Err(error) = commands {
                tracing::error!("failed to regeister slash commands\nError: {error:?}");
            }
        }
    }
}

pub(crate) async fn handle<Repository>(event_handler: &Handler<Repository>, context: Context, ready: Ready)
where
    Repository: AudioRepository<Input = Input> + Send + Sync,
{
    let handler = ReadyHandler::new(event_handler, context, ready);
    handler.handle().await;
}
