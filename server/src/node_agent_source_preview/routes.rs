use super::{
    compose_renderer::{
        capabilities, render_compose_preview, ComposeRenderRequest, RendererCapabilitiesRequest,
    },
    parser::load_document,
    pwa_verifier::{verify_pwa_source, VerifyPwaSourceRequest},
    pwa_writer::{commit_pwa_style, PwaCommitErrorKind},
    types::{CommitPreviewRequest, CommitPwaStyleRequest, LoadPreviewRequest},
    writer::commit_changes,
};
use crate::NodeRuntime;
use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/source-preview/load", post(load_handler))
        .route(
            "/api/source-preview/renderers",
            post(renderer_capabilities_handler),
        )
        .route(
            "/api/source-preview/render-compose",
            post(render_compose_handler),
        )
        .route("/api/source-preview/commit", post(commit_handler))
        .route(
            "/api/source-preview/commit-pwa-style",
            post(commit_pwa_style_handler),
        )
        .route(
            "/api/source-preview/verify-pwa-source",
            post(verify_pwa_source_handler),
        )
}

async fn renderer_capabilities_handler(
    State(_runtime): State<Arc<NodeRuntime>>,
    Json(req): Json<RendererCapabilitiesRequest>,
) -> Response {
    match capabilities(&req.project_root) {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(StatusCode::BAD_REQUEST, error),
    }
}

async fn render_compose_handler(
    State(_runtime): State<Arc<NodeRuntime>>,
    Json(req): Json<ComposeRenderRequest>,
) -> Response {
    match render_compose_preview(req).await {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(StatusCode::BAD_REQUEST, error),
    }
}

async fn load_handler(
    State(_runtime): State<Arc<NodeRuntime>>,
    Json(req): Json<LoadPreviewRequest>,
) -> Response {
    match load_document(&req.project_root, req.layout_file.as_deref()) {
        Ok(document) => Json(document).into_response(),
        Err(error) => error_response(StatusCode::BAD_REQUEST, error),
    }
}

async fn commit_handler(
    State(_runtime): State<Arc<NodeRuntime>>,
    Json(req): Json<CommitPreviewRequest>,
) -> Response {
    match commit_changes(&req) {
        Ok(source_revision) => {
            Json(json!({ "ok": true, "sourceRevision": source_revision })).into_response()
        }
        Err(error) => error_response(StatusCode::CONFLICT, error),
    }
}

pub(super) async fn commit_pwa_style_handler(
    payload: Result<Json<CommitPwaStyleRequest>, JsonRejection>,
) -> Response {
    let Json(req) = match payload {
        Ok(request) => request,
        Err(error) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                anyhow::anyhow!("无效的 PWA 写回请求: {error}"),
            );
        }
    };
    match commit_pwa_style(&req) {
        Ok(value) => Json(value).into_response(),
        Err(error) => {
            let status = match error.kind() {
                PwaCommitErrorKind::Invalid => StatusCode::BAD_REQUEST,
                PwaCommitErrorKind::Conflict => StatusCode::CONFLICT,
                PwaCommitErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            };
            error_response(status, error.into_anyhow())
        }
    }
}

async fn verify_pwa_source_handler(
    State(_runtime): State<Arc<NodeRuntime>>,
    Json(req): Json<VerifyPwaSourceRequest>,
) -> Response {
    match verify_pwa_source(req).await {
        Ok(value) => Json(value).into_response(),
        Err(error) => error_response(StatusCode::BAD_REQUEST, error),
    }
}

fn error_response(status: StatusCode, error: anyhow::Error) -> Response {
    (
        status,
        Json(json!({ "ok": false, "error": format!("{error:#}") })),
    )
        .into_response()
}
