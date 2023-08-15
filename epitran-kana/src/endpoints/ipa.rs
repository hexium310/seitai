use anyhow::Error;
use axum::extract::Path;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::transliterator::ipa;

#[derive(Deserialize)]
pub(crate) struct GetParams {
    word: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Ipa {
    pub(crate) word: String,
    pub(crate) pronunciation: String,
}

pub(crate) enum IpaError {
    EpitranFailed(Error),
}

impl IntoResponse for IpaError {
    fn into_response(self) -> Response {
        let detail = match self {
            IpaError::EpitranFailed(error) => format!("{error:?}"),
        };
        let json = json!({
            "message": "failed to transliterate",
            "detail": detail
        });

        (StatusCode::INTERNAL_SERVER_ERROR, Json(json)).into_response()
    }
}

pub(crate) async fn get(Path(GetParams { word }): Path<GetParams>) -> std::result::Result<impl IntoResponse, IpaError> {
    let pronunciation = ipa::transliterate(&word).map_err(IpaError::EpitranFailed)?;

    let response = Ipa {
        word,
        pronunciation,
    };

    Ok((StatusCode::OK, Json(response)))
}
