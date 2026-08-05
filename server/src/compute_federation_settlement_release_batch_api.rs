use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    compute_federation_settlement_release_batch_service::{self, ReleaseDueComputeSettlementsBody},
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/admin/compute/settlement-releases/due",
        get(list_due).post(release_due),
    )
}

#[derive(Debug, Deserialize)]
struct DueQuery {
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    cursor: Option<String>,
}

async fn list_due(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<DueQuery>,
) -> Response {
    if let Err(response) = platform_admin(&state, &headers) {
        return response;
    }
    batch_response(
        compute_federation_settlement_release_batch_service::list_due_for_platform_admin(
            &state.store,
            query.limit,
            query.cursor.as_deref(),
        ),
    )
}

async fn release_due(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ReleaseDueComputeSettlementsBody>,
) -> Response {
    let admin_user_id = match platform_admin(&state, &headers) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    batch_response(
        compute_federation_settlement_release_batch_service::release_due_for_platform_admin(
            &state.store,
            &admin_user_id,
            body,
        ),
    )
}

fn platform_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    let user = auth_from_headers(state, headers)
        .map_err(|error| json_error(StatusCode::UNAUTHORIZED, error))?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "只有平台管理员可以读取或处理到期算力结算释放队列",
        ));
    }
    Ok(user.id)
}

fn batch_response<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":format!("{error:#}")})),
        )
            .into_response(),
    }
}

fn default_limit() -> usize {
    50
}
