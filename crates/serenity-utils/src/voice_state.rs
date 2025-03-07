use serenity::all::{ChannelId, UserId, VoiceState};

#[derive(Debug)]
pub struct VoiceStateAction {
    old_state: Option<VoiceState>,
    new_state: VoiceState,
}

pub enum VoiceStateConnection {
    Joined(ChannelId),
    Left(ChannelId),
    Moved(ChannelId, ChannelId),
    NoAction,
}

impl VoiceStateAction {
    pub fn new(old_state: Option<VoiceState>, new_state: VoiceState) -> Self {
        Self {
            old_state,
            new_state,
        }
    }

    pub fn connection(&self) -> VoiceStateConnection {
        match &self.old_state {
            Some(old_state) => match (old_state.channel_id, self.new_state.channel_id) {
                (Some(old_channel_id), Some(new_channel_id)) if old_channel_id != new_channel_id => {
                    tracing::debug!("moved voice channel from {old_channel_id} to {new_channel_id}");
                    VoiceStateConnection::Moved(old_channel_id, new_channel_id)
                },
                (Some(channel_id), None) => {
                    tracing::debug!("left voice channel from {channel_id}");
                    VoiceStateConnection::Left(channel_id)
                },
                _ => VoiceStateConnection::NoAction,
            },
            None => match self.new_state.channel_id {
                Some(channel_id) => {
                    tracing::debug!("joined voice channel to {channel_id}");
                    VoiceStateConnection::Joined(channel_id)
                },
                None => VoiceStateConnection::NoAction,
            },
        }
    }

    pub fn is_bot_action(&self, bot_id: UserId) -> bool {
        self.new_state.user_id == bot_id
    }
}
