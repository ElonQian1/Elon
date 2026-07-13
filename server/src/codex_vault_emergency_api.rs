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
    billing,
    codex_vault_api::{decrypt_auth_json, verify_node_proof},
    project_auth::{auth_from_headers, json_error},
    store::codex_vault_emergency::CodexVaultEmergencyLeaseCreate,
    store::{
        ActiveBillingReservation, CodexVaultEmergencyCredentialDeliveryClaim, NodeComputeRun, Store,
    },
    types::AppState,
};

#[path = "codex_vault_emergency_api_cleanup.rs"]
mod cleanup;

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
    /// The immutable accounting key of an already-running PC CLI request. A
    /// shared credential is not returned for this path until that run has a
    /// live balance hold and is rebound to the provider lease.
    pub compute_call_id: Option<String>,
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
        Ok(Some(cancel_targets)) => {
            let mut cancel_sent = 0usize;
            for (node_id, req_id) in &cancel_targets {
                if state
                    .agent_manager
                    .cancel_cli_prompt_on_agent(node_id, req_id)
                    .await
                {
                    cancel_sent += 1;
                } else {
                    tracing::warn!(
                        %node_id,
                        %req_id,
                        %grant_id,
                        "共享 Codex 授权已撤销，但在线任务取消消息未送达"
                    );
                }
            }
            Json(serde_json::json!({
                "ok": true,
                "cancel_targets": cancel_targets.len(),
                "cancel_sent": cancel_sent,
                "message": "已撤销 Codex 账号授权共享。",
            }))
            .into_response()
        }
        Ok(None) => json_error(StatusCode::NOT_FOUND, "没有可撤销的授权共享"),
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
    let midrun = match validate_midrun_switch(
        &state.store,
        &consumer.id,
        &req.agent_id,
        req.compute_call_id.as_deref(),
    )
    .await
    {
        Ok(run) => run,
        Err(error) => return json_error(StatusCode::CONFLICT, error),
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
    let reservation = match midrun.as_ref() {
        Some(run) => match reserve_midrun_shared_call(&state.store, run) {
            Ok(reservation) => Some(reservation),
            Err(error) => return json_error(StatusCode::PAYMENT_REQUIRED, error),
        },
        None => None,
    };
    let lease_input = || CodexVaultEmergencyLeaseCreate {
        grant_id: &grant.id,
        provider_user_id: &provider.id,
        consumer_user_id: &consumer.id,
        consumer_node_id: &req.agent_id,
        provider_slot_id: &slot.slot_id,
        account_hint_hash: slot.account_hint_hash.as_deref(),
        purpose: req.purpose.as_deref(),
        failure_reason: req.failure_reason.as_deref(),
        max_lease_seconds: grant.max_lease_seconds,
    };
    let lease_result = match (midrun.as_ref(), reservation.as_ref()) {
        (Some(run), Some(reservation)) => state
            .store
            .create_codex_vault_emergency_lease_for_run(
                lease_input(),
                run,
                &reservation.reservation_id,
            )
            .map(|issue| {
                issue.map(|issue| {
                    let deadline = issue
                        .run
                        .replay_deadline
                        .unwrap_or_else(|| issue.lease.expires_at.clone());
                    (issue.lease, deadline, issue.superseded_cancel_targets)
                })
            }),
        (None, None) => state
            .store
            .create_codex_vault_emergency_lease_with_superseded_runs(lease_input())
            .map(|issue| {
                let deadline = issue.lease.expires_at.clone();
                Some((issue.lease, deadline, issue.superseded_cancel_targets))
            }),
        _ => unreachable!("midrun shared lease and its reservation are created together"),
    };
    let lease = match lease_result {
        Ok(Some(issue)) => issue,
        Ok(None) => {
            release_midrun_reservation_if_unbound(&state.store, midrun.as_ref());
            let _ = state.store.record_codex_vault_event(
                &consumer.id,
                "emergency_lease_billing_bind_failed",
                Some(&req.agent_id),
                false,
                Some("运行中 PC 任务已被并发请求绑定"),
            );
            return json_error(
                StatusCode::CONFLICT,
                "运行中 PC 任务已经绑定了其他共享 Codex 租约，本次不会下发凭据",
            );
        }
        Err(error) => {
            release_midrun_reservation_if_unbound(&state.store, midrun.as_ref());
            let message = format!("创建并绑定共享 Codex 租约失败: {error}");
            let _ = state.store.record_codex_vault_event(
                &consumer.id,
                "emergency_lease_billing_bind_failed",
                Some(&req.agent_id),
                false,
                Some(&message),
            );
            return json_error(StatusCode::CONFLICT, message);
        }
    };
    let (lease, cloud_control_deadline, superseded_cancel_targets) = lease;
    for (node_id, req_id) in &superseded_cancel_targets {
        if !state
            .agent_manager
            .cancel_cli_prompt_on_agent(node_id, req_id)
            .await
        {
            tracing::warn!(
                %node_id,
                %req_id,
                lease_id = %lease.id,
                "共享 Codex 旧租约已被替换，但旧运行取消消息未送达"
            );
        }
    }
    // The lease/run transaction is the authorization boundary. These legacy
    // slot timestamps are advisory bookkeeping and must not turn a committed
    // credential dispatch into a client-visible failure.
    if let Err(error) = state.store.mark_user_codex_credential_slot_leased(
        &provider.id,
        &slot.slot_id,
        Some(&req.agent_id),
    ) {
        tracing::warn!(
            lease_id = %lease.id,
            slot_id = %slot.slot_id,
            error = %error,
            "共享 Codex 租约已提交，但槽位最近租用时间更新失败"
        );
    }
    let _ = state
        .store
        .mark_user_codex_credential_leased(&provider.id, Some(&req.agent_id));
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
    let cloud_control_window =
        match crate::homecli_agent::freeze_cloud_control_dispatch_window(&cloud_control_deadline) {
            Ok(window) => window,
            Err(error) => {
                tracing::warn!(
                    lease_id = %lease.id,
                    %error,
                    "共享 Codex 租约已提交，但授权窗口在凭据响应前已失效"
                );
                let reason = error.to_string();
                cleanup::cleanup_failed_credential_delivery(
                    &state,
                    &lease.id,
                    &consumer.id,
                    &req.agent_id,
                    midrun.as_ref(),
                    &reason,
                )
                .await;
                return json_error(StatusCode::CONFLICT, reason);
            }
        };
    let delivery_claim = CodexVaultEmergencyCredentialDeliveryClaim {
        lease_id: &lease.id,
        expected_lease_updated_at: &lease.updated_at,
        grant_id: &grant.id,
        provider_user_id: &provider.id,
        consumer_user_id: &consumer.id,
        consumer_node_id: &req.agent_id,
        provider_slot_id: &slot.slot_id,
        credential_version: slot.credential_version,
        compute_call_id: midrun.as_ref().map(|run| run.compute_call_id.as_str()),
        cloud_control_deadline: &cloud_control_deadline,
    };
    match state
        .store
        .claim_codex_vault_emergency_credential_delivery(delivery_claim)
    {
        Ok(true) => {}
        Ok(false) => {
            let reason = "共享 Codex 租约在凭据返回前已撤销、清除、过期或被替换";
            cleanup::cleanup_failed_credential_delivery(
                &state,
                &lease.id,
                &consumer.id,
                &req.agent_id,
                midrun.as_ref(),
                reason,
            )
            .await;
            return json_error(StatusCode::CONFLICT, reason);
        }
        Err(error) => {
            let reason = format!("共享 Codex 凭据返回前严格核验失败: {error}");
            cleanup::cleanup_failed_credential_delivery(
                &state,
                &lease.id,
                &consumer.id,
                &req.agent_id,
                midrun.as_ref(),
                &reason,
            )
            .await;
            return json_error(StatusCode::CONFLICT, reason);
        }
    }
    if let Some(run) = midrun.as_ref() {
        if let Some(req_id) = run
            .compute_call_id
            .strip_prefix("pc_agent_cli:")
            .filter(|req_id| !req_id.is_empty())
        {
            let Some(cancel_at) = tokio::time::Instant::now().checked_add(
                std::time::Duration::from_millis(cloud_control_window.ttl_ms),
            ) else {
                let reason = "共享 Codex 授权 TTL 超出服务器计时范围";
                cleanup::cleanup_failed_credential_delivery(
                    &state,
                    &lease.id,
                    &consumer.id,
                    &req.agent_id,
                    midrun.as_ref(),
                    reason,
                )
                .await;
                return json_error(StatusCode::CONFLICT, reason);
            };
            let agent_manager = Arc::clone(&state.agent_manager);
            let node_id = req.agent_id.clone();
            let req_id = req_id.to_string();
            let lease_id = lease.id.clone();
            tokio::spawn(async move {
                tokio::time::sleep_until(cancel_at).await;
                if !agent_manager
                    .cancel_cli_prompt_on_agent(&node_id, &req_id)
                    .await
                {
                    tracing::warn!(
                        %node_id,
                        %req_id,
                        %lease_id,
                        "共享 Codex 授权到期，但服务器取消消息未送达"
                    );
                }
            });
        }
    }
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
        "provider_avatar_data_url": provider.avatar_data_url,
        "consumer_user_id": consumer.id,
        "consumer_account": consumer.account,
        "consumer_nickname": consumer.nickname,
        "consumer_avatar_data_url": consumer.avatar_data_url,
        "consumer_node_id": req.agent_id,
        "device_name": req.device_name,
        "purpose": req.purpose,
        "billing_source": "shared_codex",
        "lease_expires_at": lease.expires_at,
        "cloud_control_deadline": cloud_control_deadline,
        "cloud_control_issued_at": cloud_control_window.issued_at,
        "cloud_control_ttl_ms": cloud_control_window.ttl_ms,
        "cleanup_recommended_seconds": grant.max_lease_seconds,
        "message": "已切换到授权机器人的共享 Codex 账号。",
    }))
    .into_response()
}

async fn validate_midrun_switch(
    store: &Store,
    consumer_user_id: &str,
    node_id: &str,
    compute_call_id: Option<&str>,
) -> Result<Option<NodeComputeRun>, String> {
    let Some(compute_call_id) = compute_call_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    // The node ACKs the prompt just before the cloud dispatch path persists its
    // compute run. A very fast auth failure can therefore reach this endpoint a
    // few milliseconds early; wait briefly instead of dropping a valid switch.
    let mut run = None;
    for attempt in 0..=20 {
        run = store
            .get_node_compute_run_by_compute_call_id(compute_call_id)
            .map_err(|error| format!("查询运行中 PC 任务失败: {error}"))?;
        if run.is_some() || attempt == 20 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let run = run.ok_or_else(|| "没有找到可绑定共享租约的运行中 PC 任务".to_string())?;
    if run.consumer_user_id != consumer_user_id
        || run.node_id != node_id
        || run.usage_mode != "pc_agent_cli"
        || run.status != "started"
    {
        return Err("共享 Codex 切换与当前用户、节点或运行中任务不匹配".to_string());
    }
    if run.billing_source != "own_codex" || run.offline_policy != "allow_offline" {
        return Err("只有从本机自有 Codex 发起的运行中任务可以切换共享账号".to_string());
    }
    Ok(Some(run))
}

fn reserve_midrun_shared_call(
    store: &Store,
    run: &NodeComputeRun,
) -> Result<ActiveBillingReservation, String> {
    let reservation_key = if run.feature.contains("dev") {
        "billing_cli_dev_reservation_fen"
    } else {
        "billing_cli_chat_reservation_fen"
    };
    let fallback = if run.feature.contains("dev") { 100 } else { 10 };
    let reserve_fen = billing::configured_reservation_fen(store, reservation_key, fallback);
    billing::reserve_trusted_call(
        store,
        &run.consumer_user_id,
        &run.compute_call_id,
        &run.feature,
        &run.usage_mode,
        run.model_id.as_deref(),
        reserve_fen,
    )?;
    store
        .get_active_billing_reservation(&run.consumer_user_id, &run.compute_call_id)
        .map_err(|error| format!("确认共享 Codex 计费预留失败: {error}"))?
        .ok_or_else(|| "共享 Codex 必须先完成在线计费预留，本次不会下发共享凭据".to_string())
}

fn release_midrun_reservation_if_unbound(store: &Store, run: Option<&NodeComputeRun>) {
    let Some(run) = run else {
        return;
    };
    // An idempotent reservation can be shared by racing requests. Never let a
    // losing request release the hold after the winner committed its lease.
    let still_unbound = store
        .get_node_compute_run_by_compute_call_id(&run.compute_call_id)
        .ok()
        .flatten()
        .is_some_and(|current| {
            current.status == "started"
                && current.consumer_user_id == run.consumer_user_id
                && current.node_id == run.node_id
                && current.billing_source == "own_codex"
                && current.offline_policy == "allow_offline"
                && current.lease_id.is_none()
        });
    if still_unbound {
        billing::release_trusted_call(
            store,
            &run.consumer_user_id,
            &run.compute_call_id,
            "released_error",
        );
    }
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
    match state
        .store
        .clear_codex_vault_emergency_lease_for_node_with_cancel_targets(
            &consumer.id,
            &req.agent_id,
            req.lease_id.as_deref(),
        ) {
        Ok(Some(issue)) => {
            let mut cancel_sent = 0usize;
            for (node_id, req_id) in &issue.cancel_targets {
                if state
                    .agent_manager
                    .cancel_cli_prompt_on_agent(node_id, req_id)
                    .await
                {
                    cancel_sent += 1;
                } else {
                    tracing::warn!(
                        %node_id,
                        %req_id,
                        lease_id = %issue.lease.id,
                        "共享 Codex 租约已清除，但关联运行取消消息未送达"
                    );
                }
            }
            Json(serde_json::json!({
                "ok": true,
                "cleared": true,
                "lease": issue.lease,
                "cancel_targets": issue.cancel_targets.len(),
                "cancel_sent": cancel_sent,
                "message": "已清除当前节点 Codex 账号共享租约。",
            }))
            .into_response()
        }
        Ok(None) => Json(serde_json::json!({
            "ok": true,
            "cleared": false,
            "message": "当前节点没有需要清除的 Codex 账号共享租约。",
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

#[cfg(test)]
#[path = "codex_vault_emergency_api_tests.rs"]
mod midrun_switch_tests;
