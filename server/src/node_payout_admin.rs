//! 管理员节点提现处理 API。

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

#[derive(Deserialize)]
pub struct PayoutListQuery {
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

#[derive(Deserialize)]
pub struct ResolvePayoutBody {
    #[serde(default = "default_operator")]
    pub operator_id: String,
    pub admin_note: Option<String>,
}

fn default_limit() -> i64 {
    100
}

fn default_operator() -> String {
    "admin".to_string()
}

pub async fn list_payouts(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<PayoutListQuery>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    match state
        .store
        .admin_list_node_payout_requests(q.status.as_deref(), q.limit)
    {
        Ok(rows) => Json(json!({
            "status": q.status.unwrap_or_else(|| "all".to_string()),
            "limit": q.limit.clamp(1, 500),
            "payouts": rows,
        }))
        .into_response(),
        Err(e) => {
            tracing::warn!("admin list node payouts error: {}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询提现申请失败")
        }
    }
}

pub async fn mark_paid(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(payout_id): Path<String>,
    Json(body): Json<ResolvePayoutBody>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    match state.store.admin_mark_node_payout_paid(
        &payout_id,
        &body.operator_id,
        body.admin_note.as_deref(),
    ) {
        Ok(payout) => Json(json!({ "ok": true, "payout": payout })).into_response(),
        Err(e) => store_error("确认打款失败", e),
    }
}

pub async fn reject(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(payout_id): Path<String>,
    Json(body): Json<ResolvePayoutBody>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    match state.store.admin_reject_node_payout(
        &payout_id,
        &body.operator_id,
        body.admin_note.as_deref(),
    ) {
        Ok(payout) => Json(json!({ "ok": true, "payout": payout })).into_response(),
        Err(e) => store_error("拒绝提现失败", e),
    }
}

fn store_error(prefix: &str, err: anyhow::Error) -> Response {
    let msg = err.to_string();
    let status = if msg.contains("不存在") {
        StatusCode::NOT_FOUND
    } else if msg.contains("待处理") || msg.contains("不能为空") {
        StatusCode::BAD_REQUEST
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };
    if status == StatusCode::INTERNAL_SERVER_ERROR {
        tracing::warn!("{}: {}", prefix, msg);
    }
    json_error(status, &format!("{prefix}: {msg}"))
}
