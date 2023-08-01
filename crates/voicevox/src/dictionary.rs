use anyhow::Result;
use hyper::{Body, StatusCode};
use indexmap::IndexMap;
use serde::Deserialize;
use url::Url;
use uuid::Uuid;

use crate::request::Request;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct UnprocessableEntity {
    pub detail: String,
}

#[derive(Debug)]
pub enum PostUserDictWordResponse {
    Ok(Uuid),
    UnprocessableEntity(UnprocessableEntity),
}

#[derive(Debug)]
pub enum PutUserDictWordResponse {
    NoContent,
    UnprocessableEntity(UnprocessableEntity),
}

#[derive(Debug)]
pub enum DeleteUserDictWordResponse {
    NoContent,
    UnprocessableEntity(UnprocessableEntity),
}

pub type UserDictResponse = IndexMap<Uuid, UserDict>;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct UserDict {
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

#[derive(Debug, Clone)]
pub struct Dictionary {
    pub(crate) base: Url,
}

impl Request for Dictionary {
    fn base(&self) -> Url {
        self.base.clone()
    }
}

impl Dictionary {
    pub async fn list(&self) -> Result<UserDictResponse> {
        let (_status, bytes) = self.get("user_dict", &[]).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub async fn register_word(&self, parameters: &[(&str, &str)]) -> Result<PostUserDictWordResponse> {
        let (status, bytes) = self.post("user_dict_word", parameters, Body::empty()).await?;
        match status {
            StatusCode::OK => Ok(PostUserDictWordResponse::Ok(Uuid::parse_str(&String::from_utf8(
                bytes.to_vec(),
            )?)?)),
            StatusCode::UNPROCESSABLE_ENTITY => Ok(PostUserDictWordResponse::UnprocessableEntity(
                serde_json::from_slice(&bytes)?,
            )),
            _ => unreachable!(),
        }
    }

    pub async fn update_word(&self, uuid: &Uuid, parameters: &[(&str, &str)]) -> Result<PutUserDictWordResponse> {
        let (status, bytes) = self
            .put(&format!("user_dict_word/{uuid}"), parameters, Body::empty())
            .await?;
        match status {
            StatusCode::NO_CONTENT => Ok(PutUserDictWordResponse::NoContent),
            StatusCode::UNPROCESSABLE_ENTITY => Ok(PutUserDictWordResponse::UnprocessableEntity(
                serde_json::from_slice(&bytes)?,
            )),
            _ => unreachable!(),
        }
    }

    pub async fn delete_word(&self, uuid: &Uuid) -> Result<DeleteUserDictWordResponse> {
        let (status, bytes) = self
            .delete(&format!("user_dict_word/{uuid}"), &[], Body::empty())
            .await?;
        match status {
            StatusCode::NO_CONTENT => Ok(DeleteUserDictWordResponse::NoContent),
            StatusCode::UNPROCESSABLE_ENTITY => Ok(DeleteUserDictWordResponse::UnprocessableEntity(
                serde_json::from_slice(&bytes)?,
            )),
            _ => unreachable!(),
        }
    }
}
