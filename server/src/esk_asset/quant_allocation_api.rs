use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error},
    types::AppState,
};

use super::{
    api::{domain_error, internal_error, require_paper_mode},
    parse_esk_amount,
    quant_allocation::{
        view, CancelEskQuantAllocationBody, CreateEskQuantAllocationBody,
        EskQuantAllocationListQuery, ESK_QUANT_CANCEL_CONFIRMATION, ESK_QUANT_REQUEST_CONFIRMATION,
    },
    service::validate_bounded_label,
    EskQuantAllocationInput, ESK_QUANT_RISK_DISCLOSURE_REVISION,
};

pub(crate) async fn list_my_requests(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<EskQuantAllocationListQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    match state
        .store
        .list_esk_quant_allocation_requests(&user.id, query.limit.clamp(1, 100))
    {
        Ok(records) => Json(json!({
            "schema": "yilong.esk.quant_allocation_request_list.v2",
            "simulated": true,
            "funds_moved": false,
            "position_created": false,
            "requests": records.into_iter().map(view).collect::<Vec<_>>(),
        }))
        .into_response(),
        Err(error) => internal_error("读取 ESK 量化 Paper 分配申请失败", error),
    }
}

pub(crate) async fn create_my_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateEskQuantAllocationBody>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    if let Some(response) = require_paper_mode() {
        return response;
    }
    if body.confirmation != ESK_QUANT_REQUEST_CONFIRMATION {
        return json_error(StatusCode::BAD_REQUEST, "量化 Paper 分配申请确认文本不匹配");
    }
    if body.risk_disclosure_revision != ESK_QUANT_RISK_DISCLOSURE_REVISION {
        return json_error(StatusCode::BAD_REQUEST, "量化 Paper 风险披露版本不匹配");
    }
    let amount_base_units = match parse_esk_amount(&body.amount) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    let idempotency_key = match validate_bounded_label(&body.idempotency_key, "幂等键", 160) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    let input = EskQuantAllocationInput {
        user_id: user.id,
        amount_base_units,
        idempotency_key,
        risk_disclosure_revision: body.risk_disclosure_revision,
    };
    match state.store.create_esk_quant_allocation_request(&input) {
        Ok(record) => (StatusCode::CREATED, Json(view(record))).into_response(),
        Err(error) => domain_error(error),
    }
}

pub(crate) async fn cancel_my_request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(body): Json<CancelEskQuantAllocationBody>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    if let Some(response) = require_paper_mode() {
        return response;
    }
    if body.confirmation != ESK_QUANT_CANCEL_CONFIRMATION {
        return json_error(
            StatusCode::BAD_REQUEST,
            "取消量化 Paper 分配申请确认文本不匹配",
        );
    }
    match state
        .store
        .cancel_esk_quant_allocation_request(&user.id, request_id.trim())
    {
        Ok(record) => Json(view(record)).into_response(),
        Err(error) => domain_error(error),
    }
}
