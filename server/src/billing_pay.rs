//! 微信支付路由处理器
//!
//! - POST /api/me/pay/create_order  — 用户端：创建订单，返回 App Pay 签名参数
//! - POST /api/pay/notify           — 微信回调（无用户鉴权，验签后充值）
//! - GET  /api/me/pay/orders        — 用户端：查询最近支付记录（可选）

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
    wechat_pay::{self, PayNotifyBody, WechatPayConfig},
};

// ── POST /api/me/pay/create_order ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateOrderBody {
    /// 充值金额（分）
    pub amount_fen: i64,
    /// 商品描述（可选，默认"一龙AI余额充值"）
    pub description: Option<String>,
}

pub async fn create_order(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateOrderBody>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };

    if body.amount_fen < 100 {
        return json_error(StatusCode::BAD_REQUEST, "最低充值金额为 1 元（100分）");
    }
    if body.amount_fen > 100_000_00 {
        return json_error(StatusCode::BAD_REQUEST, "单次充值上限 10000 元");
    }

    let cfg = match WechatPayConfig::from_env() {
        Some(c) => c,
        None => {
            return json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "微信支付未配置，请联系管理员",
            )
        }
    };

    // 生成商户订单号：取用户ID前8位 + 时间戳 + 随机6位
    let ts = wechat_pay::timestamp_secs();
    let nonce6 = &wechat_pay::new_nonce()[..6];
    let uid8 = &user.id.replace('-', "")[..8.min(user.id.replace('-', "").len())];
    let out_trade_no = format!("{uid8}{ts}{nonce6}");

    // 落库（幂等）
    if let Err(e) = state
        .store
        .pay_order_create(&out_trade_no, &user.id, body.amount_fen)
    {
        tracing::warn!("pay_order_create error: {e}");
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, "创建订单失败");
    }

    let desc = body.description.as_deref().unwrap_or("一龙AI余额充值");
    match wechat_pay::create_app_order(&cfg, &out_trade_no, body.amount_fen, desc).await {
        Ok(params) => Json(json!({
            "ok": true,
            "out_trade_no": out_trade_no,
            "pay_params": params,
        }))
        .into_response(),
        Err(e) => {
            tracing::warn!("create_app_order failed for user {}: {e}", user.id);
            json_error(StatusCode::BAD_GATEWAY, &format!("微信下单失败: {e}"))
        }
    }
}

// ── POST /api/pay/notify ──────────────────────────────────────────────────────
//
// 微信异步通知，必须在 5 秒内返回 {"code":"SUCCESS"} 或错误。
// 验签失败时返回 FAIL；成功处理后返回 SUCCESS（幂等）。

pub async fn pay_notify(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PayNotifyBody>,
) -> Response {
    // 仅处理支付成功通知，其他事件直接 ACK
    if body.event_type != "TRANSACTION.SUCCESS" {
        tracing::debug!("pay_notify: 忽略事件类型 {}", body.event_type);
        return Json(json!({"code": "SUCCESS", "message": "ignored"})).into_response();
    }

    let cfg = match WechatPayConfig::from_env() {
        Some(c) => c,
        None => {
            tracing::error!("pay_notify: 微信支付未配置");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"code": "FAIL", "message": "server misconfigured"})),
            )
                .into_response();
        }
    };

    let txn = match wechat_pay::parse_pay_notify(&cfg, &body) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("pay_notify: 解密/解析失败: {e}");
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"code": "FAIL", "message": format!("{e}")})),
            )
                .into_response();
        }
    };

    // 只处理 SUCCESS 状态
    if txn.trade_state != "SUCCESS" {
        tracing::debug!(
            "pay_notify: 订单 {} 状态 {} 不是 SUCCESS，跳过",
            txn.out_trade_no,
            txn.trade_state
        );
        return Json(json!({"code": "SUCCESS", "message": "non-success state ignored"}))
            .into_response();
    }

    let tx_id = txn.transaction_id.as_deref().unwrap_or(&txn.out_trade_no);

    match state.store.pay_order_complete(&txn.out_trade_no, tx_id) {
        Ok(()) => {
            tracing::info!(
                "pay_notify: 订单 {} 充值成功 {} 分",
                txn.out_trade_no,
                txn.amount.total
            );
            Json(json!({"code": "SUCCESS", "message": "ok"})).into_response()
        }
        Err(e) => {
            tracing::error!("pay_notify: pay_order_complete error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"code": "FAIL", "message": format!("{e}")})),
            )
                .into_response()
        }
    }
}

// ── GET /api/me/pay/orders ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct OrdersQuery {
    #[serde(default = "default_page")]
    page: i64,
    #[serde(default = "default_size")]
    size: i64,
}
fn default_page() -> i64 {
    1
}
fn default_size() -> i64 {
    20
}

pub async fn list_my_orders(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<OrdersQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };

    let size = q.size.clamp(1, 50);
    let offset = (q.page - 1).max(0) * size;

    let orders = state.store.pay_orders_list_user(&user.id, size, offset);
    match orders {
        Ok(rows) => Json(json!({ "page": q.page, "size": size, "orders": rows })).into_response(),
        Err(e) => {
            tracing::warn!("list_my_orders error: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询失败")
        }
    }
}
