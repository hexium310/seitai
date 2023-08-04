pub mod response;

use anyhow::{bail, Result};
use hyper::{Body, StatusCode};
use url::Url;
use uuid::Uuid;

use self::response::{DeleteUserDictWordResult, GetUserDictResult, PostUserDictWordResult, PutUserDictWordResult};
use crate::request::Request;

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
    pub async fn list(&self) -> Result<GetUserDictResult> {
        let (status, bytes) = self.get("user_dict", &[]).await?;
        match status {
            StatusCode::OK => Ok(GetUserDictResult::Ok(serde_json::from_slice(&bytes)?)),
            _ => bail!("error: unexpected status code"),
        }
    }

    pub async fn register_word(&self, parameters: &[(&str, &str)]) -> Result<PostUserDictWordResult> {
        let (status, bytes) = self.post("user_dict_word", parameters, Body::empty()).await?;
        match status {
            StatusCode::OK => Ok(PostUserDictWordResult::Ok(Uuid::parse_str(&String::from_utf8(
                bytes.to_vec(),
            )?)?)),
            StatusCode::UNPROCESSABLE_ENTITY => Ok(PostUserDictWordResult::UnprocessableEntity(
                serde_json::from_slice(&bytes)?,
            )),
            _ => bail!("error: unexpected status code"),
        }
    }

    pub async fn update_word(&self, uuid: &Uuid, parameters: &[(&str, &str)]) -> Result<PutUserDictWordResult> {
        let (status, bytes) = self
            .put(&format!("user_dict_word/{uuid}"), parameters, Body::empty())
            .await?;
        match status {
            StatusCode::NO_CONTENT => Ok(PutUserDictWordResult::NoContent),
            StatusCode::UNPROCESSABLE_ENTITY => Ok(PutUserDictWordResult::UnprocessableEntity(serde_json::from_slice(
                &bytes,
            )?)),
            _ => bail!("error: unexpected status code"),
        }
    }

    pub async fn delete_word(&self, uuid: &Uuid) -> Result<DeleteUserDictWordResult> {
        let (status, bytes) = self
            .delete(&format!("user_dict_word/{uuid}"), &[], Body::empty())
            .await?;
        match status {
            StatusCode::NO_CONTENT => Ok(DeleteUserDictWordResult::NoContent),
            StatusCode::UNPROCESSABLE_ENTITY => Ok(DeleteUserDictWordResult::UnprocessableEntity(
                serde_json::from_slice(&bytes)?,
            )),
            _ => bail!("error: unexpected status code"),
        }
    }
}
