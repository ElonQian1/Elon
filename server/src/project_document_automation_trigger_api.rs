//! Loopback HTTP adapter for commit-triggered document organization.

use axum::{response::IntoResponse, routing::post, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};

use crate::{
    project_document_automation_trigger::{claim, enqueue, get_pending},
    NodeRuntime,
};

#[derive(Debug, Deserialize)]
struct TriggerRequest {
    project_root: String,
    #[serde(default)]
    commit_sha: String,
    #[serde(default)]
    severity: String,
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    reasons: Vec<String>,
    #[serde(default)]
    trigger_id: String,
    #[serde(default)]
    operation_id: String,
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route(
            "/api/project-docs/organization/automatic-trigger",
            post(enqueue_handler),
        )
        .route(
            "/api/project-docs/organization/automatic-trigger/pending",
            post(pending_handler),
        )
        .route(
            "/api/project-docs/organization/automatic-trigger/claim",
            post(claim_handler),
        )
}

#[cfg(test)]
pub(crate) fn test_routes() -> Router {
    Router::new()
        .route(
            "/api/project-docs/organization/automatic-trigger",
            post(enqueue_handler),
        )
        .route(
            "/api/project-docs/organization/automatic-trigger/pending",
            post(pending_handler),
        )
        .route(
            "/api/project-docs/organization/automatic-trigger/claim",
            post(claim_handler),
        )
}

async fn enqueue_handler(Json(request): Json<TriggerRequest>) -> axum::response::Response {
    trigger_response(enqueue(
        Path::new(request.project_root.trim()),
        &request.commit_sha,
        &request.severity,
        &request.paths,
        &request.reasons,
    ))
}

async fn pending_handler(Json(request): Json<TriggerRequest>) -> axum::response::Response {
    match get_pending(Path::new(request.project_root.trim())) {
        Ok(value) => Json(json!({"ok": true, "trigger": value["trigger"]})).into_response(),
        Err(error) => bad_request(error),
    }
}

async fn claim_handler(Json(request): Json<TriggerRequest>) -> axum::response::Response {
    trigger_response(claim(
        Path::new(request.project_root.trim()),
        &request.trigger_id,
        &request.operation_id,
    ))
}

fn trigger_response(result: anyhow::Result<Value>) -> axum::response::Response {
    match result {
        Ok(value) => Json(json!({"ok": true, "trigger": value})).into_response(),
        Err(error) => bad_request(error),
    }
}

fn bad_request(error: anyhow::Error) -> axum::response::Response {
    (
        axum::http::StatusCode::BAD_REQUEST,
        Json(json!({"ok": false, "error": format!("{error:#}")})),
    )
        .into_response()
}
