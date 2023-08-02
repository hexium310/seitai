use hyper::body::Bytes;

use super::AudioQuery;
use crate::response::UnprocessableEntity;

pub type Audio = Bytes;

#[derive(Debug)]
pub enum PostAudioQueryResult {
    Ok(AudioQuery),
    UnprocessableEntity(UnprocessableEntity),
}

#[derive(Debug)]
pub enum PostSynthesisResult {
    Ok(Audio),
    UnprocessableEntity(UnprocessableEntity),
}
