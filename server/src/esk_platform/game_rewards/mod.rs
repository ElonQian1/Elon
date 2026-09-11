//! Profit evidence and reward budget journal; no Paper balances or payment signing keys.
use crate::types::AppState;
use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
mod api;
mod authority;
mod funding;
mod ledger;
pub(crate) mod migration;
mod model;
mod policy;
#[cfg(test)]
pub(crate) mod tests;

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/admin/game-rewards/v1/settlements",
            post(api::settlement),
        )
        .route(
            "/api/admin/game-rewards/v1/budgets/prepare",
            post(api::prepare),
        )
        .route(
            "/api/admin/game-rewards/v1/budgets/confirm-funding",
            post(api::confirm),
        )
        .route("/api/me/game-rewards/v1/account", get(api::account))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(middleware::from_fn(
            super::access::transport::require_secure_transport,
        ))
}
