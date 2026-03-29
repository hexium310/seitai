use std::{borrow::Cow, sync::Arc};

use anyhow::{Context as _, Result};
use futures::lock::Mutex;
use serenity::{
    all::{GuildId, User},
    builder::{CreateInteractionResponse, CreateInteractionResponseMessage},
    client::Context,
    model::{application::CommandInteraction, guild::Guild},
    utils::{content_safe, ContentSafeOptions},
};
use songbird::Songbird;
use soundboard::sound::SoundId;
use voicevox::Voicevox;

use crate::{regex::{self, SOUNDMOJI}, VoicevoxClient};

pub(crate) async fn get_manager(context: &Context) -> Result<Arc<Songbird>> {
    songbird::get(context)
        .await
        .context("failed to get songbird voice client: it placed in at initialisation")
}

pub(crate) fn get_guild(context: &Context, interaction: &CommandInteraction) -> Option<Guild> {
    let guild_id = interaction.guild_id?;
    guild_id.to_guild_cached(&context.cache).map(|guild| guild.to_owned())
}

pub(crate) fn parse_soundmoji(value: impl AsRef<str>) -> Result<(Option<SoundId>, Option<GuildId>)> {
    let value = value.as_ref();

    match SOUNDMOJI.captures(value) {
        Some(caps) => {
            let sound_id = caps.name("sound_id").map(|m| m.as_str().parse::<u64>()).transpose()?;
            let guild_id = caps.name("guild_id").map(|m| m.as_str().parse::<u64>()).transpose()?;

            Ok((sound_id.map(SoundId::new), guild_id.map(GuildId::new)))
        },
        None => Ok((None, None)),
    }
}

pub(crate) async fn respond(
    context: &Context,
    interaction: &CommandInteraction,
    message: &CreateInteractionResponseMessage,
) -> Result<()> {
    let builder = CreateInteractionResponse::Message(message.clone());
    interaction
        .create_response(&context.http, builder)
        .await
        .with_context(|| format!("failed to create interaction response with message: {message:?}"))?;

    Ok(())
}

pub(crate) fn normalize<'a>(context: &Context, guild_id: &GuildId, users: &[User], text: &'a str) -> Cow<'a, str> {
    match regex::MENTION_CHANNEL.is_match(text) {
        true => {
            let content_safe_options = ContentSafeOptions::new()
                .clean_role(true)
                .clean_user(true)
                .clean_channel(true)
                .display_as_member_from(guild_id)
                .clean_here(false)
                .clean_everyone(false);

            let normalized = content_safe(&context.cache, text, &content_safe_options, users);
            Cow::Owned(normalized)
        },
        false => Cow::Borrowed(text),
    }
}

pub(crate) async fn get_voicevox(context: &Context) -> Option<Arc<Mutex<Voicevox>>> {
    let data = context.data.read().await;
    data.get::<VoicevoxClient>().cloned()
}
