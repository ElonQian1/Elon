//! Codex 保险箱机器人授权共享 API。
//!
//! 这里允许 provider 账号显式授权 consumer 账号临时租用
//! provider 的保险箱槽位。云端只在通过 consumer 节点 secret 证明后
//! 返回一次性租约响应；浏览器状态接口不会包含 Codex 登录文件明文。

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    codex_vault_api::{decrypt_auth_json, verify_node_proof},
    project_auth::{auth_from_headers, json_error},
    store::codex_vault_emergency::CodexVaultEmergencyLeaseCreate,
    types::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateEmergencyGrantRequest {
    pub consumer_user_id: Option<String>,
    pub consumer_account: Option<String>,
    pub label: Option<String>,
    pub purpose: Option<String>,
    pub max_lease_seconds: Option<i64>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LeaseEmergencyAuthCacheRequest {
    pub provider_user_id: Option<String>,
    pub provider_account: Option<String>,
    pub agent_id: String,
    pub agent_secret: String,
    pub device_name: Option<String>,
    pub purpose: Option<String>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ClearEmergencyLeaseRequest {
    pub lease_id: Option<String>,
    pub agent_id: String,
    pub agent_secret: String,
}

pub async fn status(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    match (
        state.store.list_codex_vault_emergency_grants(&user.id),
        state.store.list_codex_vault_emergency_leases(&user.id, 50),
        state.store.codex_vault_sharing_health(&user.id),
    ) {
        (Ok(grants), Ok(leases), Ok(health)) => Json(serde_json::json!({
            "ok": true,
            "grants": grants,
            "leases": leases,
            "health": health,
        }))
        .into_response(),
        (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => {
            json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
        }
    }
}

pub async fn create_grant(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateEmergencyGrantRequest>,
) -> Response {
    let provider = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    let consumer_query = req
        .consumer_user_id
        .as_deref()
        .or(req.consumer_account.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(consumer_query) = consumer_query else {
        return json_error(StatusCode::BAD_REQUEST, "请填写被授权机器人账号");
    };
    let consumer = match state
        .store
        .resolve_codex_vault_emergency_user(consumer_query)
    {
        Ok(Some(user)) => user,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "没有找到这个机器人账号"),
        Err(error) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    };
    if consumer.id == provider.id {
        return json_error(StatusCode::BAD_REQUEST, "不能把保险箱授权共享给自己");
    }
    match state.store.upsert_codex_vault_emergency_grant(
        &provider.id,
        &consumer.id,
        req.label.as_deref(),
        req.purpose.as_deref(),
        req.max_lease_seconds,
        req.expires_at.as_deref(),
        &provider.id,
    ) {
        Ok(grant) => Json(serde_json::json!({
            "ok": true,
            "grant": grant,
            "message": "已保存 Codex 账号授权共享。",
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

pub async fn revoke_grant(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(grant_id): Path<String>,
) -> Response {
    let provider = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    match state
        .store
        .revoke_codex_vault_emergency_grant(&grant_id, &provider.id)
    {
        Ok(true) => Json(serde_json::json!({
            "ok": true,
            "message": "已撤销 Codex 账号授权共享。",
        }))
        .into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "没有可撤销的授权共享"),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

pub async fn lease_auth_cache(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<LeaseEmergencyAuthCacheRequest>,
) -> Response {
    let consumer = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    if let Err(error) = verify_node_proof(&state, &consumer.id, &req.agent_id, &req.agent_secret) {
        return json_error(StatusCode::FORBIDDEN, error.to_string());
    }
    let provider_query = req
        .provider_user_id
        .as_deref()
        .or(req.provider_account.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(provider_query) = provider_query else {
        return json_error(StatusCode::BAD_REQUEST, "请指定授权提供方机器人账号");
    };
    let provider = match state
        .store
        .resolve_codex_vault_emergency_user(provider_query)
    {
        Ok(Some(user)) => user,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "没有找到授权提供方账号"),
        Err(error) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    };
    if provider.id == consumer.id {
        return json_error(
            StatusCode::BAD_REQUEST,
            "自己的 Codex 账号请使用普通切换，不走授权共享租约",
        );
    }
    let grant = match state
        .store
        .find_active_codex_vault_emergency_grant(&provider.id, &consumer.id)
    {
        Ok(Some(grant)) => grant,
        Ok(None) => {
            return json_error(
                StatusCode::FORBIDDEN,
                "授权提供方尚未给当前机器人开启 Codex 账号授权共享",
            )
        }
        Err(error) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    };
    let slot = match state
        .store
        .select_user_codex_credential_slot(&provider.id, None)
    {
        Ok(Some(slot)) => slot,
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "授权提供方没有可用的 Codex 账号槽位")
        }
        Err(error) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    };
    let auth_json = match decrypt_auth_json(&slot.ciphertext_b64, &slot.nonce_b64) {
        Ok(text) => text,
        Err(error) => {
            let _ = state.store.record_codex_vault_event(
                &provider.id,
                "emergency_lease_decrypt_failed",
                Some(&req.agent_id),
                false,
                Some(&error.to_string()),
            );
            return json_error(StatusCode::SERVICE_UNAVAILABLE, error.to_string());
        }
    };
    if let Err(error) = state.store.mark_user_codex_credential_slot_leased(
        &provider.id,
        &slot.slot_id,
        Some(&req.agent_id),
    ) {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
    }
    let _ = state
        .store
        .mark_user_codex_credential_leased(&provider.id, Some(&req.agent_id));
    let lease =
        match state
            .store
            .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
                grant_id: &grant.id,
                provider_user_id: &provider.id,
                consumer_user_id: &consumer.id,
                consumer_node_id: &req.agent_id,
                provider_slot_id: &slot.slot_id,
                account_hint_hash: slot.account_hint_hash.as_deref(),
                purpose: req.purpose.as_deref(),
                failure_reason: req.failure_reason.as_deref(),
                max_lease_seconds: grant.max_lease_seconds,
            }) {
            Ok(lease) => lease,
            Err(error) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
        };
    let _ = state.store.record_codex_vault_event(
        &provider.id,
        "emergency_lease_provider",
        Some(&req.agent_id),
        true,
        None,
    );
    let _ = state.store.record_codex_vault_event(
        &consumer.id,
        "emergency_lease_consumer",
        Some(&req.agent_id),
        true,
        None,
    );
    Json(serde_json::json!({
        "ok": true,
        "lease_id": lease.id,
        "grant_id": grant.id,
        "slot_id": slot.slot_id,
        "auth_json": auth_json,
        "auth_mode": slot.auth_mode,
        "credential_version": slot.credential_version,
        "account_hint_hash": slot.account_hint_hash,
        "provider_user_id": provider.id,
        "provider_account": provider.account,
        "provider_nickname": provider.nickname,
        "consumer_user_id": consumer.id,
        "consumer_node_id": req.agent_id,
        "device_name": req.device_name,
        "purpose": req.purpose,
        "billing_source": "shared_codex",
        "lease_expires_at": lease.expires_at,
        "cleanup_recommended_seconds": grant.max_lease_seconds,
        "message": "已切换到授权机器人的共享 Codex 账号。",
    }))
    .into_response()
}

pub async fn clear_active_lease(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ClearEmergencyLeaseRequest>,
) -> Response {
    let consumer = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    if let Err(error) = verify_node_proof(&state, &consumer.id, &req.agent_id, &req.agent_secret) {
        return json_error(StatusCode::FORBIDDEN, error.to_string());
    }
    match state.store.clear_codex_vault_emergency_lease_for_node(
        &consumer.id,
        &req.agent_id,
        req.lease_id.as_deref(),
    ) {
        Ok(Some(lease)) => Json(serde_json::json!({
            "ok": true,
            "cleared": true,
            "lease": lease,
            "message": "已清除当前节点 Codex 账号共享租约。",
        }))
        .into_response(),
        Ok(None) => Json(serde_json::json!({
            "ok": true,
            "cleared": false,
            "message": "当前节点没有需要清除的 Codex 账号共享租约。",
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}
