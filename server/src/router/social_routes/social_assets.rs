use crate::types::AppState;
use axum::{routing::get, Router};
use std::sync::Arc;

pub(super) fn routes() -> Router<Arc<AppState>> {
    let headers = [
        ("content-type", "application/javascript; charset=utf-8"),
        ("cache-control", "no-cache"),
    ];
    Router::new()
        .route("/assets/social_source_links.js", get(move || async move { (headers, include_str!("../../assets/social_source_links.js")) }))
        .route("/assets/social_source_compose.js", get(move || async move { (headers, include_str!("../../assets/social_source_compose.js")) }))
        .route("/assets/social_source_worker.js", get(move || async move { (headers, include_str!("../../assets/social_source_worker.js")) }))
        .route("/assets/vendor/jsqr.js", get(move || async move { (headers, include_str!("../../assets/vendor/jsqr.js")) }))
        .route(
            "/assets/ai_conversation_share.js",
            get(move || async move {
                (
                    headers,
                    include_str!("../../assets/ai_conversation_share.js"),
                )
            }),
        )
        .route(
            "/assets/ai_conversation_reader.js",
            get(move || async move {
                (
                    headers,
                    include_str!("../../assets/ai_conversation_reader.js"),
                )
            }),
        )
        .route(
            "/assets/ai_conversation_rich.js",
            get(move || async move {
                (
                    headers,
                    include_str!("../../assets/ai_conversation_rich.js"),
                )
            }),
        )
        .route(
            "/assets/ai_share_marked.js",
            get(move || async move { (headers, include_str!("../../assets/ai_share_marked.js")) }),
        )
        .route(
            "/assets/ai_conversation_share.css",
            get(|| async {
                (
                    [
                        ("content-type", "text/css; charset=utf-8"),
                        ("cache-control", "no-cache"),
                    ],
                    include_str!("../../assets/ai_conversation_share.css"),
                )
            }),
        )
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
