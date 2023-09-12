use anyhow::{Context as _, Result};
use ordered_float::NotNan;
use serenity::{
    builder::{CreateCommand, CreateEmbed, CreateInteractionResponseMessage},
    client::Context,
    model::{application::CommandInteraction, Colour},
};
use songbird::input::Input;

use crate::{
    audio::{cache::CacheTarget, Audio, AudioRepository},
    speaker::Speaker,
    utils::{get_guild, get_manager, respond},
};

pub(crate) async fn run<R>(context: &Context, audio_repository: &R, interaction: &CommandInteraction) -> Result<()>
where
    R: AudioRepository<Input = Input> + Send + Sync,
{
    let guild = match get_guild(context, interaction) {
        Some(guild) => guild,
        None => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .description("このコマンドは使えません。")
                    .colour(Colour::RED),
            );
            respond(context, interaction, &message).await?;
            return Ok(());
        },
    };
    let channel_id = guild
        .voice_states
        .get(&interaction.user.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .description("接続先のボイスチャンネルが見つかりません。")
                    .colour(Colour::RED),
            );
            respond(context, interaction, &message).await?;
            return Ok(());
        },
    };

    let manager = get_manager(context).await?;
    let call = manager.get_or_insert(guild.id);
    let join = {
        let mut call = call.lock().await;
        call.deafen(true).await?;
        call.join(connect_to).await?
    };
    join.await?;

    let message = CreateInteractionResponseMessage::new().embed(
        CreateEmbed::new()
            .description("ボイスチャンネルに接続しました。")
            .colour(Colour::FOOYOO),
    );
    respond(context, interaction, &message).await?;

    {
        let mut call = call.lock().await;

        let audio = Audio {
            text: CacheTarget::Connected.as_ref().to_string(),
            speaker: "1".to_string(),
            speed: NotNan::new(Speaker::default_speed()).unwrap(),
        };
        let input = audio_repository
            .get(audio)
            .await
            .context("failed to get audio source")?;
        call.enqueue_input(input).await;
    }
    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("join").description("ボイスチャンネルに接続します。")
}
