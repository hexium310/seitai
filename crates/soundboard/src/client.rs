use std::sync::LazyLock;

use reqwest::{
    Client,
    Url,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use serenity::constants;

use crate::error::SoundboardError;

pub static BASE_URL: LazyLock<Url> = LazyLock::new(|| Url::parse("https://discord.com/api/v10").expect("base url parse error"));

pub fn client(token: &str) -> Result<Client, SoundboardError> {
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(token).map_err(SoundboardError::InvalidHeaderValue)?);

    Ok(Client::builder()
        .user_agent(constants::USER_AGENT)
        .default_headers(headers)
        .build()
        .expect("default client build error"))
}
