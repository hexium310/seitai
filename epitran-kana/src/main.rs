use axum::http::{Request, Response};
use axum::{
    routing::get, Router,
};
use logging::initialize_logging;
use tokio::signal::unix::SignalKind;
use tokio::signal::unix::signal;
use tower_http::trace::{TraceLayer, self};
use std::net::SocketAddr;
use std::process::exit;
use std::time::Duration;
use tracing::{Span, Level};

use crate::endpoints::{ipa, kana};

mod endpoints;
mod transliterator;

#[tokio::main]
async fn main() {
    initialize_logging();

    let trace_layer = TraceLayer::new_for_http()
        .on_request(|request: &Request<_>, _span: &Span| {
            tracing::info!("request: {} {}", request.method(), request.uri())
        })
        .on_response(|response: &Response<_>, latency: Duration, _span: &Span| {
            tracing::info!("response: {} in {latency:?}", response.status())
        })
        .on_failure(trace::DefaultOnFailure::new().level(Level::ERROR));
    let app = Router::new()
        .route("/ipa/:word", get(ipa::get))
        .route("/kana/:word", get(kana::get))
        .layer(trace_layer);

    let listener = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {listener:?}");

    tokio::spawn(async move {
        axum::Server::bind(&listener)
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();

    tokio::select! {
        _ = sigint.recv() => {
            tracing::info!("received SIGINT, shutting down");
            exit(130);
        },
        _ = sigterm.recv() => {
            tracing::info!("received SIGTERM, shutting down");
            exit(143);
        },
    }
}
