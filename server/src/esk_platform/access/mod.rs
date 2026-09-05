use crate::types::AppState;
use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
    Router,
};
use std::sync::Arc;

mod api;
pub(crate) mod migration;
mod model;
pub(crate) mod transport;
mod validation;

pub(crate) use model::*;
pub(crate) use validation::{
    challenge, valid_client, valid_grant_id, valid_scopes, valid_secret, validate_authorize,
    validate_exchange, validate_revoke,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/me/asset-access/authorize", post(api::authorize))
        .route("/api/asset-access/token", post(api::exchange))
        .route("/api/me/asset-access/grants", get(api::list_grants))
        .route(
            "/api/me/asset-access/grants/:grant_id/revoke",
            post(api::revoke_grant),
        )
        .route("/api/asset-access/revoke", post(api::revoke_self))
        .route("/api/asset-access/me", get(api::me))
        .route("/api/asset-access/esk", get(api::esk))
        .layer(DefaultBodyLimit::max(8 * 1024))
        .layer(middleware::from_fn(transport::require_secure_transport))
        .layer(middleware::from_fn(super::api::private_no_store))
}

#[cfg(test)]
mod tests_validation;
