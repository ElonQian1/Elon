use crate::types::AppState;
use axum::{response::IntoResponse, routing::get, Router};
use std::sync::Arc;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/assets/articles.js", get(script))
        .route("/assets/articles.css", get(styles))
}
async fn script() -> impl IntoResponse {
    (
        [
            ("content-type", "application/javascript; charset=utf-8"),
            ("cache-control", "no-cache"),
        ],
        include_str!("../../../assets/articles.js"),
    )
}
async fn styles() -> impl IntoResponse {
    (
        [
            ("content-type", "text/css; charset=utf-8"),
            ("cache-control", "no-cache"),
        ],
        include_str!("../../../assets/articles.css"),
    )
}
