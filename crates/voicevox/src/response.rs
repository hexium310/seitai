use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UnprocessableEntity {
    pub detail: String,
}
