use std::{fmt::Display, num::NonZeroU64, str::FromStr};

use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use serenity::all::{ChannelId, DiscordJsonError, EmojiId, GuildId, Http, User};

use crate::{client, error::SoundboardError};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct SoundId(NonZeroU64);

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct SoundboardSound {
    pub name: String,
    #[serde_as(as = "DisplayFromStr")]
    pub sound_id: SoundId,
    pub volume: f64,
    pub emoji_id: Option<EmojiId>,
    pub emoji_name: Option<String>,
    pub guild_id: Option<GuildId>,
    pub available: bool,
    pub user: Option<User>,
}

impl Display for SoundId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SoundId {
    type Err = <NonZeroU64 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        NonZeroU64::from_str(s).map(Self)
    }
}

impl From<u64> for SoundId {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl SoundId {
    pub fn new(id: u64) -> Self {
        // Panics according to the treatment of id if NonZeroU64::new() is None
        Self(NonZeroU64::new(id).expect("Attempted to call SoundId::new with invalid (0) value"))
    }

    pub fn get(&self) -> u64 {
        self.0.get()
    }

    pub async fn to_soundboard_sound(&self, http: impl AsRef<Http>, guild_id: GuildId) -> Result<SoundboardSound, SoundboardError> {
        crate::soundboard::sound(http, guild_id, *self).await
    }

    pub async fn send(&self, http: impl AsRef<Http>, channel_id: ChannelId, guild_id: Option<GuildId>) -> Result<(), SoundboardError> {
        let url = client::BASE_URL
            .join(&format!("channels/{channel_id}/send-soundboard-sound"))
            .map_err(SoundboardError::InvalidUrl)?;
        let response = client::client(http.as_ref().token())?
            .post(url)
            .json(&serde_json::json!({
                "sound_id": self.0,
                "source_guild_id": guild_id,
            }))
            .send()
            .await
            .map_err(SoundboardError::RequestError)?;

        match response.status().is_success() {
            true => Ok(()),
            false => Err(SoundboardError::UnsuccessfulRequest(
                response
                    .json::<DiscordJsonError>()
                    .await
                    .expect("failed to parse as discord json error"),
            )),
        }
    }
}

impl SoundboardSound {
    pub async fn send(&self, http: impl AsRef<Http>, channel_id: ChannelId) -> Result<(), SoundboardError> {
        self.sound_id.send(http, channel_id, self.guild_id).await
    }
}
