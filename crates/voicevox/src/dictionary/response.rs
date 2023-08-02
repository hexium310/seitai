use indexmap::IndexMap;
use serde::Deserialize;
use uuid::Uuid;

use crate::response::UnprocessableEntity;

pub type UserDict = IndexMap<Uuid, Item>;

#[derive(Debug, Deserialize)]
pub struct Item {
    pub accent_associative_rule: String,
    pub accent_type: u32,
    pub context_id: u32,
    pub inflectional_form: String,
    pub inflectional_type: String,
    pub mora_count: u32,
    pub part_of_speech: String,
    pub part_of_speech_detail_1: String,
    pub part_of_speech_detail_2: String,
    pub part_of_speech_detail_3: String,
    pub priority: u32,
    pub pronunciation: String,
    pub stem: String,
    pub surface: String,
}

#[derive(Debug)]
pub enum GetUserDictResult {
    Ok(UserDict),
}

#[derive(Debug)]
pub enum PostUserDictWordResult {
    Ok(Uuid),
    UnprocessableEntity(UnprocessableEntity),
}

#[derive(Debug)]
pub enum PutUserDictWordResult {
    NoContent,
    UnprocessableEntity(UnprocessableEntity),
}

#[derive(Debug)]
pub enum DeleteUserDictWordResult {
    NoContent,
    UnprocessableEntity(UnprocessableEntity),
}
