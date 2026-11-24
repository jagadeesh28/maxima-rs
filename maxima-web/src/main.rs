//! HTTP backend binary entrypoint for maxima-web.
//!
//! Serves the single-page offline frontend and the JSON CAS API backed by the
//! `maxima-rs` symbolic engine.

use clap::Parser;
use maxima_web::{AppState, Cli, ServerConfig, build_app};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = ServerConfig::from_cli(&cli).map_err(|e| format!("Configuration error: {e}"))?;

    let addr = format!("{}:{}", config.bind, config.port);
    let state = Arc::new(AppState::new(config.clone()));
    let app = build_app(state);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("maxima-web service running on http://{addr}");
    println!(
        "Configuration: timeout={}ms, concurrency={}, max_body={}B",
        config.timeout_ms, config.concurrency, config.max_body_size
    );

    axum::serve(listener, app).await?;
    Ok(())
}
