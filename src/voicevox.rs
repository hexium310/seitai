use anyhow::Result;
use hyper::{body::Bytes, Body, Client as HttpClient, Request};
use url::Url;

pub(crate) async fn generate_audio_query(speaker: &str, text: &str) -> Result<String> {
    let url = Url::parse_with_params(
        "http://voicevox:50021/audio_query",
        &[("speaker", speaker), ("text", text)],
    )?;
    let request = Request::post(url.as_str()).body(Body::empty())?;
    let http_client = HttpClient::new();
    let response = http_client.request(request).await?;
    let bytes = hyper::body::to_bytes(response.into_body()).await?;
    let json = String::from_utf8(bytes.to_vec())?;

    Ok(json)
}

pub(crate) async fn generate_audio(speaker: &str, json: &str) -> Result<Bytes> {
    let url = Url::parse_with_params("http://voicevox:50021/synthesis", &[("speaker", speaker)])?;
    let request = Request::post(url.as_str())
        .header("content-type", "application/json")
        .body(Body::from(json.to_owned()))?;
    let http_client = HttpClient::new();
    let response = http_client.request(request).await?;
    let bytes = hyper::body::to_bytes(response.into_body()).await?;

    Ok(bytes)
}
