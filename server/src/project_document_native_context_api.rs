//! Loopback API for reviewing native-agent project-understanding candidates.

use axum::{response::IntoResponse, routing::post, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};

use crate::{
    project_document_authorization::DocumentAutomationMode,
    project_document_native_context_health::shared_memory_health,
    project_document_native_context_receipt::revise_candidate,
    project_document_native_context_review::{candidate_page, review_candidates},
    NodeRuntime,
};

#[derive(Debug, Default, Deserialize)]
struct NativeContextRequest {
    project_root: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    limit: usize,
    #[serde(default)]
    candidate_ids: Vec<String>,
    #[serde(default)]
    action: String,
    #[serde(default)]
    authorization_mode: DocumentAutomationMode,
    #[serde(default)]
    expected_catalog_revision: Option<String>,
    #[serde(default)]
    expected_suggestions_revision: Option<String>,
    #[serde(default)]
    candidate_id: String,
    #[serde(default)]
    expected_updated_at_ms: u64,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    topics: Vec<String>,
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route(
            "/api/project-docs/native-context/candidates",
            post(candidates_handler),
        )
        .route(
            "/api/project-docs/native-context/review",
            post(review_handler),
        )
        .route(
            "/api/project-docs/native-context/revise",
            post(revise_handler),
        )
        .route(
            "/api/project-docs/native-context/health",
            post(health_handler),
        )
}

async fn candidates_handler(Json(request): Json<NativeContextRequest>) -> axum::response::Response {
    let workspace = workspace(&request);
    response(candidate_page(
        &workspace,
        &request.status,
        request.offset,
        request.limit,
    ))
}

async fn review_handler(Json(request): Json<NativeContextRequest>) -> axum::response::Response {
    let workspace = workspace(&request);
    response(review_candidates(
        &workspace,
        request.candidate_ids,
        &request.action,
        request.authorization_mode,
        request.expected_catalog_revision.as_deref(),
        request.expected_suggestions_revision.as_deref(),
    ))
}

async fn revise_handler(Json(request): Json<NativeContextRequest>) -> axum::response::Response {
    let workspace = workspace(&request);
    response(revise_candidate(
        &workspace,
        &request.candidate_id,
        request.expected_updated_at_ms,
        request.summary,
        request.topics,
    ))
}

async fn health_handler(Json(request): Json<NativeContextRequest>) -> axum::response::Response {
    response(shared_memory_health(&workspace(&request)))
}

fn workspace(request: &NativeContextRequest) -> PathBuf {
    PathBuf::from(request.project_root.trim())
}

fn response(result: anyhow::Result<Value>) -> axum::response::Response {
    match result {
        Ok(value) => Json(json!({"ok": true, "result": value})).into_response(),
        Err(error) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": format!("{error:#}")})),
        )
            .into_response(),
    }
}
