//! 用户端计费 API。
//!
//! - GET /api/me/balance      查询余额与本月消费概览
//! - GET /api/me/billing      分页查询扣费明细

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    billing,
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

// ── GET /api/me/balance ───────────────────────────────────────────────────────

pub async fn get_my_balance(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    let trial_credit = trial_credit_overview(&state, &user.id);

    let balance_fen = match state.store.billing_get_balance(&user.id) {
        Ok(Some(b)) => b,
        Ok(None) => {
            // 未开通计费，返回特殊标记
            return Json(json!({
                "billing_enabled": false,
                "balance_fen": null,
                "balance_yuan": null,
                "this_month_cost_fen": null,
                "this_month_cost_yuan": null,
                "trial_credit": trial_credit,
                "currency": "CNY",
            }))
            .into_response();
        }
        Err(e) => {
            tracing::warn!("get_my_balance store error for {}: {}", user.id, e);
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询失败");
        }
    };

    let month_cost = state.store.billing_get_month_cost(&user.id).unwrap_or(0);

    Json(json!({
        "billing_enabled": true,
        "balance_fen": balance_fen,
        "balance_yuan": balance_fen as f64 / 100.0,
        "this_month_cost_fen": month_cost,
        "this_month_cost_yuan": month_cost as f64 / 100.0,
        "trial_credit": trial_credit,
        "currency": "CNY",
    }))
    .into_response()
}

fn trial_credit_overview(state: &AppState, user_id: &str) -> Value {
    let amount_fen = billing::new_user_trial_credit_fen(&state.store);
    let grant = match state
        .store
        .billing_find_recharge_by_method(user_id, billing::NEW_USER_TRIAL_METHOD)
    {
        Ok(grant) => grant,
        Err(e) => {
            tracing::warn!("trial credit lookup failed for {}: {}", user_id, e);
            None
        }
    };
    let granted_fen = grant.as_ref().map(|record| record.amount_fen);
    json!({
        "amount_fen": amount_fen,
        "amount_yuan": amount_fen as f64 / 100.0,
        "granted": grant.is_some(),
        "granted_fen": granted_fen,
        "granted_yuan": granted_fen.map(|fen| fen as f64 / 100.0),
        "granted_at": grant.as_ref().map(|record| record.created_at.as_str()),
    })
}

// ── GET /api/me/billing ───────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct BillingQuery {
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

pub async fn list_my_billing(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<BillingQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };

    let size = q.size.clamp(1, 100);
    match state.store.billing_list_events(&user.id, q.page, size) {
        Ok((events, total)) => Json(json!({
            "events": events,
            "total": total,
            "page": q.page,
            "size": size,
        }))
        .into_response(),
        Err(e) => {
            tracing::warn!("list_my_billing store error for {}: {}", user.id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询失败")
        }
    }
}
