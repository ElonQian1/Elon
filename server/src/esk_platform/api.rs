use axum::{
    body::Body,
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Path, Query, State,
    },
    http::{header, HeaderMap, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::{
    esk_asset::format_esk_amount,
    project_auth::{bearer_token, json_error},
    store::PublicUser,
    types::AppState,
};

use super::{model::*, validation};

fn current_policy() -> anyhow::Result<PlatformPolicy> {
    #[cfg(test)]
    if let Some(value) = super::http_tests::policy_override() {
        return value;
    }
    validation::load_policy()
}

pub(super) async fn private_no_store(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    response
        .headers_mut()
        .insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    response
}

pub(super) fn real_user<'a>(
    state: &AppState,
    headers: &'a HeaderMap,
) -> Result<(PublicUser, &'a str), Response> {
    if headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
        return Err(json_error(StatusCode::UNAUTHORIZED, "需要真实用户登录"));
    }
    let token = bearer_token(headers)
        .filter(|token| token.len() <= 8192)
        .ok_or_else(|| json_error(StatusCode::UNAUTHORIZED, "需要真实用户登录"))?;
    if state.owner_token.as_deref() == Some(token) || state.admin_token == token {
        return Err(json_error(
            StatusCode::UNAUTHORIZED,
            "不接受静态管理员或虚拟账户凭据",
        ));
    }
    let user = state
        .store
        .authenticate_token(token)
        .map_err(|_| json_error(StatusCode::UNAUTHORIZED, "登录无效或已过期"))?;
    state
        .store
        .validate_esk_platform_session(&user.id, token)
        .map_err(|_| json_error(StatusCode::UNAUTHORIZED, "登录无效或已过期"))?;
    if user.id == "local-owner" || user.status != "active" {
        return Err(json_error(StatusCode::UNAUTHORIZED, "需要真实有效账户"));
    }
    Ok((user, token))
}

fn administrator<'a>(
    state: &AppState,
    headers: &'a HeaderMap,
) -> Result<(PublicUser, &'a str), Response> {
    let (user, token) = real_user(state, headers)?;
    if !matches!(user.role.as_str(), "admin" | "owner") {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "仅平台管理员可审核正式登记",
        ));
    }
    Ok((user, token))
}

pub(super) async fn prepare_allocation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<PrepareBody>, JsonRejection>,
) -> Response {
    let (actor, token) = match administrator(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let policy = match current_policy() {
        Ok(value) => value,
        Err(error) => return domain_error(error),
    };
    let body = match body {
        Ok(Json(value)) => value,
        Err(_) => return domain_error(PlatformError::InvalidInput.into()),
    };
    let input = match validation::prepare_input(&policy, body) {
        Ok(value) => value,
        Err(error) => return domain_error(error),
    };
    match state
        .store
        .prepare_esk_platform_allocation(&policy, &input, &actor.id, token)
    {
        Ok(record) => allocation_response(record),
        Err(error) => domain_error(error),
    }
}

pub(super) async fn record_allocation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(allocation_id): Path<String>,
    body: Result<Json<RecordBody>, JsonRejection>,
) -> Response {
    let (actor, token) = match administrator(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let policy = match current_policy() {
        Ok(value) => value,
        Err(error) => return domain_error(error),
    };
    let body = match body {
        Ok(Json(value)) => value,
        Err(_) => return domain_error(PlatformError::InvalidInput.into()),
    };
    if body.confirmation != RECORD_CONFIRMATION {
        return domain_error(PlatformError::InvalidInput.into());
    }
    match state.store.record_esk_platform_allocation(
        &policy,
        &allocation_id,
        &body.expected_request_digest,
        &actor.id,
        token,
    ) {
        Ok(record) => allocation_response(record),
        Err(error) => domain_error(error),
    }
}

pub(super) async fn cancel_allocation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(allocation_id): Path<String>,
    body: Result<Json<RecordBody>, JsonRejection>,
) -> Response {
    let (actor, token) = match administrator(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let policy = match current_policy() {
        Ok(value) => value,
        Err(error) => return domain_error(error),
    };
    let body = match body {
        Ok(Json(value)) => value,
        Err(_) => return domain_error(PlatformError::InvalidInput.into()),
    };
    if body.confirmation != CANCEL_CONFIRMATION {
        return domain_error(PlatformError::InvalidInput.into());
    }
    match state.store.cancel_esk_platform_allocation(
        &policy,
        &allocation_id,
        &body.expected_request_digest,
        &actor.id,
        token,
    ) {
        Ok(record) => allocation_response(record),
        Err(error) => domain_error(error),
    }
}

fn allocation_response(record: PlatformAllocationRecord) -> Response {
    let recorded = record.recorded_at.is_some();
    let status = if recorded {
        "recorded"
    } else if record.canceled_at.is_some() {
        "canceled"
    } else {
        "prepared"
    };
    Json(json!({
        "schema": "yilong.esk.platform_allocation_receipt.v1",
        "allocation_id": record.allocation_id,
        "request_digest": record.input.request_digest,
        "user_id": record.input.user_id,
        "amount": format_esk_amount(record.input.amount_base_units),
        "amount_base_units": record.input.amount_base_units.to_string(),
        "payment_key": record.input.payment_key,
        "policy_digest": record.input.policy_digest,
        "status": status,
        "prepared_by": record.prepared_by,
        "prepared_at": record.prepared_at,
        "recorded_at": record.recorded_at,
        "canceled_at": record.canceled_at,
        "replayed": record.replayed,
        "allocation_recorded": recorded,
        "balance_written": recorded && !record.replayed,
        "source": "platform_recorded",
        "chain_status": "not_deployed",
        "simulated": false,
        "funds_moved": false,
        "external_payment_verified": false,
        "verification_basis": if recorded { "authenticated_operator_review" } else { "operator_supplied_materials" },
    })).into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AccountQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    20
}

pub(super) async fn get_my_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<AccountQuery>, QueryRejection>,
) -> Response {
    let (user, token) = match real_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let query = match query {
        Ok(Query(value)) => value,
        Err(_) => return domain_error(PlatformError::InvalidInput.into()),
    };
    if !(1..=100).contains(&query.limit) {
        return domain_error(PlatformError::InvalidInput.into());
    }
    match state
        .store
        .esk_platform_account(&user.id, token, query.limit)
    {
        Ok(account) => Json(json!({
            "schema": "yilong.esk.platform_account.v1",
            "asset_id": "esk", "symbol": "ESK", "decimals": 6,
            "source": "platform_recorded", "chain_status": "not_deployed",
            "simulated": false, "funds_moved": false,
            "verification_basis": "authenticated_operator_review",
            "external_payment_verified": false,
            "total": format_esk_amount(account.total_base_units),
            "total_base_units": account.total_base_units.to_string(),
            "entry_count": account.entry_count.to_string(),
            "updated_at": account.updated_at,
            "history_has_more": account.entry_count > account.entries.len() as i64,
            "entries": account.entries.into_iter().map(|entry| json!({
                "entry_id": entry.entry_id,
                "allocation_id": entry.allocation_id,
                "amount": format_esk_amount(entry.amount_base_units),
                "amount_base_units": entry.amount_base_units.to_string(),
                "created_at": entry.created_at,
                "kind": "approved_payment_allocation",
            })).collect::<Vec<_>>(),
            "capabilities": {
                "service_spending": false, "quant_subscription": false,
                "sellback_settlement": false, "onchain_transfer": false,
                "chain_migration": false,
            },
            "status_message": "经管理员审核的 ESK 平台登记；未上链，不代表可提现、固定价格或已产生收益。模拟余额另行显示。",
        })).into_response(),
        Err(error) => domain_error(error),
    }
}

pub(super) fn domain_error(error: anyhow::Error) -> Response {
    let Some(kind) = error.downcast_ref::<PlatformError>() else {
        // SQL errors may carry data; never echo or log their text.
        tracing::warn!(
            code = "ESK_PLATFORM_STORAGE_ERROR",
            "ESK platform operation failed"
        );
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ESK_PLATFORM_STORAGE_ERROR",
        );
    };
    let status = match kind {
        PlatformError::Disabled | PlatformError::InvalidPolicy => StatusCode::SERVICE_UNAVAILABLE,
        PlatformError::Unauthorized => StatusCode::FORBIDDEN,
        PlatformError::UserUnavailable | PlatformError::NotFound => StatusCode::NOT_FOUND,
        PlatformError::Conflict
        | PlatformError::PolicyChanged
        | PlatformError::HistoryChanged
        | PlatformError::LimitExceeded => StatusCode::CONFLICT,
        PlatformError::InvalidInput => StatusCode::BAD_REQUEST,
        PlatformError::CorruptLedger => StatusCode::INTERNAL_SERVER_ERROR,
    };
    json_error(status, kind)
}
