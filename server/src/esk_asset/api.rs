use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

use crate::{
    admin::check_auth,
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

use super::{format_esk_amount, parse_esk_amount};
use super::{
    model::{
        CancelSellbackBody, CreateSellbackBody, EskAssetMode, EskSellbackInput,
        PaperAllocationBody, SellbackListQuery, PAPER_ALLOCATION_CONFIRMATION,
        SELLBACK_CANCEL_CONFIRMATION,
    },
    service::{account_view, sellback_view, validate_bounded_label},
    EskAllocationInput,
};

pub(crate) async fn get_my_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    match state.store.esk_account_ledger(&user.id) {
        Ok(ledger) => Json(account_view(EskAssetMode::from_env(), ledger)).into_response(),
        Err(error) => internal_error("读取 ESK 资产账户失败", error),
    }
}

pub(crate) async fn list_my_sellback_requests(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<SellbackListQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    match state
        .store
        .list_esk_sellback_requests(&user.id, query.limit.clamp(1, 100))
    {
        Ok(records) => Json(json!({
            "schema": "yilong.esk.sellback_request_list.v1",
            "simulated": true,
            "funds_moved": false,
            "requests": records.into_iter().map(sellback_view).collect::<Vec<_>>(),
        }))
        .into_response(),
        Err(error) => internal_error("读取 ESK 卖回申请失败", error),
    }
}

pub(crate) async fn create_my_sellback_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateSellbackBody>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    if let Some(response) = require_paper_mode() {
        return response;
    }
    let amount_base_units = match parse_esk_amount(&body.amount) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    let idempotency_key = match validate_bounded_label(&body.idempotency_key, "幂等键", 160) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    let input = EskSellbackInput {
        user_id: user.id,
        amount_base_units,
        idempotency_key,
    };
    match state.store.create_esk_sellback_request(&input) {
        Ok(record) => (StatusCode::CREATED, Json(sellback_view(record))).into_response(),
        Err(error) => domain_error(error),
    }
}

pub(crate) async fn cancel_my_sellback_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(body): Json<CancelSellbackBody>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    if let Some(response) = require_paper_mode() {
        return response;
    }
    if body.confirmation != SELLBACK_CANCEL_CONFIRMATION {
        return json_error(StatusCode::BAD_REQUEST, "取消确认文本不匹配");
    }
    match state
        .store
        .cancel_esk_sellback_request(&user.id, request_id.trim())
    {
        Ok(record) => Json(sellback_view(record)).into_response(),
        Err(error) => domain_error(error),
    }
}

pub(crate) async fn create_paper_allocation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<PaperAllocationBody>,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }
    if let Some(response) = require_paper_mode() {
        return response;
    }
    if body.confirmation != PAPER_ALLOCATION_CONFIRMATION {
        return json_error(StatusCode::BAD_REQUEST, "ESK Paper 登记确认文本不匹配");
    }
    let amount_base_units = match parse_esk_amount(&body.amount) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    let user_id = match validate_bounded_label(&body.user_id, "用户 ID", 160) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    let reference = match validate_bounded_label(&body.reference, "登记引用", 240) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    let idempotency_key = match validate_bounded_label(&body.idempotency_key, "幂等键", 160) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    let input = EskAllocationInput {
        user_id,
        amount_base_units,
        reference,
        idempotency_key,
    };
    match state.store.create_esk_paper_allocation(&input) {
        Ok(receipt) => match state.store.esk_account_ledger(&input.user_id) {
            Ok(ledger) => (
                StatusCode::CREATED,
                Json(json!({
                    "schema": "yilong.esk.paper_allocation_receipt.v1",
                    "entry_id": receipt.entry_id,
                    "user_id": receipt.user_id,
                    "amount": format_esk_amount(receipt.amount_base_units),
                    "amount_base_units": receipt.amount_base_units.to_string(),
                    "reference": receipt.reference,
                    "created_at": receipt.created_at,
                    "replayed": receipt.replayed,
                    "simulated": true,
                    "funds_moved": false,
                    "account": account_view(EskAssetMode::Paper, ledger),
                })),
            )
                .into_response(),
            Err(error) => internal_error("ESK 登记后余额复核失败", error),
        },
        Err(error) => domain_error(error),
    }
}

pub(super) fn require_paper_mode() -> Option<Response> {
    match EskAssetMode::from_env() {
        EskAssetMode::Paper => None,
        EskAssetMode::Disabled => Some(json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "ESK Paper 资产写入尚未启用",
        )),
        EskAssetMode::Invalid => Some(json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "ESK 资产模式配置无效，写入已失败关闭",
        )),
    }
}

pub(super) fn domain_error(error: anyhow::Error) -> Response {
    let message = error.to_string();
    let status = if message.contains("幂等键") || message.contains("批次 ID 不能用于") {
        StatusCode::CONFLICT
    } else if message.contains("不存在") {
        StatusCode::NOT_FOUND
    } else if message.contains("超过")
        || message.contains("必须")
        || message.contains("无效")
        || message.contains("重复")
        || message.contains("不能取消")
    {
        StatusCode::BAD_REQUEST
    } else {
        tracing::warn!(error = %message, "ESK asset request failed");
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, "ESK 资产操作失败");
    };
    json_error(status, message)
}

pub(super) fn internal_error(context: &'static str, error: anyhow::Error) -> Response {
    tracing::warn!(error = %error, context, "ESK asset storage failed");
    json_error(StatusCode::INTERNAL_SERVER_ERROR, context)
}
