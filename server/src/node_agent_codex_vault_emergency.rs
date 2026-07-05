//! 本机节点 Codex 保险箱授权共享桥接。
//!
//! PC 页面调用本机节点；节点再带用户 token 与节点 secret 向云端申请租约。
//! auth.json 只会落到节点托管的临时 CODEX_HOME。

use anyhow::{bail, Context, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

use crate::node_agent_codex_vault::{
    cloud_post_typed, local_status_payload, managed_slot_codex_home, validate_chatgpt_auth_cache,
    write_managed_auth_home, write_slot_meta, ManagedSlotMeta,
};

#[derive(Debug, Deserialize)]
pub(crate) struct EmergencyRestoreRequest {
    pub(crate) provider_user_id: Option<String>,
    pub(crate) provider_account: Option<String>,
    pub(crate) purpose: Option<String>,
    pub(crate) failure_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EmergencyLeaseResponse {
    pub(crate) auth_json: String,
    pub(crate) lease_id: Option<String>,
    pub(crate) slot_id: Option<String>,
    pub(crate) account_hint_hash: Option<String>,
    pub(crate) provider_user_id: Option<String>,
    pub(crate) provider_nickname: Option<String>,
    pub(crate) provider_account: Option<String>,
    pub(crate) billing_source: Option<String>,
    pub(crate) lease_expires_at: Option<String>,
    pub(crate) message: Option<String>,
}

pub(crate) fn routes() -> Router<Arc<crate::NodeRuntime>> {
    Router::new()
        .route(
            "/api/codex-vault/emergency-restore",
            post(emergency_restore_handler),
        )
        .route(
            "/api/codex-vault/sharing/restore",
            post(emergency_restore_handler),
        )
        .route(
            "/api/codex-vault/emergency-grants",
            post(create_grant_handler),
        )
        .route(
            "/api/codex-vault/sharing/grants",
            post(create_grant_handler),
        )
        .route(
            "/api/codex-vault/emergency-grants/:grant_id",
            delete(revoke_grant_handler),
        )
        .route(
            "/api/codex-vault/sharing/grants/:grant_id",
            delete(revoke_grant_handler),
        )
        .route(
            "/api/codex-vault/sharing/usage-snapshots",
            post(record_usage_snapshot_handler),
        )
}

async fn emergency_restore_handler(
    State(rt): State<Arc<crate::NodeRuntime>>,
    Json(req): Json<EmergencyRestoreRequest>,
) -> impl IntoResponse {
    match restore_emergency_from_cloud(&rt, req).await {
        Ok(lease) => (
            StatusCode::OK,
            Json(json!({
                "ok": true,
                "message": lease.message.unwrap_or_else(|| "已切换为授权机器人的临时 Codex Pro 会话。".to_string()),
                "lease_id": lease.lease_id,
                "slot_id": lease.slot_id,
                "account_hint_hash": lease.account_hint_hash,
                "provider_user_id": lease.provider_user_id,
                "provider_nickname": lease.provider_nickname,
                "billing_source": lease.billing_source,
                "lease_expires_at": lease.lease_expires_at,
                "local": local_status_payload(),
            })),
        ),
        Err(error) => error_response(StatusCode::BAD_GATEWAY, error.to_string()),
    }
}

async fn create_grant_handler(
    State(rt): State<Arc<crate::NodeRuntime>>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let token = match rt.user_token().await {
        Some(token) => token,
        None => return error_response(StatusCode::UNAUTHORIZED, "本机节点尚未绑定云端账号"),
    };
    let url = format!(
        "{}/api/me/codex-vault/sharing/grants",
        rt.cloud_http_url().trim_end_matches('/')
    );
    match cloud_post_typed::<Value>(&url, &token, &body).await {
        Ok(value) => (StatusCode::OK, Json(value)),
        Err(error) => error_response(StatusCode::BAD_GATEWAY, error.to_string()),
    }
}

async fn revoke_grant_handler(
    State(rt): State<Arc<crate::NodeRuntime>>,
    Path(grant_id): Path<String>,
) -> impl IntoResponse {
    let token = match rt.user_token().await {
        Some(token) => token,
        None => return error_response(StatusCode::UNAUTHORIZED, "本机节点尚未绑定云端账号"),
    };
    let url = format!(
        "{}/api/me/codex-vault/sharing/grants/{}",
        rt.cloud_http_url().trim_end_matches('/'),
        grant_id
    );
    match cloud_delete(&url, &token).await {
        Ok(value) => (StatusCode::OK, Json(value)),
        Err(error) => error_response(StatusCode::BAD_GATEWAY, error.to_string()),
    }
}

async fn record_usage_snapshot_handler(
    State(rt): State<Arc<crate::NodeRuntime>>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let token = match rt.user_token().await {
        Some(token) => token,
        None => return error_response(StatusCode::UNAUTHORIZED, "本机节点尚未绑定云端账号"),
    };
    let url = format!(
        "{}/api/me/codex-vault/sharing/usage-snapshots",
        rt.cloud_http_url().trim_end_matches('/')
    );
    match cloud_post_typed::<Value>(&url, &token, &body).await {
        Ok(value) => (StatusCode::OK, Json(value)),
        Err(error) => error_response(StatusCode::BAD_GATEWAY, error.to_string()),
    }
}

pub(crate) async fn restore_emergency_from_cloud(
    rt: &Arc<crate::NodeRuntime>,
    req: EmergencyRestoreRequest,
) -> Result<EmergencyLeaseResponse> {
    let creds = rt.creds().await.context("请先绑定本机节点账号")?;
    let token = creds
        .user_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .context("本机节点缺少云端登录 token，请重新绑定")?
        .to_string();
    let provider_user_id = req.provider_user_id.as_deref().map(str::trim);
    let provider_account = req.provider_account.as_deref().map(str::trim);
    if provider_user_id
        .or(provider_account)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        bail!("请指定授权提供方机器人账号");
    }
    let url = format!(
        "{}/api/me/codex-vault/sharing/lease",
        rt.cloud_http_url().trim_end_matches('/')
    );
    let body = json!({
        "provider_user_id": provider_user_id,
        "provider_account": provider_account,
        "agent_id": creds.agent_id,
        "agent_secret": creds.agent_secret,
        "device_name": crate::machine_label(),
        "purpose": req.purpose.unwrap_or_else(|| "pc_web_robot_shared_codex_cli".to_string()),
        "failure_reason": req.failure_reason,
    });
    let lease = cloud_post_typed::<EmergencyLeaseResponse>(&url, &token, &body).await?;
    let auth_value: Value =
        serde_json::from_str(&lease.auth_json).context("云端返回的 auth_json 不是有效 JSON")?;
    validate_chatgpt_auth_cache(&auth_value)
        .map_err(|error| anyhow::anyhow!("云端共享凭据校验失败: {error}"))?;

    let provider_key = lease
        .provider_user_id
        .as_deref()
        .or(lease.provider_account.as_deref())
        .unwrap_or("provider");
    let provider_slot = lease.slot_id.as_deref().unwrap_or("legacy");
    let local_slot_id = format!("shared-{provider_key}-{provider_slot}");
    let home = managed_slot_codex_home(&local_slot_id);
    write_managed_auth_home(&home, &lease.auth_json)?;
    write_slot_meta(
        &home,
        &ManagedSlotMeta {
            slot_id: local_slot_id,
            account_hint_hash: lease.account_hint_hash.clone(),
            lease_id: lease.lease_id.clone(),
            lease_expires_at: lease.lease_expires_at.clone(),
        },
    )?;
    std::env::set_var("CODEX_HOME", &home);
    rt.refresh_cli_probe_now().await;
    Ok(lease)
}

pub(crate) async fn clear_cloud_emergency_lease(
    rt: &Arc<crate::NodeRuntime>,
    meta: Option<&ManagedSlotMeta>,
) -> Value {
    let lease_id = meta.and_then(|meta| meta.lease_id.as_deref());
    if lease_id.is_none() {
        return json!({"attempted": false, "reason": "no_active_shared_lease"});
    }
    let creds = match rt.creds().await {
        Some(creds) => creds,
        None => return json!({"attempted": false, "reason": "node_not_bound"}),
    };
    let token = match creds
        .user_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(token) => token.to_string(),
        None => return json!({"attempted": false, "reason": "missing_user_token"}),
    };
    let url = format!(
        "{}/api/me/codex-vault/sharing/leases/clear",
        rt.cloud_http_url().trim_end_matches('/')
    );
    let body = json!({
        "lease_id": lease_id,
        "agent_id": creds.agent_id,
        "agent_secret": creds.agent_secret,
    });
    match cloud_post_typed::<Value>(&url, &token, &body).await {
        Ok(value) => json!({"attempted": true, "ok": true, "response": value}),
        Err(error) => json!({"attempted": true, "ok": false, "error": error.to_string()}),
    }
}

async fn cloud_delete(url: &str, token: &str) -> Result<Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap_or_default();
    let resp = client.delete(url).bearer_auth(token).send().await?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    let value: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({ "raw": text }));
    if !status.is_success() {
        let message = value
            .get("error")
            .or_else(|| value.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("云端请求失败");
        bail!("云端返回 {}: {}", status, message);
    }
    Ok(value)
}

fn error_response(status: StatusCode, message: impl ToString) -> (StatusCode, Json<Value>) {
    (
        status,
        Json(json!({
            "ok": false,
            "error": message.to_string(),
        })),
    )
}
