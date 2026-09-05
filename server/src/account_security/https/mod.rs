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
mod transport;

pub(crate) async fn serve(legacy_app: Router, state: Arc<AppState>) -> Result<()> {
    let Some(config) = config::Config::from_env()? else {
        return crate::node_endpoint_transport::serve(legacy_app, state).await;
    };
    // Fail before starting legacy ingress if explicitly enabled TLS cannot bind.
    let server = transport::Server::bind(config).await?;
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
        .with_state(state.clone());
    tokio::try_join!(
        crate::node_endpoint_transport::serve(legacy_app, state),
        server.serve(policy::protect(app)),
    )?;
    Ok(())
}
