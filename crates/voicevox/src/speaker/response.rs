use serde::Deserialize;

use crate::response::UnprocessableEntity;

#[derive(Debug, Deserialize)]
pub struct Speaker {
    pub supported_features: SupportedFeatures,
    pub name: String,
    pub speaker_uuid: String,
    pub styles: Vec<Style>,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct SupportedFeatures {
    pub permitted_synthesis_morphing: String,
}

#[derive(Debug, Deserialize)]
pub struct Style {
    pub name: String,
    pub id: u32,
}

#[derive(Debug)]
pub enum GetSpeakerInfoResult {
    Ok(Speaker),
    UnprocessableEntity(UnprocessableEntity),
}

#[derive(Debug)]
pub enum GetSpeakersResult {
    Ok(Vec<Speaker>),
    UnprocessableEntity(UnprocessableEntity),
}
