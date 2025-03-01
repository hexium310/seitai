use reqwest::header::InvalidHeaderValue;
use serenity::all::DiscordJsonError;
use url::ParseError;

#[derive(Debug, thiserror::Error)]
pub enum SoundboardError {
    #[error("invalid header value: {0}")]
    InvalidHeaderValue(#[source] InvalidHeaderValue),

    #[error("invalid url: {0}")]
    InvalidUrl(#[source] ParseError),

    #[error("failed to request: {0}")]
    RequestError(#[source] reqwest::Error),

    #[error("")]
    UnsuccessfulRequest(DiscordJsonError),
}
