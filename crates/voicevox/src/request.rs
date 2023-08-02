use anyhow::Result;
use async_trait::async_trait;
use hyper::{body::Bytes, Body, Client as HttpClient, Request as _Request, StatusCode};
use url::Url;

#[async_trait]
pub trait Request: Send + Sync {
    fn base(&self) -> Url;

    async fn get(&self, endpoint: &str, parameters: &[(&str, &str)]) -> Result<(StatusCode, Bytes)> {
        let url = self.url(endpoint, parameters);
        let req = _Request::get(url.as_str()).body(Body::empty())?;
        request(req).await
    }

    async fn post(&self, endpoint: &str, parameters: &[(&str, &str)], body: Body) -> Result<(StatusCode, Bytes)> {
        let url = self.url(endpoint, parameters);
        let req = _Request::post(url.as_str())
            .header("content-type", "application/json")
            .body(body)?;
        request(req).await
    }

    async fn put(&self, endpoint: &str, parameters: &[(&str, &str)], body: Body) -> Result<(StatusCode, Bytes)> {
        let url = self.url(endpoint, parameters);
        let req = _Request::put(url.as_str()).body(body)?;
        request(req).await
    }

    async fn delete(&self, endpoint: &str, parameters: &[(&str, &str)], body: Body) -> Result<(StatusCode, Bytes)> {
        let url = self.url(endpoint, parameters);
        let req = _Request::delete(url.as_str()).body(body)?;
        request(req).await
    }

    fn url(&self, endpoint: &str, parameters: &[(&str, &str)]) -> Url {
        let mut url = self.base();
        url.set_path(endpoint);
        if !parameters.is_empty() {
            url.query_pairs_mut().extend_pairs(parameters);
        }
        url
    }
}

async fn request(request: _Request<Body>) -> Result<(StatusCode, Bytes)> {
    let http_client = HttpClient::new();
    let response = http_client.request(request).await?;
    let status = response.status();
    let bytes = hyper::body::to_bytes(response.into_body()).await?;

    Ok((status, bytes))
}
