use std::{error::Error, future::Future};

use anyhow::{Context, Result};
use http_body_util::{BodyExt, Empty};
use hyper::{
    body::{Body, Bytes},
    Request as _Request,
    StatusCode,
};
use hyper_util::{client::legacy::Client as HttpClient, rt::TokioExecutor};
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
                .body(Empty::<Bytes>::new())
                .with_context(|| format!("failed to request with GET {url}"))?;
            request(req).await
        }
    }

    fn post(
        &self,
        endpoint: &str,
        parameters: &[(&str, &str)],
        body: impl Body<Data = impl Send, Error = impl Into<Box<dyn Error + Send + Sync>>> + Send + Unpin + 'static,
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
        body: impl Body<Data = impl Send, Error = impl Into<Box<dyn Error + Send + Sync>>> + Send + Unpin + 'static,
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
        body: impl Body<Data = impl Send, Error = impl Into<Box<dyn Error + Send + Sync>>> + Send + Unpin + 'static,
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

async fn request(
    request: _Request<
        impl Body<Data = impl Send, Error = impl Into<Box<dyn Error + Send + Sync>>> + Send + Unpin + 'static,
    >,
) -> Result<(StatusCode, Bytes)> {
    let http_client = HttpClient::builder(TokioExecutor::new()).build_http();
    let response = http_client.request(request).await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();

    Ok((status, bytes))
}
