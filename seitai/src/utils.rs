use std::{collections::HashMap, sync::Arc};

use anyhow::{Context as _, Result};
use serenity::{
    builder::{CreateInteractionResponse, CreateInteractionResponseMessage},
    client::Context,
    futures::lock::Mutex,
    model::{application::CommandInteraction, guild::Guild},
};
use songbird::{
    input::{cached::Compressed, Input},
    Songbird,
};

use crate::SoundStore;

pub(crate) async fn get_manager(context: &Context) -> Result<Arc<Songbird>> {
    songbird::get(context)
        .await
        .context("Songbird Voice client placed in at initialisation.")
}

pub(crate) fn get_guild(context: &Context, interaction: &CommandInteraction) -> Option<Guild> {
    let guild_id = interaction.guild_id.unwrap();
    guild_id.to_guild_cached(&context.cache).map(|guild| guild.to_owned())
}

pub(crate) async fn respond(
    context: &Context,
    interaction: &CommandInteraction,
    message: CreateInteractionResponseMessage,
) -> Result<()> {
    let builder = CreateInteractionResponse::Message(message);
    interaction.create_response(&context.http, builder).await?;

    Ok(())
}

pub(crate) async fn get_sound_store(context: &Context) -> Arc<Mutex<HashMap<String, Compressed>>> {
    let data = context.data.read().await;
    data.get::<SoundStore>().unwrap().clone()
}

pub(crate) async fn get_cached_audio(context: &Context, key: &str) -> Option<Input> {
    let sound_store = get_sound_store(context).await;
    let sound_store = sound_store.lock().await;
    sound_store.get(key).map(|source| source.new_handle().into())
}
