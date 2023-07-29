use std::{borrow::Borrow, env};

use anyhow::{Context as _, Result};
use hyper::{body::Bytes, Body, Client as HttpClient, Request};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

const DEFAULT_SPEED: f32 = 1.2;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AudioQuery {
    #[serde(rename = "accent_phrases")]
    pub(crate) accent_phrases: Value,
    pub(crate) speed_scale: f32,
    pub(crate) pitch_scale: f32,
    pub(crate) intonation_scale: f32,
    pub(crate) volume_scale: f32,
    pub(crate) pre_phoneme_length: f32,
    pub(crate) post_phoneme_length: f32,
    pub(crate) output_sampling_rate: u32,
    pub(crate) output_stereo: bool,
    pub(crate) kana: Option<String>,
}

pub(crate) async fn generate_audio_query(speaker: &str, text: &str) -> Result<AudioQuery> {
    let url = build_url("audio_query", &[("speaker", speaker), ("text", text)])?;
    let request = Request::post(&url).body(Body::empty())?;
    let http_client = HttpClient::new();
    let response = http_client.request(request).await?;
    let bytes = hyper::body::to_bytes(response.into_body()).await?;

    Ok(serde_json::from_slice(&bytes)?)
}

pub(crate) async fn synthesize(speaker: &str, json: &str) -> Result<Bytes> {
    let url = build_url("synthesis", &[("speaker", speaker)])?;
    let request = Request::post(&url)
        .header("content-type", "application/json")
        .body(Body::from(json.to_owned()))?;
    let http_client = HttpClient::new();
    let response = http_client.request(request).await?;
    let bytes = hyper::body::to_bytes(response.into_body()).await?;

    Ok(bytes)
}

pub(crate) async fn generate_audio(speaker: &str, text: &str) -> Result<Bytes> {
    let mut audio_query = generate_audio_query(speaker, text)
        .await
        .with_context(|| format!("Generating audio query with `{text}` failed"))?;
    // TODO: Truncate message too long
    audio_query.speed_scale = DEFAULT_SPEED + (text.len() / 50) as f32 * 0.1;
    let json = serde_json::to_string(&audio_query).unwrap();
    synthesize(speaker, &json)
        .await
        .with_context(|| format!("Synthesizing failed. The audio query used is {json}"))
}

fn build_url<I, K, V>(endpoint: &str, params: I) -> Result<String>
where
    I: IntoIterator,
    I::Item: Borrow<(K, V)>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    let host = env::var("VOICEVOX_HOST").expect("`VOICEVOX_HOST` is not set.");
    let url = Url::parse_with_params(&format!("http://{host}:50021/{endpoint}"), params)?;
    Ok(url.to_string())
}
