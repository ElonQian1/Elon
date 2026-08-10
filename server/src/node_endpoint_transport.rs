//! Optional direct-TLS ingress for future authoritative node endpoint sessions.
//!
//! The current secure route is deliberately compute-inert. It can seal evidence from a real
//! rustls handshake, but it never upgrades a WebSocket or calls the endpoint credential Store.

use std::net::SocketAddr;

use anyhow::Result;
use axum::Router;
use tokio::net::TcpListener;
use tracing::info;

mod config;
mod direct_tls;
mod evidence_slot;
mod secure_router;

pub(crate) use direct_tls::DirectTlsVerifierSeal;

pub(crate) async fn serve(legacy_app: Router) -> Result<()> {
    let legacy_addr: SocketAddr = std::env::var("LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".into())
        .parse()?;
    let direct_tls_config = config::DirectTlsTransportConfig::from_env(legacy_addr)?;

    // Bind every enabled ingress before serving either one. A bad secure binding must not leave a
    // partially started process silently serving only the legacy listener.
    let direct_tls = match direct_tls_config {
        Some(config) => Some(direct_tls::DirectTlsServer::bind(config).await?),
        None => None,
    };
    let legacy_listener = TcpListener::bind(legacy_addr).await?;
    info!(%legacy_addr, "elon legacy HTTP server listening");

    match direct_tls {
        Some(server) => {
            tokio::try_join!(serve_legacy(legacy_listener, legacy_app), server.serve())?;
            Ok(())
        }
        None => serve_legacy(legacy_listener, legacy_app).await,
    }
}

async fn serve_legacy(listener: TcpListener, app: Router) -> Result<()> {
    axum::serve(listener, app).await?;
    Ok(())
}
