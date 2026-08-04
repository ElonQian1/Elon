//! Loopback API for reviewing native-agent project-understanding candidates.

use axum::{response::IntoResponse, routing::post, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};

use crate::{
    project_document_authorization::DocumentAutomationMode,
    project_document_native_context_health::{shared_memory_health, MemoryHealthOptions},
    project_document_native_context_observation::{
        finish_window, ingest_event, overview, start_window,
    },
    project_document_native_context_receipt::revise_candidate,
    project_document_native_context_repair::create_relocation_repair_candidate,
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
    #[serde(default)]
    failure_policy: String,
    #[serde(default)]
    include_capabilities: bool,
    #[serde(default)]
    review_reason: String,
    #[serde(default)]
    source_path: String,
    #[serde(default)]
    replacement_path: String,
    #[serde(default)]
    producer: String,
    #[serde(default)]
    benchmark_key: String,
    #[serde(default)]
    measurement_window: String,
    #[serde(default)]
    window_id: String,
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    event: Value,
    #[serde(default)]
    selected_memory_count: usize,
    #[serde(default)]
    returned_metadata_bytes: usize,
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
        .route(
            "/api/project-docs/native-context/repair-relocation",
            post(repair_relocation_handler),
        )
        .route(
            "/api/project-docs/native-context/observation/start",
            post(observation_start_handler),
        )
        .route(
            "/api/project-docs/native-context/observation/event",
            post(observation_event_handler),
        )
        .route(
            "/api/project-docs/native-context/observation/finish",
            post(observation_finish_handler),
        )
        .route(
            "/api/project-docs/native-context/observation/summary",
            post(observation_summary_handler),
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
        &request.review_reason,
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
    let options = MemoryHealthOptions {
        offset: request.offset,
        limit: if request.limit == 0 {
            50
        } else {
            request.limit
        },
        failure_policy: request.failure_policy.clone(),
        include_capabilities: request.include_capabilities,
    };
    response(shared_memory_health(&workspace(&request), &options))
}

async fn repair_relocation_handler(
    Json(request): Json<NativeContextRequest>,
) -> axum::response::Response {
    response(create_relocation_repair_candidate(
        &workspace(&request),
        &request.candidate_id,
        &request.source_path,
        &request.replacement_path,
        if request.producer.trim().is_empty() {
            "pc_memory_repair"
        } else {
            &request.producer
        },
    ))
}

async fn observation_start_handler(
    Json(request): Json<NativeContextRequest>,
) -> axum::response::Response {
    response(start_window(
        &workspace(&request),
        &request.benchmark_key,
        &request.measurement_window,
        &request.session_id,
    ))
}

async fn observation_event_handler(
    Json(request): Json<NativeContextRequest>,
) -> axum::response::Response {
    let workspace = workspace(&request);
    response(ingest_event(&workspace, &request.window_id, request.event))
}

async fn observation_finish_handler(
    Json(request): Json<NativeContextRequest>,
) -> axum::response::Response {
    response(finish_window(
        &workspace(&request),
        &request.window_id,
        request.selected_memory_count,
        request.returned_metadata_bytes,
    ))
}

async fn observation_summary_handler(
    Json(request): Json<NativeContextRequest>,
) -> axum::response::Response {
    response(overview(
        &workspace(&request),
        (!request.benchmark_key.trim().is_empty()).then_some(request.benchmark_key.as_str()),
    ))
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
