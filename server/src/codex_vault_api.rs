//! 用户 Codex Pro 登录态保险箱 API。
//!
//! 浏览器只拿到状态；上传/租用凭据要求本机节点提供 agent secret 证明，避免
//! PC 前端 JS 直接接触 `auth.json` 明文。

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error},
    store::CodexVaultRecord,
    types::AppState,
};

const MAX_AUTH_JSON_BYTES: usize = 256 * 1024;
const MASTER_KEY_ENV: &str = "CODEX_VAULT_MASTER_KEY";
const FALLBACK_MASTER_KEY_ENVS: [&str; 2] = ["USER_API_KEY_SECRET", "SECRET_KEY"];

#[derive(Debug, Deserialize)]
pub struct SaveCodexAuthCacheRequest {
    pub auth_json: Value,
    pub source_device: Option<String>,
    pub agent_id: Option<String>,
    pub agent_secret: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LeaseCodexAuthCacheRequest {
    pub agent_id: String,
    pub agent_secret: String,
    pub device_name: Option<String>,
    pub purpose: Option<String>,
}

#[derive(Debug, Serialize)]
struct CodexVaultStatus {
    configured: bool,
    bound: bool,
    auth_mode: Option<String>,
    account_hint_hash: Option<String>,
    source_device: Option<String>,
    credential_version: Option<i64>,
    last_backup_at: Option<String>,
    last_lease_at: Option<String>,
    updated_at: Option<String>,
}

pub async fn status(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    match state.store.get_user_codex_credential(&user.id) {
        Ok(record) => Json(serde_json::json!({
            "ok": true,
            "vault": status_from_record(record.as_ref()),
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

pub async fn save_auth_cache(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<SaveCodexAuthCacheRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    let agent_id = req.agent_id.as_deref().unwrap_or_default();
    let agent_secret = req.agent_secret.as_deref().unwrap_or_default();
    if let Err(error) = verify_node_proof(&state, &user.id, agent_id, agent_secret) {
        return json_error(StatusCode::FORBIDDEN, error.to_string());
    }

    let parsed = match validate_auth_cache(&req.auth_json) {
        Ok(parsed) => parsed,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    let (ciphertext_b64, nonce_b64) = match encrypt_auth_json(&parsed.canonical_json) {
        Ok(value) => value,
        Err(error) => return json_error(StatusCode::SERVICE_UNAVAILABLE, error.to_string()),
    };
    match state.store.upsert_user_codex_credential(
        &user.id,
        &parsed.auth_mode,
        parsed.account_hint_hash.as_deref(),
        req.source_device.as_deref(),
        &ciphertext_b64,
        &nonce_b64,
    ) {
        Ok(record) => Json(serde_json::json!({
            "ok": true,
            "vault": status_from_record(Some(&record)),
            "message": "Codex Pro 凭据已加密保存到保险箱。",
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

pub async fn delete_auth_cache(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    match state.store.delete_user_codex_credential(&user.id) {
        Ok(deleted) => Json(serde_json::json!({
            "ok": true,
            "deleted": deleted,
            "vault": status_from_record(None),
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

pub async fn lease_auth_cache(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<LeaseCodexAuthCacheRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error.to_string()),
    };
    if let Err(error) = verify_node_proof(&state, &user.id, &req.agent_id, &req.agent_secret) {
        let _ = state.store.record_codex_vault_event(
            &user.id,
            "lease_rejected",
            Some(&req.agent_id),
            false,
            Some(&error.to_string()),
        );
        return json_error(StatusCode::FORBIDDEN, error.to_string());
    }
    let record = match state.store.get_user_codex_credential(&user.id) {
        Ok(Some(record)) => record,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "尚未绑定 Codex Pro 凭据保险箱"),
        Err(error) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    };
    let auth_json = match decrypt_auth_json(&record.ciphertext_b64, &record.nonce_b64) {
        Ok(text) => text,
        Err(error) => {
            let _ = state.store.record_codex_vault_event(
                &user.id,
                "lease_decrypt_failed",
                Some(&req.agent_id),
                false,
                Some(&error.to_string()),
            );
            return json_error(StatusCode::SERVICE_UNAVAILABLE, error.to_string());
        }
    };
    if let Err(error) = state
        .store
        .mark_user_codex_credential_leased(&user.id, Some(&req.agent_id))
    {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
    }
    Json(serde_json::json!({
        "ok": true,
        "lease_id": format!("cvl_{}", uuid::Uuid::new_v4().simple()),
        "auth_json": auth_json,
        "auth_mode": record.auth_mode,
        "credential_version": record.credential_version,
        "account_hint_hash": record.account_hint_hash,
        "device_name": req.device_name,
        "purpose": req.purpose,
        "cleanup_recommended_seconds": 900,
        "message": "租用凭据只应写入本机节点管理的临时 CODEX_HOME，退出后必须清理。",
    }))
    .into_response()
}

fn status_from_record(record: Option<&CodexVaultRecord>) -> CodexVaultStatus {
    CodexVaultStatus {
        configured: master_key_configured(),
        bound: record.is_some(),
        auth_mode: record.map(|r| r.auth_mode.clone()),
        account_hint_hash: record.and_then(|r| r.account_hint_hash.clone()),
        source_device: record.and_then(|r| r.source_device.clone()),
        credential_version: record.map(|r| r.credential_version),
        last_backup_at: record.and_then(|r| r.last_backup_at.clone()),
        last_lease_at: record.and_then(|r| r.last_lease_at.clone()),
        updated_at: record.map(|r| r.updated_at.clone()),
    }
}

struct ValidatedAuthCache {
    canonical_json: String,
    auth_mode: String,
    account_hint_hash: Option<String>,
}

fn validate_auth_cache(value: &Value) -> Result<ValidatedAuthCache, String> {
    let canonical_json =
        serde_json::to_string(value).map_err(|_| "auth_json 不是有效 JSON".to_string())?;
    if canonical_json.len() > MAX_AUTH_JSON_BYTES {
        return Err("auth_json 过大".to_string());
    }
    let auth_mode = value
        .get("auth_mode")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|mode| !mode.is_empty())
        .unwrap_or("chatgpt")
        .to_string();
    if auth_mode != "chatgpt" {
        return Err("只支持 ChatGPT / Pro 登录态备份，不支持 API key 凭据".to_string());
    }
    let refresh_token = value
        .pointer("/tokens/refresh_token")
        .or_else(|| value.get("refresh_token"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|token| token.len() >= 20);
    if refresh_token.is_none() {
        return Err("auth_json 缺少 refresh_token，不能作为 Codex Pro 长期凭据备份".to_string());
    }
    let account_hint_hash = value
        .pointer("/tokens/account_id")
        .or_else(|| value.get("account_id"))
        .and_then(Value::as_str)
        .map(hash_hint);
    Ok(ValidatedAuthCache {
        canonical_json,
        auth_mode,
        account_hint_hash,
    })
}

fn encrypt_auth_json(plaintext: &str) -> anyhow::Result<(String, String)> {
    let key = vault_master_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| anyhow::anyhow!("Codex vault master key 无效"))?;
    let uuid = uuid::Uuid::new_v4();
    let nonce_bytes = &uuid.as_bytes()[..12];
    let nonce = Nonce::from_slice(nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| anyhow::anyhow!("Codex 凭据加密失败"))?;
    Ok((BASE64.encode(ciphertext), BASE64.encode(nonce_bytes)))
}

fn decrypt_auth_json(ciphertext_b64: &str, nonce_b64: &str) -> anyhow::Result<String> {
    let key = vault_master_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| anyhow::anyhow!("Codex vault master key 无效"))?;
    let ciphertext = BASE64
        .decode(ciphertext_b64)
        .map_err(|_| anyhow::anyhow!("Codex 凭据密文损坏"))?;
    let nonce_bytes = BASE64
        .decode(nonce_b64)
        .map_err(|_| anyhow::anyhow!("Codex 凭据 nonce 损坏"))?;
    if nonce_bytes.len() != 12 {
        anyhow::bail!("Codex 凭据 nonce 长度不正确");
    }
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
        .map_err(|_| anyhow::anyhow!("Codex 凭据解密失败"))?;
    String::from_utf8(plaintext).map_err(|_| anyhow::anyhow!("Codex 凭据不是 UTF-8 JSON"))
}

fn vault_master_key() -> anyhow::Result<[u8; 32]> {
    let raw = read_master_secret().ok_or_else(|| {
        anyhow::anyhow!(
            "服务器未配置 {MASTER_KEY_ENV} / USER_API_KEY_SECRET / SECRET_KEY，Codex 凭据保险箱不可用"
        )
    })?;
    let bytes = if raw.len() == 64 && raw.chars().all(|ch| ch.is_ascii_hexdigit()) {
        hex::decode(&raw).unwrap_or_default()
    } else {
        BASE64.decode(&raw).unwrap_or_default()
    };
    if bytes.len() == 32 {
        let mut key = [0_u8; 32];
        key.copy_from_slice(&bytes);
        return Ok(key);
    }
    if raw.len() < 16 {
        anyhow::bail!("{MASTER_KEY_ENV} 至少需要 16 个字符，建议使用 32 字节随机值");
    }
    let digest = Sha256::digest(raw.as_bytes());
    let mut key = [0_u8; 32];
    key.copy_from_slice(&digest);
    Ok(key)
}

fn master_key_configured() -> bool {
    read_master_secret().is_some()
}

fn read_master_secret() -> Option<String> {
    std::iter::once(MASTER_KEY_ENV)
        .chain(FALLBACK_MASTER_KEY_ENVS)
        .find_map(|name| {
            std::env::var(name)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
}

fn verify_node_proof(
    state: &AppState,
    user_id: &str,
    agent_id: &str,
    agent_secret: &str,
) -> anyhow::Result<()> {
    let agent_id = agent_id.trim();
    let agent_secret = agent_secret.trim();
    if agent_id.is_empty() || agent_secret.is_empty() {
        anyhow::bail!("节点凭证不能为空");
    }
    let owner = state
        .store
        .get_node_credential_owner(agent_id)?
        .ok_or_else(|| anyhow::anyhow!("节点凭证不存在"))?;
    if owner != user_id {
        anyhow::bail!("节点不属于当前用户");
    }
    let expected_hash = state
        .store
        .get_node_credential_hash(agent_id)?
        .ok_or_else(|| anyhow::anyhow!("节点凭证不存在"))?;
    let actual_hash = hex::encode(Sha256::digest(agent_secret.as_bytes()));
    if actual_hash != expected_hash {
        anyhow::bail!("节点 secret 不匹配");
    }
    Ok(())
}

fn hash_hint(value: &str) -> String {
    let digest = Sha256::digest(value.trim().as_bytes());
    hex::encode(&digest[..8])
}

#[cfg(test)]
mod tests {
    use super::validate_auth_cache;
    use serde_json::json;

    #[test]
    fn validate_auth_cache_requires_chatgpt_refresh_token() {
        let good = validate_auth_cache(&json!({
            "auth_mode": "chatgpt",
            "tokens": {
                "refresh_token": "refresh-token-long-enough",
                "account_id": "acct_123"
            }
        }))
        .expect("chatgpt auth cache should validate");
        assert_eq!(good.auth_mode, "chatgpt");
        assert!(good.account_hint_hash.is_some());

        assert!(validate_auth_cache(&json!({
            "auth_mode": "api_key",
            "tokens": { "refresh_token": "refresh-token-long-enough" }
        }))
        .is_err());
        assert!(validate_auth_cache(&json!({ "auth_mode": "chatgpt" })).is_err());
    }
}
