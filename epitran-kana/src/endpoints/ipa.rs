#![allow(dead_code)]
use anyhow::{Result, Error};
use axum::extract::Path;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use pyo3::marker::Python;
use serde::{Deserialize, Serialize};
use serde_json::json;

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
    let pronunciation = transliterate(&word).map_err(IpaError::EpitranFailed)?;

    let response = Ipa {
        word,
        pronunciation,
    };

    Ok((StatusCode::OK, Json(response)))
}

pub(crate) fn transliterate(word: &str) -> Result<String> {
    Python::with_gil(|py| {
        let epitran = py.import("epitran")?.getattr("Epitran")?;
        let transliterator = epitran.call1(("eng-Latn", ))?;
        let ipa = transliterator.getattr("transliterate")?.call1((word, ))?.extract::<String>().map_err(Error::msg)?;
        Ok(ipa.to_string())
    })
}
