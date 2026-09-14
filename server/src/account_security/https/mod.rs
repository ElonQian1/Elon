//! Optional, account-only native TLS listener. Legacy and node ingress remain independent.
use std::sync::Arc;

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};

use crate::types::AppState;

mod config;
mod policy;
mod quant_public;
mod transport;

pub(crate) async fn serve(legacy_app: Router, state: Arc<AppState>) -> Result<()> {
    let config = config::Config::from_env()?;
    let public_quant = quant_public::enabled(
        std::env::var("QUANT_PUBLIC_HTTPS_ENABLED").ok().as_deref(),
        config.is_some(),
    )?;
    let Some(config) = config else {
        return crate::node_endpoint_transport::serve(legacy_app, state).await;
    };
    // Fail before starting legacy ingress if explicitly enabled TLS cannot bind.
    let server = transport::Server::bind(config).await?;
    let app = routes(state.clone(), public_quant);
    tokio::try_join!(
        crate::node_endpoint_transport::serve(legacy_app, state),
        server.serve(app),
    )?;
    Ok(())
}

/// Share the exact TLS route assembly with integration tests, including ingress policy.
pub(crate) fn routes(state: Arc<AppState>, public_quant: bool) -> Router {
    let app = Router::new()
        .route(
            "/health",
            get(|| async {
                axum::Json(serde_json::json!({"service":"elon-account-https","ok":true}))
            }),
        )
        .route("/api/auth/login", post(crate::auth_api::login))
        .route("/api/auth/register", post(crate::auth_api::register))
        .route("/api/me", get(crate::auth_api::me))
        .merge(crate::account_security::routes())
        .merge(crate::node_endpoint_transport::asset_access::routes(
            &state.public_url,
        ))
        .with_state(state.clone());
    quant_public::attach(policy::protect(app), &state.data_dir, public_quant)
}
