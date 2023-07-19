use std::{borrow::Borrow, env};

use anyhow::Result;
use hyper::{body::Bytes, Body, Client as HttpClient, Request};
use url::Url;

pub(crate) async fn generate_audio_query(speaker: &str, text: &str) -> Result<String> {
    let url = build_url("audio_query", &[("speaker", speaker), ("text", text)])?;
    let request = Request::post(&url).body(Body::empty())?;
    let http_client = HttpClient::new();
    let response = http_client.request(request).await?;
    let bytes = hyper::body::to_bytes(response.into_body()).await?;
    let json = String::from_utf8(bytes.to_vec())?;

    Ok(json)
}

pub(crate) async fn generate_audio(speaker: &str, json: &str) -> Result<Bytes> {
    let url = build_url("synthesis", &[("speaker", speaker)])?;
    let request = Request::post(&url)
        .header("content-type", "application/json")
        .body(Body::from(json.to_owned()))?;
    let http_client = HttpClient::new();
    let response = http_client.request(request).await?;
    let bytes = hyper::body::to_bytes(response.into_body()).await?;

    Ok(bytes)
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
