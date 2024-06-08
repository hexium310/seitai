use std::{error::Error, future::Future};

use anyhow::{Context, Result};
use http_body_util::{BodyExt, Empty};
use hyper::{
    body::{Body, Bytes},
    Request as _Request,
    StatusCode,
};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
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
            let uri = format!("{}?{}", url.path(), url.query().unwrap_or_default());
            let req = _Request::get(uri)
                .header(hyper::header::HOST, url.authority())
                .body(Empty::<Bytes>::new())
                .with_context(|| format!("failed to request with GET {url}"))?;
            request(url, req).await
        }
    }

    fn post(
        &self,
        endpoint: &str,
        parameters: &[(&str, &str)],
        body: impl Body<Data = impl Send, Error = impl Into<Box<dyn Error + Send + Sync>>> + Send + Unpin + 'static,
    ) -> impl Future<Output = Result<(StatusCode, Bytes)>> + Send {
        async move {
            let url = self.url(endpoint, parameters);
            let uri = format!("{}?{}", url.path(), url.query().unwrap_or_default());
            let req = _Request::post(uri)
                .header(hyper::header::CONTENT_TYPE, "application/json")
                .header(hyper::header::HOST, url.authority())
                .body(body)
                .with_context(|| format!("failed to request with POST {url}"))?;
            request(url, req).await
        }
    }

    fn put(
        &self,
        endpoint: &str,
        parameters: &[(&str, &str)],
        body: impl Body<Data = impl Send, Error = impl Into<Box<dyn Error + Send + Sync>>> + Send + Unpin + 'static,
    ) -> impl Future<Output = Result<(StatusCode, Bytes)>> + Send {
        async move {
            let url = self.url(endpoint, parameters);
            let uri = format!("{}?{}", url.path(), url.query().unwrap_or_default());
            let req = _Request::put(uri)
                .header(hyper::header::HOST, url.authority())
                .body(body)
                .with_context(|| format!("failed to request with PUT {url}"))?;
            request(url, req).await
        }
    }

    fn delete(
        &self,
        endpoint: &str,
        parameters: &[(&str, &str)],
        body: impl Body<Data = impl Send, Error = impl Into<Box<dyn Error + Send + Sync>>> + Send + Unpin + 'static,
    ) -> impl Future<Output = Result<(StatusCode, Bytes)>> + Send {
        async move {
            let url = self.url(endpoint, parameters);
            let uri = format!("{}?{}", url.path(), url.query().unwrap_or_default());
            let req = _Request::delete(uri)
                .header(hyper::header::HOST, url.authority())
                .body(body)
                .with_context(|| format!("failed to request with DELETE {url}"))?;
            request(url, req).await
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

async fn request<RequestBody>(url: Url, request: _Request<RequestBody>) -> Result<(StatusCode, Bytes)>
where
    RequestBody: Body + Send + Unpin + 'static,
    RequestBody::Data: Send,
    RequestBody::Error: Into<Box<dyn Error + Send + Sync>>,
{
    let address = url.socket_addrs(|| None)?;
    let stream = TcpStream::connect(&*address).await?;
    let io = TokioIo::new(stream);
    let (mut sender, connection) = hyper::client::conn::http1::handshake(io).await?;

    tokio::task::spawn(async move {
        if let Err(error) = connection.await {
            tracing::error!("connection failed\nError: {error:?}");
        }
    });

    let response = sender.send_request(request).await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();

    Ok((status, bytes))
}
