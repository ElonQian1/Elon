use super::{
    parser::load_document,
    types::{CommitPreviewRequest, LoadPreviewRequest},
    writer::commit_changes,
};
use crate::NodeRuntime;
use axum::{
    extract::State,
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
        .route("/api/source-preview/commit", post(commit_handler))
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

fn error_response(status: StatusCode, error: anyhow::Error) -> Response {
    (
        status,
        Json(json!({ "ok": false, "error": format!("{error:#}") })),
    )
        .into_response()
}
