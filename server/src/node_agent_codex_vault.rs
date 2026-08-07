//! 本机节点 Codex 账号保险箱桥接。浏览器触发操作，但不接触登录文件
//! 明文：读取、上传、租用和落盘都由本机节点完成。

use anyhow::{anyhow, bail, Context, Result};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

const MAX_AUTH_JSON_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CodexVaultLocalStatus {
    pub managed_home: String,
    pub active_codex_home: Option<String>,
    pub active_home_managed: bool,
    pub active_slot_id: Option<String>,
    pub active_account_hint_hash: Option<String>,
    pub managed_auth: AuthCacheInspection,
    pub default_auth: AuthCacheInspection,
    pub managed_slots: Vec<ManagedAuthSlotInspection>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AuthCacheInspection {
    pub present: bool,
    pub path: Option<String>,
    pub auth_mode: Option<String>,
    pub has_refresh_token: bool,
    pub account_hint_hash: Option<String>,
    pub problem: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ManagedAuthSlotInspection {
    pub slot_id: String,
    pub account_hint_hash: Option<String>,
    pub active: bool,
    pub home: String,
    pub auth: AuthCacheInspection,
}

#[derive(Debug, Deserialize)]
struct CloudLeaseResponse {
    auth_json: String,
    lease_id: Option<String>,
    slot_id: Option<String>,
    account_hint_hash: Option<String>,
    #[serde(default)]
    cloud_control_deadline: Option<String>,
    #[serde(default)]
    cloud_control_issued_at: Option<String>,
    #[serde(default)]
    cloud_control_ttl_ms: Option<u64>,
    message: Option<String>,
}

pub(crate) struct CodexVaultAutoSwitchCandidate {
    pub(crate) message: String,
    pub(crate) cloud_control_deadline: Option<String>,
    pub(crate) cloud_control_issued_at: Option<String>,
    pub(crate) cloud_control_ttl_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ManagedSlotMeta {
    pub(crate) slot_id: String,
    pub(crate) account_hint_hash: Option<String>,
    pub(crate) lease_id: Option<String>,
    pub(crate) lease_expires_at: Option<String>,
}

pub(crate) fn routes() -> Router<Arc<crate::NodeRuntime>> {
    Router::new()
        .route("/api/codex-vault/status", get(status_handler))
        .route("/api/codex-vault/backup", post(backup_handler))
        .route("/api/codex-vault/restore", post(restore_handler))
        .route("/api/codex-vault/delete-cloud", post(delete_cloud_handler))
        .route("/api/codex-vault/clear", post(clear_handler))
        .merge(crate::node_agent_codex_vault_consent::routes())
        .merge(crate::node_agent_codex_vault_emergency::routes())
}

pub(crate) fn local_status_payload() -> Value {
    serde_json::to_value(local_status()).unwrap_or_else(|_| json!({}))
}

async fn status_handler(State(rt): State<Arc<crate::NodeRuntime>>) -> (StatusCode, Json<Value>) {
    let local = local_status();
    let cloud = match cloud_status(&rt).await {
        Ok(value) => value,
        Err(error) => json!({
            "ok": false,
            "error": error.to_string(),
        }),
    };
    (
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "local": local,
            "cloud": cloud,
        })),
    )
}

async fn backup_handler(
    State(rt): State<Arc<crate::NodeRuntime>>,
    Json(req): Json<crate::node_agent_codex_vault_consent::VaultOperationRequest>,
) -> impl IntoResponse {
    use crate::node_agent_codex_vault_consent::{BeginOperation, VaultOperation};
    let operation = match crate::node_agent_codex_vault_consent::begin_operation(
        VaultOperation::Backup,
        &req,
        None,
    ) {
        Ok(BeginOperation::Started(operation)) => operation,
        Ok(BeginOperation::Replay(status, body)) => return (status, body),
        Err(response) => return response,
    };
    let creds = match rt.creds().await {
        Some(creds) => creds,
        None => {
            operation.fail("node_not_bound");
            return error_response(StatusCode::UNAUTHORIZED, "请先绑定本机节点账号");
        }
    };
    let token = match creds
        .user_token
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        Some(token) => token.to_string(),
        None => {
            operation.fail("missing_cloud_token");
            return error_response(
                StatusCode::UNAUTHORIZED,
                "本机节点缺少云端登录 token，请重新绑定",
            );
        }
    };
    let auth_path = match source_auth_json_path() {
        Ok(path) => path,
        Err(error) => {
            operation.fail("source_auth_unavailable");
            return error_response(StatusCode::BAD_REQUEST, error.to_string());
        }
    };
    if path_in_managed_vault(&auth_path) {
        operation.fail("shared_account_backup_forbidden");
        return error_response(
            StatusCode::BAD_REQUEST,
            "当前使用共享 Codex 账号，不能覆盖云端。",
        );
    }
    let auth_json = match read_auth_json_value(&auth_path) {
        Ok(value) => value,
        Err(error) => {
            operation.fail("auth_cache_read_failed");
            return error_response(StatusCode::BAD_REQUEST, error.to_string());
        }
    };
    if let Err(error) = validate_chatgpt_auth_cache(&auth_json) {
        operation.fail("auth_cache_invalid");
        return error_response(StatusCode::BAD_REQUEST, error.to_string());
    }
    let body = json!({
        "auth_json": auth_json,
        "source_device": crate::machine_label(),
        "agent_id": creds.agent_id,
        "agent_secret": creds.agent_secret,
    });
    let url = format!(
        "{}/api/me/codex-vault/auth-cache",
        rt.cloud_http_url().trim_end_matches('/')
    );
    match cloud_post(&url, &token, &body).await {
        Ok(value) => {
            operation.complete();
            (
                StatusCode::OK,
                Json(json!({
                    "ok": true,
                    "message": "已把这台电脑的 Codex 账号加密保存到云端账号保险箱。",
                    "cloud": value,
                    "local": local_status(),
                })),
            )
        }
        Err(error) => {
            operation.fail("cloud_backup_failed");
            error_response(StatusCode::BAD_GATEWAY, error.to_string())
        }
    }
}

async fn restore_handler(
    State(rt): State<Arc<crate::NodeRuntime>>,
    Json(req): Json<crate::node_agent_codex_vault_consent::VaultOperationRequest>,
) -> impl IntoResponse {
    use crate::node_agent_codex_vault_consent::{BeginOperation, VaultOperation};
    let operation = match crate::node_agent_codex_vault_consent::begin_operation(
        VaultOperation::Restore,
        &req,
        None,
    ) {
        Ok(BeginOperation::Started(operation)) => operation,
        Ok(BeginOperation::Replay(status, body)) => return (status, body),
        Err(response) => return response,
    };
    match restore_from_cloud(
        &rt,
        req.purpose
            .unwrap_or_else(|| "temporary_local_codex_cli".to_string()),
        None,
        None,
    )
    .await
    {
        Ok(lease) => {
            operation.complete();
            (
                StatusCode::OK,
                Json(json!({
                    "ok": true,
                    "message": lease.message.unwrap_or_else(|| "已切换为本机临时 Codex 会话。".to_string()),
                    "lease_id": lease.lease_id,
                    "slot_id": lease.slot_id,
                    "account_hint_hash": lease.account_hint_hash,
                    "local": local_status(),
                })),
            )
        }
        Err(error) => {
            operation.fail("cloud_restore_failed");
            error_response(StatusCode::BAD_GATEWAY, error.to_string())
        }
    }
}

pub(crate) async fn try_auto_switch_after_codex_failure(
    rt: &Arc<crate::NodeRuntime>,
    stdout_text: &str,
    stderr_text: &str,
) -> Result<Option<CodexVaultAutoSwitchCandidate>> {
    let combined = format!("{stdout_text}\n{stderr_text}");
    let classified = crate::errors::classify_ai_error(&combined);
    if !matches!(
        classified.category,
        crate::errors::AiErrorCategory::Quota | crate::errors::AiErrorCategory::AuthConfig
    ) {
        return Ok(None);
    }
    let previous = active_managed_slot_meta().and_then(|meta| meta.account_hint_hash);
    let Some(previous_hint) = previous.as_deref() else {
        return Ok(None);
    };
    let reason = classified
        .operator_detail
        .as_deref()
        .unwrap_or(classified.code);
    let lease = match restore_from_cloud(
        rt,
        "auto_switch_after_codex_failure".to_string(),
        Some(previous_hint.to_string()),
        Some(reason.to_string()),
    )
    .await
    {
        Ok(lease) => lease,
        Err(error) => {
            tracing::warn!("Codex 保险箱自动切换失败: {error:#}");
            return Ok(None);
        }
    };
    Ok(owner_vault_auto_switch_candidate(previous_hint, lease))
}

fn owner_vault_auto_switch_candidate(
    previous_hint: &str,
    lease: CloudLeaseResponse,
) -> Option<CodexVaultAutoSwitchCandidate> {
    if lease.account_hint_hash.as_deref() == Some(previous_hint) {
        return None;
    }
    Some(CodexVaultAutoSwitchCandidate {
        message: format!(
            "Codex 当前账号额度或认证不可用，已自动切换到保险箱备用账号{}，正在重试本轮任务。",
            lease
                .account_hint_hash
                .as_deref()
                .map(|hint| format!(" ({hint})"))
                .unwrap_or_default()
        ),
        cloud_control_deadline: lease.cloud_control_deadline,
        cloud_control_issued_at: lease.cloud_control_issued_at,
        cloud_control_ttl_ms: lease.cloud_control_ttl_ms,
    })
}

async fn restore_from_cloud(
    rt: &Arc<crate::NodeRuntime>,
    purpose: String,
    previous_account_hint_hash: Option<String>,
    failure_reason: Option<String>,
) -> Result<CloudLeaseResponse> {
    let creds = match rt.creds().await {
        Some(creds) => creds,
        None => bail!("请先绑定本机节点账号"),
    };
    let token = match creds
        .user_token
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        Some(token) => token.to_string(),
        None => bail!("本机节点缺少云端登录 token，请重新绑定"),
    };
    let url = format!(
        "{}/api/me/codex-vault/lease",
        rt.cloud_http_url().trim_end_matches('/')
    );
    let body = json!({
        "agent_id": creds.agent_id,
        "agent_secret": creds.agent_secret,
        "device_name": crate::machine_label(),
        "purpose": purpose,
        "previous_account_hint_hash": previous_account_hint_hash,
        "failure_reason": failure_reason,
    });
    let lease = match cloud_post_typed::<CloudLeaseResponse>(&url, &token, &body).await {
        Ok(lease) => lease,
        Err(error) => bail!(error),
    };
    let auth_value: Value = match serde_json::from_str(&lease.auth_json) {
        Ok(value) => value,
        Err(_) => bail!("云端返回的 Codex 账号凭据不是有效 JSON"),
    };
    if let Err(error) = validate_chatgpt_auth_cache(&auth_value) {
        bail!("云端 Codex 账号校验失败: {error}");
    }
    let slot_id = lease.slot_id.as_deref().unwrap_or("legacy");
    let home = managed_slot_codex_home(slot_id);
    if let Err(error) = write_managed_auth_home(&home, &lease.auth_json) {
        bail!(error);
    }
    write_slot_meta(
        &home,
        &ManagedSlotMeta {
            slot_id: slot_id.to_string(),
            account_hint_hash: lease.account_hint_hash.clone(),
            // The owner's legacy vault endpoint issues an opaque request id,
            // not a revocable/expiring shared lease. Only emergency sharing
            // metadata may carry lease_id + lease_expires_at.
            lease_id: None,
            lease_expires_at: None,
        },
    )?;
    std::env::set_var("CODEX_HOME", &home);
    rt.refresh_cli_probe_now().await;
    Ok(lease)
}

async fn clear_handler(
    State(rt): State<Arc<crate::NodeRuntime>>,
    Json(req): Json<crate::node_agent_codex_vault_consent::VaultOperationRequest>,
) -> impl IntoResponse {
    use crate::node_agent_codex_vault_consent::{BeginOperation, VaultOperation};
    let operation = match crate::node_agent_codex_vault_consent::begin_operation(
        VaultOperation::ClearLocal,
        &req,
        None,
    ) {
        Ok(BeginOperation::Started(operation)) => operation,
        Ok(BeginOperation::Replay(status, body)) => return (status, body),
        Err(response) => return response,
    };
    let active = std::env::var("CODEX_HOME").ok().map(PathBuf::from);
    let active_meta = active
        .as_ref()
        .filter(|path| path_in_managed_vault(path))
        .and_then(|path| read_slot_meta(path).ok().flatten());
    let cloud_clear = crate::node_agent_codex_vault_emergency::clear_cloud_emergency_lease(
        &rt,
        active_meta.as_ref(),
    )
    .await;
    if let Err(error) = safe_remove_all_managed_homes() {
        operation.fail("managed_home_cleanup_failed");
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
    }
    if active
        .as_ref()
        .is_some_and(|path| path_in_managed_vault(path))
    {
        std::env::remove_var("CODEX_HOME");
    }
    rt.refresh_cli_probe_now().await;
    operation.complete();
    (
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "message": "已清理本机保险箱临时 CODEX_HOME。",
            "cloud_clear": cloud_clear,
            "local": local_status(),
        })),
    )
}

async fn delete_cloud_handler(
    State(rt): State<Arc<crate::NodeRuntime>>,
    Json(req): Json<crate::node_agent_codex_vault_consent::VaultOperationRequest>,
) -> impl IntoResponse {
    use crate::node_agent_codex_vault_consent::{BeginOperation, VaultOperation};
    let operation = match crate::node_agent_codex_vault_consent::begin_operation(
        VaultOperation::DeleteCloud,
        &req,
        None,
    ) {
        Ok(BeginOperation::Started(operation)) => operation,
        Ok(BeginOperation::Replay(status, body)) => return (status, body),
        Err(response) => return response,
    };
    let creds = match rt.creds().await {
        Some(creds) => creds,
        None => {
            operation.fail("node_not_bound");
            return error_response(StatusCode::UNAUTHORIZED, "请先绑定本机节点账号");
        }
    };
    let token = match creds
        .user_token
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        Some(token) => token.to_string(),
        None => {
            operation.fail("missing_cloud_token");
            return error_response(
                StatusCode::UNAUTHORIZED,
                "本机节点缺少云端登录 token，请重新绑定",
            );
        }
    };
    let url = format!(
        "{}/api/me/codex-vault/auth-cache",
        rt.cloud_http_url().trim_end_matches('/')
    );
    match cloud_delete(&url, &token).await {
        Ok(value) => {
            operation.complete();
            (
                StatusCode::OK,
                Json(json!({
                    "ok": true,
                    "message": "已删除云端 Codex 账号记录。",
                    "cloud": value,
                    "local": local_status(),
                })),
            )
        }
        Err(error) => {
            operation.fail("cloud_delete_failed");
            error_response(StatusCode::BAD_GATEWAY, error.to_string())
        }
    }
}

mod local_helpers;
use self::local_helpers::*;
pub(crate) use self::local_helpers::{
    cloud_post_typed, managed_slot_codex_home, validate_chatgpt_auth_cache,
    write_managed_auth_home, write_slot_meta,
};

#[cfg(test)]
mod tests {
    use super::{owner_vault_auto_switch_candidate, CloudLeaseResponse};

    #[test]
    fn owner_vault_auto_switch_carries_signed_cloud_window() {
        let candidate = owner_vault_auto_switch_candidate(
            "old-account",
            CloudLeaseResponse {
                auth_json: "{}".to_string(),
                lease_id: Some("opaque-request".to_string()),
                slot_id: Some("slot-new".to_string()),
                account_hint_hash: Some("new-account".to_string()),
                cloud_control_deadline: Some("2030-01-01T00:15:00Z".to_string()),
                cloud_control_issued_at: Some("2030-01-01T00:00:00Z".to_string()),
                cloud_control_ttl_ms: Some(900_000),
                message: None,
            },
        )
        .unwrap();

        assert_eq!(
            candidate.cloud_control_deadline.as_deref(),
            Some("2030-01-01T00:15:00Z")
        );
        assert_eq!(
            candidate.cloud_control_issued_at.as_deref(),
            Some("2030-01-01T00:00:00Z")
        );
        assert_eq!(candidate.cloud_control_ttl_ms, Some(900_000));
    }

    #[test]
    fn legacy_owner_vault_response_defaults_cloud_window_to_missing() {
        let lease: CloudLeaseResponse = serde_json::from_str(
            r#"{"auth_json":"{}","slot_id":"slot-new","account_hint_hash":"new-account"}"#,
        )
        .unwrap();

        assert!(lease.cloud_control_deadline.is_none());
        assert!(lease.cloud_control_issued_at.is_none());
        assert!(lease.cloud_control_ttl_ms.is_none());
    }
}
