//! 管理员计费 API。
//!
//! 所有路由需要 Bearer admin token（与现有 admin.rs 一致）。
//!
//! - POST /api/admin/billing/recharge          充值
//! - GET  /api/admin/billing/users?page=1&size=20  用户余额列表
//! - GET  /api/admin/billing/users/:user_id        单用户计费详情
//! - GET  /api/admin/billing/config               查询计费配置
//! - PUT  /api/admin/billing/config               修改计费配置

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::{admin::check_auth, project_auth::json_error, types::AppState};

// ── POST /api/admin/billing/recharge ─────────────────────────────────────────

#[derive(Deserialize)]
pub struct RechargeBody {
    pub user_id: String,
    /// 充值金额（分），例如 1000 = 10 元
    pub amount_fen: i64,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default = "default_operator")]
    pub operator_id: String,
    pub note: Option<String>,
}
fn default_method() -> String {
    "manual".to_string()
}
fn default_operator() -> String {
    "admin".to_string()
}

pub async fn recharge_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RechargeBody>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    if body.amount_fen <= 0 {
        return json_error(StatusCode::BAD_REQUEST, "充值金额必须大于 0");
    }
    match state.store.billing_recharge(
        &body.user_id,
        body.amount_fen,
        &body.method,
        &body.operator_id,
        body.note.as_deref(),
    ) {
        Ok(new_balance) => Json(json!({
            "ok": true,
            "user_id": body.user_id,
            "amount_fen": body.amount_fen,
            "new_balance_fen": new_balance,
            "new_balance_yuan": new_balance as f64 / 100.0,
        }))
        .into_response(),
        Err(e) => {
            tracing::warn!("admin recharge error: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "充值失败，请稍后重试")
        }
    }
}

// ── GET /api/admin/billing/users ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PageQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_size")]
    pub size: i64,
}
fn default_page() -> i64 {
    1
}
fn default_size() -> i64 {
    20
}

pub async fn list_users(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<PageQuery>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    let size = q.size.clamp(1, 100);
    match state.store.billing_admin_list_balances(q.page, size) {
        Ok(rows) => Json(json!({
            "page": q.page,
            "size": size,
            "users": rows,
        }))
        .into_response(),
        Err(e) => {
            tracing::warn!("admin list_users billing error: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询失败")
        }
    }
}

// ── GET /api/admin/billing/users/:user_id ─────────────────────────────────────

pub async fn get_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    match state.store.billing_admin_get_user(&user_id) {
        Ok(Some((balance_row, recharge_records))) => Json(json!({
            "user": balance_row,
            "recharge_records": recharge_records,
        }))
        .into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "该用户未开通计费"),
        Err(e) => {
            tracing::warn!("admin get_user billing error: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询失败")
        }
    }
}

// ── GET /api/admin/billing/config ─────────────────────────────────────────────

pub async fn get_config(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    let rate = state
        .store
        .billing_get_config("usd_to_rmb_rate_x10000")
        .ok()
        .flatten()
        .unwrap_or_else(|| "73000".to_string());
    let markup = state
        .store
        .billing_get_config("markup_x1000")
        .ok()
        .flatten()
        .unwrap_or_else(|| "1200".to_string());
    let threshold = state
        .store
        .billing_get_config("low_balance_threshold_fen")
        .ok()
        .flatten()
        .unwrap_or_else(|| "100".to_string());
    Json(json!({
        "usd_to_rmb_rate_x10000": rate.parse::<i64>().unwrap_or(73000),
        "markup_x1000": markup.parse::<i64>().unwrap_or(1200),
        "low_balance_threshold_fen": threshold.parse::<i64>().unwrap_or(100),
        "note": {
            "usd_to_rmb_rate_x10000": "汇率×10000，73000 = 7.3000",
            "markup_x1000": "加价率×1000，1200 = ×1.2（收费 = 成本 × 1.2）",
            "low_balance_threshold_fen": "低余额阈值（分），低于此值推送 WS 警告"
        }
    }))
    .into_response()
}

// ── PUT /api/admin/billing/config ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SetConfigBody {
    pub key: String,
    pub value: String,
}

const ALLOWED_CONFIG_KEYS: &[&str] = &[
    "usd_to_rmb_rate_x10000",
    "markup_x1000",
    "low_balance_threshold_fen",
];

pub async fn set_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SetConfigBody>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    if !ALLOWED_CONFIG_KEYS.contains(&body.key.as_str()) {
        return json_error(StatusCode::BAD_REQUEST, "未知的配置项");
    }
    // 简单校验：必须是合法整数
    if body.value.parse::<i64>().is_err() {
        return json_error(StatusCode::BAD_REQUEST, "配置值必须是整数");
    }
    match state.store.billing_set_config(&body.key, &body.value) {
        Ok(()) => Json(json!({ "ok": true, "key": body.key, "value": body.value })).into_response(),
        Err(e) => {
            tracing::warn!("admin set_config billing error: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "保存失败")
        }
    }
}
