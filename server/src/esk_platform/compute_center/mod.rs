mod api;
pub(crate) mod browser;
#[path = "domain.rs"]
mod domain;
pub(crate) use domain::*;

pub(crate) fn routes() -> axum::Router<std::sync::Arc<crate::types::AppState>> {
    axum::Router::new()
        .route("/api/me/esk-compute-center", axum::routing::get(api::get))
        .layer(axum::middleware::from_fn(
            super::access::transport::require_secure_transport,
        ))
}
