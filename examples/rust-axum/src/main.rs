use std::{env, net::SocketAddr};

use axum::{Router, http::StatusCode, response::Html, routing::get};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_owned());
    let address = SocketAddr::from(([0, 0, 0, 0], port.parse().expect("PORT must be a number")));

    let app = Router::new()
        .route(
            "/",
            get(|| async { Html("<!doctype html><html><body><h1>Hello World</h1></body></html>") }),
        )
        .route("/healthz", get(|| async { StatusCode::OK }));

    let listener = TcpListener::bind(address)
        .await
        .expect("failed to bind HTTP listener");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("HTTP server failed");
}

async fn shutdown_signal() {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("failed to install SIGTERM handler");

    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            result.expect("failed to listen for Ctrl+C");
        }
        _ = terminate.recv() => {}
    }
}
