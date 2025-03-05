use serenity::all::{ChannelId, UserId, VoiceState};

#[derive(Debug)]
pub struct VoiceStateAction {
    old_state: Option<VoiceState>,
    new_state: VoiceState,
}

pub enum VoiceStateConnection {
    // including moved
    Joined(ChannelId),
    Left,
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
        if self.old_state.is_some() {
            return VoiceStateConnection::NoAction;
        }

        match self.new_state.channel_id {
            Some(channel_id) => {
                tracing::debug!("joined voice channel {channel_id}");
                VoiceStateConnection::Joined(channel_id)
            },
            None => {
                tracing::debug!("left voice channel");
                VoiceStateConnection::Left
            },
        }
    }

    pub fn is_bot_action(&self, bot_id: UserId) -> bool {
        self.new_state.user_id == bot_id
    }
}
