use crate::types::AppState;
use axum::{routing::get, Router};
use std::sync::Arc;

pub(super) fn routes() -> Router<Arc<AppState>> {
    let headers = [
        ("content-type", "application/javascript; charset=utf-8"),
        ("cache-control", "no-cache"),
    ];
    Router::new()
        .route("/assets/scan_pwa.js", get(move || async move { (headers, include_str!("../../assets/scan_pwa.js")) }))
        .route("/assets/scanPayload.mjs", get(move || async move { (headers, include_str!("../../../../shared/scan/scanPayload.mjs")) }))
        .route("/assets/browserScanner.mjs", get(move || async move { (headers, include_str!("../../../../shared/scan/browserScanner.mjs")) }))
        .route("/assets/browserScanDialog.mjs", get(move || async move { (headers, include_str!("../../../../shared/scan/browserScanDialog.mjs")) }))
        .route("/assets/scan_pwa.css", get(|| async { ([("content-type", "text/css; charset=utf-8"), ("cache-control", "no-cache")], include_str!("../../../../shared/scan/browserScanDialog.css")) }))
        .route("/assets/social_message_actions.js", get(move || async move { (headers, include_str!("../../assets/social_message_actions.js")) }))
        .route("/assets/social_message_transfer.js", get(move || async move { (headers, include_str!("../../assets/social_message_transfer.js")) }))
        .route("/assets/social_message_actions.css", get(|| async { ([("content-type", "text/css; charset=utf-8"), ("cache-control", "no-cache")], include_str!("../../assets/social_message_actions.css")) }))
        .route("/assets/chat_record_presentation.js", get(move || async move { (headers, include_str!("../../assets/chat_record_presentation.js")) }))
        .route("/assets/chat_record_actions.js", get(move || async move { (headers, include_str!("../../assets/chat_record_actions.js")) }))
        .route("/assets/chat_record_video.js", get(move || async move { (headers, include_str!("../../assets/chat_record_video.js")) }))
        .route("/assets/chat_record_media.js", get(move || async move { (headers, include_str!("../../assets/chat_record_media.js")) }))
        .route("/assets/chat_records.js", get(move || async move { (headers, include_str!("../../assets/chat_records.js")) }))
        .route("/assets/chat_records.css", get(|| async { ([("content-type", "text/css; charset=utf-8"), ("cache-control", "no-cache")], include_str!("../../assets/chat_records.css")) }))
        .route("/assets/group_ai_reply_context.js", get(move || async move { (headers, include_str!("../../assets/group_ai_reply_context.js")) }))
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
        .route(
            "/assets/social_voice_player.js",
            get(move || async move { (headers, include_str!("../../assets/social_voice_player.js")) }),
        )
        .route(
            "/assets/social_image_viewer.js",
            get(move || async move { (headers, include_str!("../../assets/social_image_viewer.js")) }),
        )
}
