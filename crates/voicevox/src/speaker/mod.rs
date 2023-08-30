pub mod response;

use anyhow::{bail, Result};
use hyper::StatusCode;
use url::Url;
use uuid::Uuid;

use self::response::{GetSpeakersResult, GetSpeakerInfoResult};
use crate::request::Request;

#[derive(Debug, Clone)]
pub struct Speaker {
    pub(crate) base: Url,
}

impl Request for Speaker {
    fn base(&self) -> &Url {
        &self.base
    }
}

impl Speaker {
    pub async fn list(&self) -> Result<GetSpeakersResult> {
        let (status, bytes) = self.get("speakers", &[]).await?;
        match status {
            StatusCode::OK => Ok(GetSpeakersResult::Ok(serde_json::from_slice(&bytes)?)),
            StatusCode::UNPROCESSABLE_ENTITY => Ok(GetSpeakersResult::UnprocessableEntity(
                serde_json::from_slice(&bytes)?,
            )),
            code => bail!("received unexpected {code} from GET speakers"),
        }
    }

    pub async fn get_info(&self, uuid: &Uuid) -> Result<GetSpeakerInfoResult> {
        let (status, bytes) = self.get("speakers", &[("speaker_uuid", &uuid.to_string())]).await?;
        match status {
            StatusCode::OK => Ok(GetSpeakerInfoResult::Ok(serde_json::from_slice(&bytes)?)),
            StatusCode::UNPROCESSABLE_ENTITY => Ok(GetSpeakerInfoResult::UnprocessableEntity(
                serde_json::from_slice(&bytes)?,
            )),
            code => bail!("received unexpected {code} from GET speakers"),
        }
    }
}
