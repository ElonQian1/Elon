//! Loopback HTTP adapter for project document organization diagnostics.

use axum::{response::IntoResponse, routing::post, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};

use crate::{
    project_document_file_operations::{
        apply_reviewed_file_operations, ApplyFileOperationsRequest,
    },
    project_document_observability::{
        get_status, mark_applied, mark_dispatched, mark_failure, start_operation,
    },
    NodeRuntime,
};

#[derive(Debug, Deserialize)]
struct OperationRequest {
    project_root: String,
    #[serde(default)]
    operation_id: Option<String>,
    #[serde(default)]
    task_id: Option<String>,
    #[serde(default)]
    error_code: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    recovery: Option<String>,
    #[serde(default)]
    manifest_revision: Option<String>,
    #[serde(default)]
    suggestions_revision: Option<String>,
    #[serde(default)]
    reviewed: bool,
    #[serde(default)]
    operation_ids: Vec<String>,
    #[serde(default)]
    allow_rename: bool,
    #[serde(default)]
    allow_move: bool,
    #[serde(default)]
    expected_catalog_revision: Option<String>,
    #[serde(default)]
    expected_manifest_revision: Option<String>,
    #[serde(default)]
    expected_suggestions_revision: Option<String>,
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/project-docs/organization/start", post(start_handler))
        .route(
            "/api/project-docs/organization/dispatched",
            post(dispatched_handler),
        )
        .route(
            "/api/project-docs/organization/status",
            post(status_handler),
        )
        .route(
            "/api/project-docs/organization/applied",
            post(applied_handler),
        )
        .route("/api/project-docs/organization/fail", post(fail_handler))
        .route(
            "/api/project-docs/organization/apply-files",
            post(apply_files_handler),
        )
}

#[cfg(test)]
pub(crate) fn test_routes() -> Router {
    Router::new()
        .route("/api/project-docs/organization/start", post(start_handler))
        .route(
            "/api/project-docs/organization/dispatched",
            post(dispatched_handler),
        )
        .route(
            "/api/project-docs/organization/status",
            post(status_handler),
        )
        .route(
            "/api/project-docs/organization/applied",
            post(applied_handler),
        )
        .route("/api/project-docs/organization/fail", post(fail_handler))
        .route(
            "/api/project-docs/organization/apply-files",
            post(apply_files_handler),
        )
}

async fn start_handler(Json(request): Json<OperationRequest>) -> axum::response::Response {
    operation_response(start_operation(
        Path::new(request.project_root.trim()),
        request.operation_id.as_deref(),
    ))
}

async fn dispatched_handler(Json(request): Json<OperationRequest>) -> axum::response::Response {
    operation_response(mark_dispatched(
        Path::new(request.project_root.trim()),
        request.operation_id.as_deref(),
        request.task_id.as_deref(),
    ))
}

async fn status_handler(Json(request): Json<OperationRequest>) -> axum::response::Response {
    operation_response(get_status(
        Path::new(request.project_root.trim()),
        request.operation_id.as_deref(),
    ))
}

async fn applied_handler(Json(request): Json<OperationRequest>) -> axum::response::Response {
    operation_response(mark_applied(
        Path::new(request.project_root.trim()),
        request.operation_id.as_deref(),
        request.manifest_revision.as_deref(),
        request.suggestions_revision.as_deref(),
    ))
}

async fn fail_handler(Json(request): Json<OperationRequest>) -> axum::response::Response {
    operation_response(mark_failure(
        Path::new(request.project_root.trim()),
        request.operation_id.as_deref(),
        request.error_code.as_deref().unwrap_or("dispatch_failed"),
        request.message.as_deref().unwrap_or("AI 整理任务发送失败"),
        request
            .recovery
            .as_deref()
            .unwrap_or("确认本机节点和 AI 开发频道可用后重试。"),
    ))
}

async fn apply_files_handler(Json(request): Json<OperationRequest>) -> axum::response::Response {
    value_response(apply_reviewed_file_operations(
        Path::new(request.project_root.trim()),
        ApplyFileOperationsRequest {
            reviewed: request.reviewed,
            operation_ids: &request.operation_ids,
            allow_rename: request.allow_rename,
            allow_move: request.allow_move,
            expected_catalog_revision: request.expected_catalog_revision.as_deref().unwrap_or(""),
            expected_manifest_revision: request.expected_manifest_revision.as_deref(),
            expected_suggestions_revision: request.expected_suggestions_revision.as_deref(),
        },
    ))
}

fn operation_response(result: anyhow::Result<Value>) -> axum::response::Response {
    match result {
        Ok(trace) => Json(json!({"ok": true, "trace": trace})).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": format!("{error:#}")})),
        )
            .into_response(),
    }
}

fn value_response(result: anyhow::Result<Value>) -> axum::response::Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": format!("{error:#}")})),
        )
            .into_response(),
    }
}
