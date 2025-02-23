use serde::{Deserialize, Serialize};
use serenity::all::{DiscordJsonError, GuildId, Http};

use crate::{
    client,
    error::SoundboardError,
    sound::{SoundId, SoundboardSound},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct Soundboard {
    pub items: Vec<SoundboardSound>,
}

pub async fn default_soundboards(http: impl AsRef<Http>) -> Result<Vec<SoundboardSound>, SoundboardError> {
    let url = client::BASE_URL
        .join("soundboard-default-sounds")
        .map_err(SoundboardError::InvalidUrl)?;
    let response = client::client(http.as_ref().token())?
        .get(url)
        .send()
        .await
        .map_err(SoundboardError::RequestError)?;

    match response.status().is_success() {
        true => Ok(response
            .json::<Vec<SoundboardSound>>()
            .await
            .expect("failed to parse response as soundboard sound vec")),
        false => Err(SoundboardError::UnsuccessfulRequest(
            response
                .json::<DiscordJsonError>()
                .await
                .expect("failed to parse error response as discord json error"),
        )),
    }
}

pub async fn soundboards(http: impl AsRef<Http>, guild_id: GuildId) -> Result<Soundboard, SoundboardError> {
    let url = client::BASE_URL
        .join(&format!("guilds/{guild_id}/soundboard-sounds"))
        .map_err(SoundboardError::InvalidUrl)?;
    let response = client::client(http.as_ref().token())?
        .get(url)
        .send()
        .await
        .map_err(SoundboardError::RequestError)?;

    match response.status().is_success() {
        true => Ok(response
            .json::<Soundboard>()
            .await
            .expect("failed to parse response as soundboard")),
        false => Err(SoundboardError::UnsuccessfulRequest(
            response
                .json::<DiscordJsonError>()
                .await
                .expect("failed to parse error response as discord json error"),
        )),
    }
}

pub async fn sound(http: impl AsRef<Http>, guild_id: GuildId, sound_id: SoundId) -> Result<SoundboardSound, SoundboardError> {
    let url = client::BASE_URL
        .join(&format!("guilds/{guild_id}/soundboard-sounds/{sound_id}"))
        .map_err(SoundboardError::InvalidUrl)?;
    let response = client::client(http.as_ref().token())?
        .get(url)
        .send()
        .await
        .map_err(SoundboardError::RequestError)?;

    match response.status().is_success() {
        true => Ok(response
            .json::<SoundboardSound>()
            .await
            .expect("failed to parse response as soundboard sound")),
        false => Err(SoundboardError::UnsuccessfulRequest(
            response
                .json::<DiscordJsonError>()
                .await
                .expect("failed to parse error response as discord json error"),
        )),
    }
}
