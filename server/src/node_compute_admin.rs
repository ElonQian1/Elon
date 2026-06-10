//! 管理员节点算力执行证明 API。

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::{admin::check_auth, project_auth::json_error, types::AppState};

#[derive(Deserialize)]
pub struct ComputeRunQuery {
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    100
}

pub async fn list_runs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ComputeRunQuery>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    let runs = match state
        .store
        .admin_list_node_compute_runs(q.status.as_deref(), q.limit)
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("admin list node compute runs error: {}", e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询节点执行证明失败");
        }
    };
    let quality = match state.store.node_quality_scores() {
        Ok(scores) => scores,
        Err(e) => {
            tracing::warn!("admin list node quality scores error: {}", e);
            Default::default()
        }
    };
    Json(json!({
        "status": q.status.unwrap_or_else(|| "all".to_string()),
        "limit": q.limit.clamp(1, 500),
        "runs": runs,
        "quality": quality,
    }))
    .into_response()
}
