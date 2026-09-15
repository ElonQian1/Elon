use crate::types::AppState;
use axum::{routing::get, Router};
use std::sync::Arc;

pub(super) fn routes() -> Router<Arc<AppState>> {
    let headers = [
        ("content-type", "application/javascript; charset=utf-8"),
        ("cache-control", "no-cache"),
    ];
    Router::new()
        .route(
            "/assets/social_chat_cache.js",
            get(
                move || async move { (headers, include_str!("../../assets/social_chat_cache.js")) },
            ),
        )
        .route(
            "/assets/social_chat_recovery.js",
            get(move || async move {
                (
                    headers,
                    include_str!("../../assets/social_chat_recovery.js"),
                )
            }),
        )
        .route(
            "/assets/social_chat_view.js",
            get(move || async move { (headers, include_str!("../../assets/social_chat_view.js")) }),
        )
}
