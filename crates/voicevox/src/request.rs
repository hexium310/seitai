use std::future::Future;

use anyhow::{Context, Result};
use hyper::{body::Bytes, Body, Client as HttpClient, Request as _Request, StatusCode};
use url::Url;

pub trait Request: Send + Sync {
    fn base(&self) -> &Url;

    fn get(
        &self,
        endpoint: &str,
        parameters: &[(&str, &str)],
    ) -> impl Future<Output = Result<(StatusCode, Bytes)>> + Send {
        async move {
            let url = self.url(endpoint, parameters);
            let req = _Request::get(url.as_str())
                .body(Body::empty())
                .with_context(|| format!("failed to request with GET {url}"))?;
            request(req).await
        }
    }

    fn post(
        &self,
        endpoint: &str,
        parameters: &[(&str, &str)],
        body: Body,
    ) -> impl std::future::Future<Output = Result<(StatusCode, Bytes)>> + Send {
        async move {
            let url = self.url(endpoint, parameters);
            let req = _Request::post(url.as_str())
                .header("content-type", "application/json")
                .body(body)
                .with_context(|| format!("failed to request with POST {url}"))?;
            request(req).await
        }
    }

    fn put(
        &self,
        endpoint: &str,
        parameters: &[(&str, &str)],
        body: Body,
    ) -> impl std::future::Future<Output = Result<(StatusCode, Bytes)>> + Send {
        async move {
            let url = self.url(endpoint, parameters);
            let req = _Request::put(url.as_str())
                .body(body)
                .with_context(|| format!("failed to request with PUT {url}"))?;
            request(req).await
        }
    }

    fn delete(
        &self,
        endpoint: &str,
        parameters: &[(&str, &str)],
        body: Body,
    ) -> impl std::future::Future<Output = Result<(StatusCode, Bytes)>> + Send {
        async move {
            let url = self.url(endpoint, parameters);
            let req = _Request::delete(url.as_str())
                .body(body)
                .with_context(|| format!("failed to request with DELETE {url}"))?;
            request(req).await
        }
    }

    fn url(&self, endpoint: &str, parameters: &[(&str, &str)]) -> Url {
        let mut url = self.base().clone();
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
