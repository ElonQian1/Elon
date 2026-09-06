//! Mounted only inside the existing loopback local-admin/Origin protected router.
use super::{
    contract::{ResearchCommand, MAX_RESULT_BYTES},
    ReceiptInput,
};
use crate::NodeRuntime;
use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        DefaultBodyLimit, Path as AxumPath, Query, State,
    },
    http::{header::CACHE_CONTROL, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SubmitInput {
    project_root: String,
    command: ResearchCommand,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingQuery {
    limit: Option<usize>,
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/browser-research/actions", post(submit))
        .route("/api/browser-research/actions/pending", get(pending))
        .route("/api/browser-research/actions/:action_id", get(status))
        .route(
            "/api/browser-research/actions/:action_id/claim",
            post(claim),
        )
        .route(
            "/api/browser-research/actions/:action_id/receipt",
            post(receipt),
        )
        .layer(DefaultBodyLimit::max(MAX_RESULT_BYTES + 4096))
}

fn response(value: Value, status: StatusCode) -> Response {
    (status, [(CACHE_CONTROL, "no-store")], Json(value)).into_response()
}

fn failure(code: &'static str) -> Response {
    let status = match code {
        "action_not_found" => StatusCode::NOT_FOUND,
        "queue_full" | "queue_unavailable" => StatusCode::SERVICE_UNAVAILABLE,
        "action_not_claimable" | "invalid_claim" | "receipt_conflict" | "action_not_executing" => {
            StatusCode::CONFLICT
        }
        _ => StatusCode::BAD_REQUEST,
    };
    response(json!({"ok":false,"error":code}), status)
}

async fn submit(
    State(runtime): State<Arc<NodeRuntime>>,
    input: Result<Json<SubmitInput>, JsonRejection>,
) -> Response {
    let Ok(Json(input)) = input else {
        return failure("invalid_input");
    };
    if input.project_root.is_empty()
        || input.project_root.len() > 4096
        || input.project_root.contains('\0')
    {
        return failure("invalid_project");
    }
    match runtime
        .browser_research
        .enqueue(Path::new(&input.project_root), input.command)
    {
        Ok(action) => response(json!({"ok":true,"action":action}), StatusCode::OK),
        Err(code) => failure(code),
    }
}

async fn pending(
    State(runtime): State<Arc<NodeRuntime>>,
    query: Result<Query<PendingQuery>, QueryRejection>,
) -> Response {
    let Ok(Query(query)) = query else {
        return failure("invalid_input");
    };
    match runtime.browser_research.pending(query.limit.unwrap_or(8)) {
        Ok(actions) => response(json!({"ok":true,"actions":actions}), StatusCode::OK),
        Err(code) => failure(code),
    }
}

async fn status(
    State(runtime): State<Arc<NodeRuntime>>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    match runtime.browser_research.admin_action(&id) {
        Ok(action) => response(
            json!({"ok":true,"terminal":super::terminal(&action.status),"action":action}),
            StatusCode::OK,
        ),
        Err(code) => failure(code),
    }
}

async fn claim(
    State(runtime): State<Arc<NodeRuntime>>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    match runtime.browser_research.claim(&id) {
        Ok(value) => response(
            json!({"ok":true,"action":value.action,"claim_token":value.claim_token}),
            StatusCode::OK,
        ),
        Err(code) => failure(code),
    }
}

async fn receipt(
    State(runtime): State<Arc<NodeRuntime>>,
    AxumPath(id): AxumPath<String>,
    input: Result<Json<ReceiptInput>, JsonRejection>,
) -> Response {
    let Ok(Json(input)) = input else {
        return failure("invalid_receipt");
    };
    match runtime.browser_research.record_receipt(&id, input) {
        // The bridge already owns the result; do not send a second copy back.
        Ok(action) => response(
            json!({"ok":true,"action_id":action.action_id,"status":action.status}),
            StatusCode::OK,
        ),
        Err(code) => failure(code),
    }
}
