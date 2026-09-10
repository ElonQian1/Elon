//! First-party game authorization, independent from quant read-only asset grants.
use crate::types::AppState;
use axum::{extract::DefaultBodyLimit, middleware, routing::post, Router};
use std::sync::Arc;
mod api;
mod authority;
pub(crate) mod browser;
mod issue;
pub(crate) mod migration;
mod model;
mod observe;
mod policy;
mod protocol;
mod revoke;
#[cfg(test)]
pub(crate) mod test_support;

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/me/game-access/authorize", post(api::authorize))
        .route("/api/game-access/v1/token", post(api::exchange))
        .route("/api/game-access/v1/observe", post(api::observe))
        .route(
            "/api/me/game-access/grants/:grant_id/revoke",
            post(api::revoke_owner),
        )
        .route("/api/game-access/v1/revoke", post(api::revoke_self))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(middleware::from_fn(
            super::access::transport::require_secure_transport,
        ))
}
