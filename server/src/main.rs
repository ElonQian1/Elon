use anyhow::Result;
use dotenvy::dotenv;
use std::{net::SocketAddr, sync::Arc};
use tracing::info;

mod admin;
mod agent;
mod ai_cli;
mod api;
mod client_gateway;
mod client_protocol;
mod image_generation;
mod intent_router;
mod project_api;
mod router;
mod store;
mod tools;
mod types;
mod user_api;
mod web;

pub use types::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,elon_server=debug".into()),
        )
        .init();

    let state = Arc::new(AppState::new()?);
    let app = router::build_app(state);

    let addr: SocketAddr = std::env::var("LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".into())
        .parse()?;

    info!("elon server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
