use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{project_auth::auth_from_headers, store::CreateNodePayout, types::AppState};
#[derive(Deserialize)]
pub struct CreateNodePayoutBody {
    pub amount_fen: Option<i64>,
    pub amount_credits: Option<f64>,
    pub payout_method: String,
    pub payout_account: String,
    pub contact: Option<String>,
}

/// GET /api/me/node-payouts — 当前用户的节点收益提现申请。
pub async fn my_node_payouts(
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
                .into_response()
        }
    };

    match state.store.list_node_payout_requests(&user.id, 50) {
        Ok(payouts) => Json(serde_json::json!({ "payouts": payouts })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// POST /api/me/node-payouts — 申请提现，申请成功后立即冻结节点可用余额。
pub async fn create_node_payout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateNodePayoutBody>,
) -> impl IntoResponse {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };
    let amount_fen = match payout_amount_fen(&body) {
        Some(amount) => amount,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "提现金额必须大于 0"})),
            )
                .into_response()
        }
    };
    let min_fen = node_payout_min_fen(&state);
    if amount_fen < min_fen {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("提现金额不能低于 ¥{:.2}", min_fen as f64 / 100.0),
                "payout_min_fen": min_fen
            })),
        )
            .into_response();
    }

    match state.store.create_node_payout_request(CreateNodePayout {
        provider_user_id: &user.id,
        amount_fen,
        payout_method: &body.payout_method,
        payout_account: &body.payout_account,
        contact: body.contact.as_deref(),
    }) {
        Ok(payout) => {
            let balance = state.store.get_node_balance(&user.id).unwrap_or(0.0);
            Json(serde_json::json!({ "ok": true, "payout": payout, "balance": balance }))
                .into_response()
        }
        Err(e) => node_payout_error_response(e),
    }
}

/// POST /api/me/node-payouts/:payout_id/cancel — 取消待处理提现并退回冻结余额。
pub async fn cancel_node_payout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(payout_id): Path<String>,
) -> impl IntoResponse {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };
    match state.store.cancel_node_payout_request(&user.id, &payout_id) {
        Ok(payout) => {
            let balance = state.store.get_node_balance(&user.id).unwrap_or(0.0);
            Json(serde_json::json!({ "ok": true, "payout": payout, "balance": balance }))
                .into_response()
        }
        Err(e) => node_payout_error_response(e),
    }
}

fn payout_amount_fen(body: &CreateNodePayoutBody) -> Option<i64> {
    if let Some(amount_fen) = body.amount_fen {
        return (amount_fen > 0).then_some(amount_fen);
    }
    let credits = body.amount_credits?;
    if !credits.is_finite() || credits <= 0.0 {
        return None;
    }
    Some((credits * 100.0).round() as i64)
}

pub(super) fn node_payout_min_fen(state: &AppState) -> i64 {
    state
        .store
        .billing_get_config("node_payout_min_fen")
        .ok()
        .flatten()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(100)
        .max(1)
}

fn node_payout_error_response(err: anyhow::Error) -> axum::response::Response {
    let msg = err.to_string();
    let status = if msg.contains("不存在") {
        StatusCode::NOT_FOUND
    } else if msg.contains("余额不足")
        || msg.contains("待处理")
        || msg.contains("不能为空")
        || msg.contains("必须大于")
    {
        StatusCode::BAD_REQUEST
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };
    (
        status,
        Json(serde_json::json!({
            "error": msg
        })),
    )
        .into_response()
}

// ── /api/nodes/chat ───────────────────────────────────────────────────────────
