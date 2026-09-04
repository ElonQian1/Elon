use crate::types::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub(crate) mod api;
pub(crate) mod domain;
pub(crate) mod migration;
mod wire;
pub(crate) use domain::*;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/assets/esk/platform/sellback-requests",
            get(api::list).post(api::submit),
        )
        .route(
            "/api/me/assets/esk/platform/sellback-requests/lookup",
            post(api::lookup),
        )
        .route(
            "/api/me/assets/esk/platform/sellback-requests/:request_id",
            get(api::get),
        )
        .route(
            "/api/me/assets/esk/platform/sellback-requests/:request_id/cancel",
            post(api::cancel),
        )
}
