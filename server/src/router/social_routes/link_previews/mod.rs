//! Authenticated preview endpoint and its static client assets.
mod service;
use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/me/link-preview",
            post(preview).layer(DefaultBodyLimit::max(8192)),
        )
        .route(
            "/api/me/link-preview/report",
            post(report).layer(DefaultBodyLimit::max(16384)),
        )
        .route(
            "/assets/social_links.js",
            get(|| async {
                asset(
                    "application/javascript",
                    include_str!("../../../assets/social_links.js"),
                )
            }),
        )
        .route(
            "/assets/social_link_viewer.js",
            get(|| async {
                asset(
                    "application/javascript",
                    include_str!("../../../assets/social_link_viewer.js"),
                )
            }),
        )
        .route(
            "/assets/social_links.css",
            get(|| async { asset("text/css", include_str!("../../../assets/social_links.css")) }),
        )
}
fn asset(mime: &'static str, body: &'static str) -> impl IntoResponse {
    (
        [
            ("content-type", mime),
            ("cache-control", "no-cache"),
            ("x-content-type-options", "nosniff"),
        ],
        body,
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    url: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportRequest {
    url: String,
    read: service::Read,
}

async fn preview(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<Request>,
) -> Response {
    if auth_from_headers(&state, &headers).is_err() {
        return json_error(StatusCode::UNAUTHORIZED, "请先登录");
    }
    let Some(url) = service::public_url(&input.url) else {
        return json_error(StatusCode::BAD_REQUEST, "链接格式不支持预览");
    };
    let result = service::preview(url).await;
    // Signed share parameters stay in memory only, never in a cacheable HTTP response or log.
    ([("cache-control", "private, no-store")], Json(result)).into_response()
}

// A member who opened the original page shares its re-validated metadata with other readers.
async fn report(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<ReportRequest>,
) -> Response {
    let Ok(user) = auth_from_headers(&state, &headers) else {
        return json_error(StatusCode::UNAUTHORIZED, "请先登录");
    };
    let Some(url) = service::public_url(&input.url) else {
        return json_error(StatusCode::BAD_REQUEST, "链接格式不支持预览");
    };
    match service::report(&user.id, url, &input.read).await {
        Ok(result) => ([("cache-control", "private, no-store")], Json(result)).into_response(),
        Err(message) => json_error(StatusCode::UNPROCESSABLE_ENTITY, message),
    }
}
