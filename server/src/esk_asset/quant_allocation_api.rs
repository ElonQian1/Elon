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
        view, ApplyEskQuantAllocationReceiptBody, CancelEskQuantAllocationBody,
        CreateEskQuantAllocationBody, EskQuantAllocationListQuery, ESK_QUANT_CANCEL_CONFIRMATION,
        ESK_QUANT_REQUEST_CONFIRMATION,
    },
    service::validate_bounded_label,
    EskQuantAllocationInput, EskQuantAllocationReceiptInput, ESK_QUANT_RISK_DISCLOSURE_REVISION,
};
use crate::router::{
    quant_esk_allocation_receipt::{EskAllocationReceiptVerifier, ReceiptVerifierConfigError},
    quant_paper_access::{PaperGrantSigner, SignerConfigError},
    quant_paper_launch::esk_allocation_authorization_id,
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

pub(crate) async fn apply_my_receipt(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ApplyEskQuantAllocationReceiptBody>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    if body.receipt_token.trim() != body.receipt_token || body.receipt_token.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "量化签名回执格式无效");
    }
    let verifier = match EskAllocationReceiptVerifier::from_env() {
        Ok(value) => value,
        Err(ReceiptVerifierConfigError::Disabled) => {
            return json_error(StatusCode::SERVICE_UNAVAILABLE, "量化回执验签尚未配置")
        }
        Err(ReceiptVerifierConfigError::Invalid) => {
            return json_error(StatusCode::SERVICE_UNAVAILABLE, "量化回执验签配置无效")
        }
    };
    let receipt = match verifier.verify(&body.receipt_token, chrono::Utc::now().timestamp()) {
        Ok(value) => value,
        Err(()) => return json_error(StatusCode::BAD_REQUEST, "量化签名回执无效"),
    };
    let signer = match PaperGrantSigner::from_env() {
        Ok(value) => value,
        Err(SignerConfigError::Disabled) => {
            return json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "量化 Paper 身份映射尚未配置",
            )
        }
        Err(SignerConfigError::Invalid) => {
            return json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "量化 Paper 身份映射配置无效",
            )
        }
    };
    let participant_ref = match signer.participant_ref(&user.id) {
        Ok(value) => value,
        Err(()) => {
            return json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "暂时无法验证量化参与者身份",
            )
        }
    };
    if receipt.participant_ref != participant_ref
        || receipt.authorization_id != esk_allocation_authorization_id(&receipt.request_id)
    {
        return json_error(StatusCode::FORBIDDEN, "量化签名回执不属于当前账号或申请");
    }
    let amount_base_units = match receipt.amount_base_units.parse::<i64>() {
        Ok(value) if value > 0 => value,
        _ => return json_error(StatusCode::BAD_REQUEST, "量化签名回执金额无效"),
    };
    let input = EskQuantAllocationReceiptInput {
        user_id: user.id,
        participant_ref,
        request_id: receipt.request_id,
        amount_base_units,
        risk_disclosure_revision: ESK_QUANT_RISK_DISCLOSURE_REVISION.to_owned(),
        event: receipt.event,
        binding_id: receipt.binding_id,
        receipt_id: receipt.receipt_id,
        receipt_digest: receipt.receipt_digest,
        receipt_key_id: receipt.key_id,
        previous_receipt_digest: receipt.previous_receipt_digest,
        quant_binding_revision: receipt.binding_revision,
        occurred_at_unix: receipt.occurred_at_unix,
    };
    match state.store.apply_esk_quant_allocation_receipt(&input) {
        Ok(record) => Json(view(record)).into_response(),
        Err(error) => domain_error(error),
    }
}
