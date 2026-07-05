use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::{project_auth::auth_from_headers, types::AppState};

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

    Json(serde_json::json!({
        "consuming": consuming,
        "providing": providing,
    }))
    .into_response()
}
