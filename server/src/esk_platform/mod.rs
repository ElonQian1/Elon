use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::types::AppState;

mod api;
mod history_api;
mod history_model;
pub(crate) mod migration;
mod model;
mod payment_identity;
pub(crate) mod sellback;
mod validation;

pub(crate) use history_model::*;
pub(crate) use model::*;
pub(crate) use validation::{validate_policy_integrity, validate_prepared_input};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .merge(sellback::routes())
        .route("/api/me/assets/esk/platform", get(api::get_my_account))
        .route(
            "/api/me/assets/esk/platform/history",
            get(history_api::get_my_history),
        )
        .route(
            "/api/admin/assets/esk/platform-allocations/prepare",
            post(api::prepare_allocation),
        )
        .route(
            "/api/admin/assets/esk/platform-allocations/:allocation_id/record",
            post(api::record_allocation),
        )
        .route(
            "/api/admin/assets/esk/platform-allocations/:allocation_id/cancel",
            post(api::cancel_allocation),
        )
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(middleware::from_fn(api::private_no_store))
}

#[cfg(test)]
mod http_tests;
