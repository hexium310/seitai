use axum::{extract::Path, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::transliterator::{ipa, kana};

#[derive(Deserialize)]
pub(crate) struct GetParams {
    word: String,
}

#[derive(Serialize)]
pub(crate) struct Katakana {
    word: String,
    pronunciation_kana: String,
    pronunciation: String,
}

pub(crate) async fn get(Path(GetParams { word }): Path<GetParams>) -> impl IntoResponse {
    let pronunciation = ipa::transliterate(&word).unwrap();
    let pronunciation_kana = kana::transliterate(&pronunciation).unwrap();

    let response = Katakana {
        word,
        pronunciation,
        pronunciation_kana,
    };

    (StatusCode::OK, Json(response))
}
