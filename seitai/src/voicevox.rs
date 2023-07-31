use std::{borrow::Borrow, env};

use anyhow::{Context as _, Result};
use hyper::{body::Bytes, Body, Client as HttpClient, Request, StatusCode};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;
use uuid::Uuid;

const DEFAULT_SPEED: f32 = 1.2;

#[derive(Debug, Deserialize, Serialize)]
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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(crate) struct UnprocessableEntity {
    pub(crate) detail: String,
}

#[derive(Debug)]
pub(crate) enum PostUserDictWordResponse {
    Ok(Uuid),
    UnprocessableEntity(UnprocessableEntity),
}

#[derive(Debug)]
pub(crate) enum PutUserDictWordResponse {
    NoContent,
    UnprocessableEntity(UnprocessableEntity),
}

#[derive(Debug)]
pub(crate) enum DeleteUserDictWordResponse {
    NoContent,
    UnprocessableEntity(UnprocessableEntity),
}

pub(crate) type UserDictResponse = IndexMap<Uuid, UserDict>;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(crate) struct UserDict {
    pub(crate) accent_associative_rule: String,
    pub(crate) accent_type: u32,
    pub(crate) context_id: u32,
    pub(crate) inflectional_form: String,
    pub(crate) inflectional_type: String,
    pub(crate) mora_count: u32,
    pub(crate) part_of_speech: String,
    pub(crate) part_of_speech_detail_1: String,
    pub(crate) part_of_speech_detail_2: String,
    pub(crate) part_of_speech_detail_3: String,
    pub(crate) priority: u32,
    pub(crate) pronunciation: String,
    pub(crate) stem: String,
    pub(crate) surface: String,
}

pub(crate) async fn generate_audio_query(speaker: &str, text: &str) -> Result<AudioQuery> {
    let url = build_url_with_params("audio_query", &[("speaker", speaker), ("text", text)])?;
    let request = Request::post(&url).body(Body::empty())?;
    let http_client = HttpClient::new();
    let response = http_client.request(request).await?;
    let bytes = hyper::body::to_bytes(response.into_body()).await?;

    Ok(serde_json::from_slice(&bytes)?)
}

pub(crate) async fn synthesize(speaker: &str, json: &str) -> Result<Bytes> {
    let url = build_url_with_params("synthesis", &[("speaker", speaker)])?;
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

pub(crate) async fn get_dictionary() -> Result<UserDictResponse> {
    let url = build_url("user_dict")?;
    let request = Request::get(&url)
        .body(Body::empty())?;
    let http_client = HttpClient::new();
    let response = http_client.request(request).await?;
    let bytes = hyper::body::to_bytes(response.into_body()).await?;

    Ok(serde_json::from_slice(&bytes)?)
}

pub(crate) async fn register_word<I, K, V>(params: I) -> Result<PostUserDictWordResponse>
where
    I: IntoIterator,
    I::Item: Borrow<(K, V)>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    let url = build_url_with_params("user_dict_word", params)?;
    let request = Request::post(&url)
        .body(Body::empty())?;
    let http_client = HttpClient::new();
    let response = http_client.request(request).await?;
    let status = response.status();
    let bytes = hyper::body::to_bytes(response.into_body()).await?;
    match status {
        StatusCode::OK => Ok(PostUserDictWordResponse::Ok(Uuid::parse_str(&String::from_utf8(bytes.to_vec())?)?)),
        StatusCode::UNPROCESSABLE_ENTITY => Ok(PostUserDictWordResponse::UnprocessableEntity(serde_json::from_slice(&bytes)?)),
        _ => unreachable!(),
    }
}

pub(crate) async fn update_word<I, K, V>(uuid: &Uuid, params: I) -> Result<PutUserDictWordResponse>
where
    I: IntoIterator,
    I::Item: Borrow<(K, V)>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    let url = build_url_with_params(&format!("user_dict_word/{}", uuid), params)?;
    let request = Request::put(&url)
        .body(Body::empty())?;
    let http_client = HttpClient::new();
    let response = http_client.request(request).await?;
    let status = response.status();
    let bytes = hyper::body::to_bytes(response.into_body()).await?;
    match status {
        StatusCode::NO_CONTENT => Ok(PutUserDictWordResponse::NoContent),
        StatusCode::UNPROCESSABLE_ENTITY => Ok(PutUserDictWordResponse::UnprocessableEntity(serde_json::from_slice(&bytes)?)),
        _ => unreachable!(),
    }
}

pub(crate) async fn delete_word(uuid: &Uuid) -> Result<DeleteUserDictWordResponse> {
    let url = build_url(&format!("user_dict_word/{}", uuid))?;
    let request = Request::delete(&url)
        .body(Body::empty())?;
    let http_client = HttpClient::new();
    let response = http_client.request(request).await?;
    let status = response.status();
    let bytes = hyper::body::to_bytes(response.into_body()).await?;
    match status {
        StatusCode::NO_CONTENT => Ok(DeleteUserDictWordResponse::NoContent),
        StatusCode::UNPROCESSABLE_ENTITY => Ok(DeleteUserDictWordResponse::UnprocessableEntity(serde_json::from_slice(&bytes)?)),
        _ => unreachable!(),
    }
}

fn build_url(endpoint: &str) -> Result<String> {
    let host = env::var("VOICEVOX_HOST").expect("`VOICEVOX_HOST` is not set.");
    let url = Url::parse(&format!("http://{host}:50021/{endpoint}"))?;
    Ok(url.to_string())
}

fn build_url_with_params<I, K, V>(endpoint: &str, params: I) -> Result<String>
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
