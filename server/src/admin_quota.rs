//! 管理员配额管理 API
//!
//! 所有路由均需 `Authorization: Bearer <ADMIN_TOKEN>` 鉴权。
//!
//! 路由：
//!   GET    /api/admin/quotas                —— 列出所有已设置配额的用户
//!   PUT    /api/admin/quotas/:user_id       —— 设置/更新用户配额
//!   DELETE /api/admin/quotas/:user_id       —— 删除配额（恢复无限制）

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{admin::check_auth, project_auth::json_error, types::AppState};

/// GET /api/admin/quotas
pub async fn list_quotas(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    match state.store.admin_list_quotas() {
        Ok(quotas) => Json(serde_json::json!({ "quotas": quotas })).into_response(),
        Err(e) => {
            tracing::warn!("admin_list_quotas error: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询失败")
        }
    }
}

#[derive(Deserialize)]
pub struct UpsertQuotaBody {
    /// 月度 token 上限；为 null 时表示无限制
    pub monthly_token_limit: Option<i64>,
    #[serde(default)]
    pub is_blocked: bool,
    pub block_reason: Option<String>,
}

/// PUT /api/admin/quotas/:user_id
pub async fn upsert_quota(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(body): Json<UpsertQuotaBody>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    // monthly_token_limit 若有值，至少要 > 0
    if let Some(limit) = body.monthly_token_limit {
        if limit <= 0 {
            return json_error(StatusCode::BAD_REQUEST, "monthly_token_limit 必须大于 0");
        }
    }
    match state.store.admin_upsert_quota(
        &user_id,
        body.monthly_token_limit,
        body.is_blocked,
        body.block_reason.as_deref(),
    ) {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => {
            tracing::warn!("admin_upsert_quota error for {}: {}", user_id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "操作失败")
        }
    }
}

/// DELETE /api/admin/quotas/:user_id
pub async fn delete_quota(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    match state.store.admin_delete_quota(&user_id) {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => {
            tracing::warn!("admin_delete_quota error for {}: {}", user_id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "操作失败")
        }
    }
}
