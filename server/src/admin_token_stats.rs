//! 管理员 Token 用量统计 API
//!
//! 所有路由均需 `Authorization: Bearer <ADMIN_TOKEN>` 鉴权。
//!
//! 路由：
//!   GET /api/admin/token-stats/summary?days=30
//!   GET /api/admin/token-stats/users?days=30&limit=50
//!   GET /api/admin/token-stats/users/:user_id?days=30
//!   GET /api/admin/token-stats/trend?days=30
//!   GET /api/admin/token-stats/accounting-audit?days=30&limit=100

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{admin::check_auth, project_auth::json_error, types::AppState};

#[derive(Deserialize)]
pub struct StatsQuery {
    #[serde(default = "default_days")]
    pub days: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}
fn default_days() -> i64 {
    30
}
fn default_limit() -> i64 {
    50
}

/// GET /api/admin/token-stats/summary?days=30
pub async fn get_platform_summary(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<StatsQuery>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    let days = q.days.clamp(1, 365);
    match state.store.admin_platform_summary(days) {
        Ok(summary) => Json(summary).into_response(),
        Err(e) => {
            tracing::warn!("admin_platform_summary error: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询失败")
        }
    }
}

/// GET /api/admin/token-stats/users?days=30&limit=50
pub async fn get_users_usage(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<StatsQuery>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    let days = q.days.clamp(1, 365);
    let limit = q.limit.clamp(1, 200);
    match state.store.admin_user_usage_list(days, limit) {
        Ok(rows) => Json(serde_json::json!({ "users": rows, "days": days })).into_response(),
        Err(e) => {
            tracing::warn!("admin_user_usage_list error: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询失败")
        }
    }
}

/// GET /api/admin/token-stats/users/:user_id?days=30
pub async fn get_user_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Query(q): Query<StatsQuery>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    let days = q.days.clamp(1, 365);
    match state.store.admin_user_detail(&user_id, days) {
        Ok(detail) => Json(detail).into_response(),
        Err(e) => {
            tracing::warn!("admin_user_detail error for {}: {}", user_id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询失败")
        }
    }
}

/// GET /api/admin/token-stats/trend?days=30
pub async fn get_platform_trend(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<StatsQuery>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    let days = q.days.clamp(1, 90);
    match state.store.admin_platform_trend(days) {
        Ok(rows) => Json(serde_json::json!({ "trend": rows, "days": days })).into_response(),
        Err(e) => {
            tracing::warn!("admin_platform_trend error: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询失败")
        }
    }
}

/// GET /api/admin/token-stats/accounting-audit?days=30&limit=100
pub async fn get_accounting_audit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<StatsQuery>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    let days = q.days.clamp(1, 365);
    let limit = q.limit.clamp(1, 500);
    match state.store.admin_accounting_audit(days, limit) {
        Ok(rows) => Json(serde_json::json!({
            "rows": rows,
            "days": days,
            "limit": limit,
        }))
        .into_response(),
        Err(e) => {
            tracing::warn!("admin_accounting_audit error: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询失败")
        }
    }
}
