use anyhow::Result;
use serenity::all::{Context, VoiceState};
use serenity_utils::voice_state::{VoiceStateAction, VoiceStateConnection};

use super::Handler;

pub(crate) async fn handle(handler: &Handler, ctx: Context, old_state: Option<VoiceState>, new_state: VoiceState) -> Result<()> {
    let Some(guild_id) = new_state.guild_id else {
        return Ok(());
    };

    let bot_id = ctx.http.get_current_user().await?.id;
    let action = VoiceStateAction::new(old_state, new_state);

    if !action.is_bot_action(bot_id) {
        return Ok(());
    }

    match action.connection() {
        VoiceStateConnection::Joined(channel_id) => {
            let mut connected_channels = handler.connected_channels.lock().await;
            connected_channels.insert(guild_id, channel_id);

            handler.restarter.send(connected_channels.len()).await?;
        },
        VoiceStateConnection::Left(_) => {
            let mut connected_channels = handler.connected_channels.lock().await;
            connected_channels.remove(&guild_id);

            handler.restarter.send(connected_channels.len()).await?;

        },
        VoiceStateConnection::Moved(..) | VoiceStateConnection::NoAction => (),
    }

    Ok(())
}
