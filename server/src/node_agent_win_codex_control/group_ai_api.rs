use super::*;
use crate::NodeRuntime;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Worker {
    worker_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    worker_id: String,
    result: Value,
}

// Mounted inside the existing local-admin-token / trusted-Origin protected router.
pub(super) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/codex-control/group-ai/pending", get(pending))
        .route("/api/codex-control/group-ai/:id/claim", post(claim))
        .route("/api/codex-control/group-ai/:id/receipt", post(receipt))
}
async fn pending(State(rt): State<Arc<NodeRuntime>>) -> Json<Value> {
    Json(json!({"command_ids":rt.win_codex_control.group_ai.pending()}))
}
async fn claim(
    State(rt): State<Arc<NodeRuntime>>,
    Path(id): Path<String>,
    Json(w): Json<Worker>,
) -> Response {
    match rt.win_codex_control.group_ai.claim(&id, &w.worker_id) {
        Ok(command) => Json(json!({"command":command})).into_response(),
        Err(code) => (
            axum::http::StatusCode::CONFLICT,
            Json(json!({"error":code})),
        )
            .into_response(),
    }
}
async fn receipt(
    State(rt): State<Arc<NodeRuntime>>,
    Path(id): Path<String>,
    Json(r): Json<Receipt>,
) -> Response {
    match rt
        .win_codex_control
        .group_ai
        .receipt(&id, &r.worker_id, r.result)
    {
        Ok(()) => Json(json!({"ok":true})).into_response(),
        Err(code) => (
            axum::http::StatusCode::CONFLICT,
            Json(json!({"error":code})),
        )
            .into_response(),
    }
}
