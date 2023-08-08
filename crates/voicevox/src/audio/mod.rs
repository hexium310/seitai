pub mod response;

use anyhow::{bail, Context as _, Result};
use hyper::{body::Bytes, Body, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use self::response::{PostAudioQueryResult, PostSynthesisResult};
use crate::request::Request;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioQuery {
    #[serde(rename = "accent_phrases")]
    pub accent_phrases: Value,
    pub speed_scale: f32,
    pub pitch_scale: f32,
    pub intonation_scale: f32,
    pub volume_scale: f32,
    pub pre_phoneme_length: f32,
    pub post_phoneme_length: f32,
    pub output_sampling_rate: u32,
    pub output_stereo: bool,
    pub kana: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AudioGenerator {
    pub default_speed: f32,
    pub(crate) base: Url,
}

impl Request for AudioGenerator {
    fn base(&self) -> &Url {
        &self.base
    }
}

pub type Audio = Bytes;

impl AudioGenerator {
    pub async fn generate_query(&self, speaker: &str, text: &str) -> Result<PostAudioQueryResult> {
        let (status, bytes) = self
            .post("audio_query", &[("speaker", speaker), ("text", text)], Body::empty())
            .await?;
        match status {
            StatusCode::OK => Ok(PostAudioQueryResult::Ok(serde_json::from_slice(&bytes)?)),
            StatusCode::UNPROCESSABLE_ENTITY => Ok(PostAudioQueryResult::UnprocessableEntity(serde_json::from_slice(
                &bytes,
            )?)),
            code => bail!("received unexpected {code} from POST audio_query"),
        }
    }

    pub async fn synthesize(&self, speaker: &str, json: &str) -> Result<PostSynthesisResult> {
        let (status, bytes) = self
            .post("synthesis", &[("speaker", speaker)], Body::from(json.to_owned()))
            .await?;
        match status {
            StatusCode::OK => Ok(PostSynthesisResult::Ok(bytes)),
            StatusCode::UNPROCESSABLE_ENTITY => Ok(PostSynthesisResult::UnprocessableEntity(serde_json::from_slice(
                &bytes,
            )?)),
            code => bail!("received unexpected {code} from POST synthesis"),
        }
    }

    pub async fn generate(&self, speaker: &str, text: &str) -> Result<Audio> {
        let mut audio_query = match self
            .generate_query(speaker, text)
            .await
            .with_context(|| format!("failed to generate audio query with `{text}`"))?
        {
            PostAudioQueryResult::Ok(audio_query) => audio_query,
            PostAudioQueryResult::UnprocessableEntity(error) => {
                bail!(error.detail);
            },
        };

        // TODO: Truncate message too long
        audio_query.speed_scale = self.default_speed + (text.len() / 50) as f32 * 0.1;

        let json = serde_json::to_string(&audio_query)?;
        match self
            .synthesize(speaker, &json)
            .await
            .with_context(|| format!("failed to synthesize with {json}"))?
        {
            PostSynthesisResult::Ok(audio) => Ok(audio),
            PostSynthesisResult::UnprocessableEntity(error) => {
                bail!(error.detail);
            },
        }
    }
}
