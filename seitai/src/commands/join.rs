use anyhow::Result;
use serenity::{
    builder::{CreateCommand, CreateEmbed, CreateInteractionResponseMessage},
    client::Context,
    model::{application::CommandInteraction, Colour},
};

use crate::utils::{get_guild, get_manager, respond, get_cached_audio};

pub(crate) async fn run(context: &Context, interaction: &CommandInteraction) -> Result<()> {
    let guild = match get_guild(context, interaction) {
        Some(guild) => guild,
        None => {
            let message = CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .description("このコマンドは使えません。")
                    .colour(Colour::RED),
            );
            respond(context, interaction, message).await?;
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
            respond(context, interaction, message).await?;
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
    if let Err(why) = join.await {
        eprintln!("err: {why}");
    }

    {
        let mut call = call.lock().await;
        let audio = get_cached_audio(context, "connected").await.unwrap();
        call.enqueue_input(audio).await;
    }

    let message = CreateInteractionResponseMessage::new().embed(
        CreateEmbed::new()
            .description("ボイスチャンネルに接続しました。")
            .colour(Colour::FOOYOO),
    );
    respond(context, interaction, message).await?;
    Ok(())
}

pub fn register() -> CreateCommand {
    CreateCommand::new("join").description("ボイスチャンネルに接続します。")
}
