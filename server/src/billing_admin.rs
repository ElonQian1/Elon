//! 管理员计费 API。
//!
//! 所有路由需要 Bearer admin token（与现有 admin.rs 一致）。
//!
//! - POST /api/admin/billing/recharge          充值
//! - GET  /api/admin/billing/users?page=1&size=20  用户余额列表
//! - GET  /api/admin/billing/users/:user_id        单用户计费详情
//! - GET  /api/admin/billing/events                最近扣费解释
//! - GET  /api/admin/billing/reservations          预授权明细
//! - GET  /api/admin/billing/price-rules           查询模型/算力计价规则
//! - PUT  /api/admin/billing/price-rules           新增或更新计价规则
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

use crate::{
    admin::check_auth, project_auth::json_error, store::BillingPriceRuleUpsert, types::AppState,
};

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

#[derive(Deserialize)]
pub struct ReservationQuery {
    pub status: Option<String>,
    #[serde(default = "default_reservation_limit")]
    pub limit: i64,
}
fn default_reservation_limit() -> i64 {
    100
}

#[derive(Deserialize)]
pub struct BillingEventsQuery {
    pub user_id: Option<String>,
    #[serde(default = "default_reservation_limit")]
    pub limit: i64,
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

// ── GET /api/admin/billing/events ───────────────────────────────────────────

pub async fn list_events(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<BillingEventsQuery>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    let limit = q.limit.clamp(1, 500);
    let user_id = q
        .user_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match state.store.admin_billing_events(user_id, limit) {
        Ok(rows) => Json(json!({
            "user_id": user_id,
            "limit": limit,
            "rows": rows,
        }))
        .into_response(),
        Err(e) => {
            tracing::warn!("admin billing events error: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询失败")
        }
    }
}

// ── GET /api/admin/billing/reservations ──────────────────────────────────────

pub async fn list_reservations(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ReservationQuery>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    let limit = q.limit.clamp(1, 500);
    match state
        .store
        .admin_billing_reservations(q.status.as_deref(), limit)
    {
        Ok(rows) => Json(json!({
            "status": q.status.unwrap_or_else(|| "all".to_string()),
            "limit": limit,
            "rows": rows,
        }))
        .into_response(),
        Err(e) => {
            tracing::warn!("admin billing reservations error: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询失败")
        }
    }
}

// ── GET/PUT /api/admin/billing/price-rules ─────────────────────────────────

pub async fn list_price_rules(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    match state.store.billing_list_price_rules() {
        Ok(rows) => Json(json!({ "rules": rows })).into_response(),
        Err(e) => {
            tracing::warn!("admin billing list price rules error: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询失败")
        }
    }
}

pub async fn upsert_price_rule(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<BillingPriceRuleUpsert>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    if body.pattern.trim().is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "匹配规则不能为空");
    }
    if !body.input_usd_per_m.is_finite()
        || !body.cached_usd_per_m.is_finite()
        || !body.output_usd_per_m.is_finite()
        || body.input_usd_per_m < 0.0
        || body.cached_usd_per_m < 0.0
        || body.output_usd_per_m < 0.0
    {
        return json_error(StatusCode::BAD_REQUEST, "计价金额必须是非负数字");
    }
    match state.store.billing_upsert_price_rule(&body) {
        Ok(rule) => Json(json!({ "ok": true, "rule": rule })).into_response(),
        Err(e) => {
            tracing::warn!("admin billing upsert price rule error: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "保存失败")
        }
    }
}

// ── GET /api/admin/billing/config ─────────────────────────────────────────────

pub async fn get_config(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    let config_int = |key: &str, default: i64| {
        state
            .store
            .billing_get_config(key)
            .ok()
            .flatten()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(default)
    };
    Json(json!({
        "usd_to_rmb_rate_x10000": config_int("usd_to_rmb_rate_x10000", 73000),
        "markup_x1000": config_int("markup_x1000", 1200),
        "low_balance_threshold_fen": config_int("low_balance_threshold_fen", 100),
        "billing_default_reservation_fen": config_int("billing_default_reservation_fen", 1),
        "billing_cli_dev_reservation_fen": config_int("billing_cli_dev_reservation_fen", 100),
        "billing_cli_chat_reservation_fen": config_int("billing_cli_chat_reservation_fen", 10),
        "billing_node_llm_min_reservation_fen": config_int("billing_node_llm_min_reservation_fen", 1),
        "billing_image_min_reservation_fen": config_int("billing_image_min_reservation_fen", 1),
        "billing_realtime_voice_min_reservation_fen": config_int("billing_realtime_voice_min_reservation_fen", 1),
        "new_user_trial_credit_fen": config_int("new_user_trial_credit_fen", 30000),
        "external_app_fb2_trial_credit_fen": config_int("external_app_fb2_trial_credit_fen", 100),
        "external_app_bb64a_trial_credit_fen": config_int("external_app_bb64a_trial_credit_fen", 100),
        "billing_open_reservation_alert_threshold": config_int("billing_open_reservation_alert_threshold", 100),
        "node_provider_revenue_share_x1000": config_int("node_provider_revenue_share_x1000", 800),
        "node_payout_min_fen": config_int("node_payout_min_fen", 100),
        "note": {
            "usd_to_rmb_rate_x10000": "汇率×10000，73000 = 7.3000",
            "markup_x1000": "加价率×1000，1200 = ×1.2（收费 = 成本 × 1.2）",
            "low_balance_threshold_fen": "低余额阈值（分），低于此值推送 WS 警告",
            "billing_default_reservation_fen": "默认最低预授权冻结金额（分）",
            "billing_cli_dev_reservation_fen": "开发类 CLI 调用预授权冻结金额（分）",
            "billing_cli_chat_reservation_fen": "轻量聊天 CLI 调用预授权冻结金额（分）",
            "billing_node_llm_min_reservation_fen": "节点 LLM 调用最低预授权冻结金额（分）",
            "billing_image_min_reservation_fen": "图片生成最低预授权冻结金额（分）",
            "billing_realtime_voice_min_reservation_fen": "AI 实时语音对话每轮最低预授权冻结金额（分）",
            "new_user_trial_credit_fen": "普通新用户首次 AI 调用自动赠送的试用余额（分），设为 0 可关闭",
            "external_app_fb2_trial_credit_fen": "fb2 用户首次创建外部应用会话时赠送的 AI 回复试用余额（分），ASR/TTS 不消耗此额度",
            "billing_open_reservation_alert_threshold": "冻结中预授权数量超过该值时产生对账告警",
            "node_provider_revenue_share_x1000": "节点提供者分账比例×1000，800 = 消费者真实扣费的 80%",
            "node_payout_min_fen": "节点收益最低提现金额（分），100 = 1 元"
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
    "billing_default_reservation_fen",
    "billing_cli_dev_reservation_fen",
    "billing_cli_chat_reservation_fen",
    "billing_node_llm_min_reservation_fen",
    "billing_image_min_reservation_fen",
    "billing_realtime_voice_min_reservation_fen",
    "new_user_trial_credit_fen",
    "external_app_fb2_trial_credit_fen",
    "external_app_bb64a_trial_credit_fen",
    "billing_open_reservation_alert_threshold",
    "node_provider_revenue_share_x1000",
    "node_payout_min_fen",
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
