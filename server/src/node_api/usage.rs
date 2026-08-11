use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::{
    compute_federation::legacy::project_legacy_llm_v1_lists, project_auth::auth_from_headers,
    store::NodeComputeRun, types::AppState,
};

/// GET /api/me/node-usage — 当前用户使用和提供节点算力的执行账本。
pub async fn my_node_usage(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    let consuming = match state
        .store
        .list_node_compute_runs_for_consumer(&user.id, 50)
    {
        Ok(runs) => runs,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };
    let providing = match state
        .store
        .list_node_compute_runs_for_provider(&user.id, 50)
    {
        Ok(runs) => runs,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    Json(node_usage_response(consuming, providing)).into_response()
}

fn node_usage_response(
    consuming: Vec<NodeComputeRun>,
    providing: Vec<NodeComputeRun>,
) -> serde_json::Value {
    let federation_compatibility = project_legacy_llm_v1_lists(&consuming, &providing);
    serde_json::json!({
        "consuming": consuming,
        "providing": providing,
        "federation_compatibility": federation_compatibility,
    })
}

#[cfg(test)]
#[path = "usage_tests.rs"]
mod tests;
