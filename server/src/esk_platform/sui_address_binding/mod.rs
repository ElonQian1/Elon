use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::types::AppState;

mod api;
mod challenge;
mod crypto;
pub(crate) mod migration;
mod model;
mod wire;

#[cfg(test)]
mod crypto_vector_tests;

pub(crate) use challenge::*;
pub(crate) use crypto::*;
pub(crate) use model::*;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/assets/esk/platform/sui-address-binding/challenges",
            post(api::create_challenge),
        )
        .route(
            "/api/me/assets/esk/platform/sui-address-binding/challenges/:challenge_id/complete",
            post(api::complete_challenge),
        )
        .route(
            "/api/me/assets/esk/platform/sui-address-binding",
            get(api::get_my_binding),
        )
}
