use crate::types::AppState;
use axum::{response::IntoResponse, routing::get, Router};
use std::sync::Arc;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/assets/articles.js", get(script))
        .route("/assets/articles.css", get(styles))
        .route("/assets/article_square.js", get(square))
        .route("/assets/article_square_media.js", get(square_media))
}
async fn square() -> impl IntoResponse {
    (
        [
            ("content-type", "application/javascript; charset=utf-8"),
            ("cache-control", "no-cache"),
        ],
        include_str!("../../../assets/article_square.js"),
    )
}
async fn square_media() -> impl IntoResponse {
    (
        [
            ("content-type", "application/javascript; charset=utf-8"),
            ("cache-control", "no-cache"),
        ],
        include_str!("../../../assets/article_square_media.js"),
    )
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
