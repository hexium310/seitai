use std::{fmt::Display, num::NonZeroU64, str::FromStr};

use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use serenity::all::{ChannelId, DiscordJsonError, EmojiId, GuildId, Http, User};

use crate::{client, error::SoundboardError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct SoundId(NonZeroU64);

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct SoundboardSound {
    name: String,
    #[serde_as(as = "DisplayFromStr")]
    sound_id: SoundId,
    volume: f64,
    emoji_id: Option<EmojiId>,
    emoji_name: Option<String>,
    guild_id: Option<GuildId>,
    available: bool,
    user: Option<User>,
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

impl SoundboardSound {
    pub async fn send(&self, http: impl AsRef<Http>, channel_id: ChannelId) -> Result<(), SoundboardError> {
        let url = client::BASE_URL
            .join(&format!("channels/{channel_id}/send-soundboard-sound"))
            .map_err(SoundboardError::InvalidUrl)?;
        let response = client::client(http.as_ref().token())?
            .post(url)
            .json(&serde_json::json!({
                "sound_id": self.sound_id,
                "source_guild_id": self.guild_id,
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
