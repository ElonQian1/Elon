use super::*;

pub(super) fn status_from_record(
    record: Option<&CodexVaultRecord>,
    slots: &[CodexVaultSlotRecord],
) -> CodexVaultStatus {
    let active = slots.first();
    CodexVaultStatus {
        configured: master_key_configured(),
        bound: record.is_some() || !slots.is_empty(),
        active_slot_id: active.map(|r| r.slot_id.clone()),
        available_count: slots.iter().filter(|s| s.status != "deleted").count(),
        auth_mode: active
            .map(|r| r.auth_mode.clone())
            .or_else(|| record.map(|r| r.auth_mode.clone())),
        account_hint_hash: active
            .and_then(|r| r.account_hint_hash.clone())
            .or_else(|| record.and_then(|r| r.account_hint_hash.clone())),
        source_device: active
            .and_then(|r| r.source_device.clone())
            .or_else(|| record.and_then(|r| r.source_device.clone())),
        credential_version: active
            .map(|r| r.credential_version)
            .or_else(|| record.map(|r| r.credential_version)),
        last_backup_at: active
            .and_then(|r| r.last_backup_at.clone())
            .or_else(|| record.and_then(|r| r.last_backup_at.clone())),
        last_lease_at: active
            .and_then(|r| r.last_lease_at.clone())
            .or_else(|| record.and_then(|r| r.last_lease_at.clone())),
        updated_at: active
            .map(|r| r.updated_at.clone())
            .or_else(|| record.map(|r| r.updated_at.clone())),
        slots: slots
            .iter()
            .map(|slot| CodexVaultSlotStatus {
                slot_id: slot.slot_id.clone(),
                auth_mode: slot.auth_mode.clone(),
                account_hint_hash: slot.account_hint_hash.clone(),
                source_device: slot.source_device.clone(),
                credential_version: slot.credential_version,
                status: slot.status.clone(),
                failure_count: slot.failure_count,
                last_backup_at: slot.last_backup_at.clone(),
                last_lease_at: slot.last_lease_at.clone(),
                last_failure_at: slot.last_failure_at.clone(),
                last_error: slot.last_error.clone(),
                updated_at: slot.updated_at.clone(),
            })
            .collect(),
    }
}

pub(super) fn ensure_codex_usage_snapshot_access(
    state: &AppState,
    requester_user_id: &str,
    provider_user_id: &str,
) -> anyhow::Result<()> {
    if requester_user_id == provider_user_id {
        return Ok(());
    }
    let grants = state
        .store
        .list_codex_vault_emergency_grants(requester_user_id)?;
    let allowed = grants.iter().any(|grant| {
        grant.status == "active"
            && ((grant.provider_user_id == provider_user_id
                && grant.consumer_user_id == requester_user_id)
                || (grant.consumer_user_id == provider_user_id
                    && grant.provider_user_id == requester_user_id))
    });
    if allowed {
        Ok(())
    } else {
        anyhow::bail!("无权记录或查看该 Codex 账号的共享用量估算")
    }
}

pub(super) struct ValidatedAuthCache {
    pub(super) canonical_json: String,
    pub(super) auth_mode: String,
    pub(super) account_hint_hash: Option<String>,
}

pub(super) fn validate_auth_cache(value: &Value) -> Result<ValidatedAuthCache, String> {
    let canonical_json =
        serde_json::to_string(value).map_err(|_| "Codex 账号凭据不是有效 JSON".to_string())?;
    if canonical_json.len() > MAX_AUTH_JSON_BYTES {
        return Err("Codex 账号凭据过大".to_string());
    }
    let auth_mode = value
        .get("auth_mode")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|mode| !mode.is_empty())
        .unwrap_or("chatgpt")
        .to_string();
    if auth_mode != "chatgpt" {
        return Err("账号保险箱只支持 ChatGPT / Pro 登录态，不支持 API key 模式".to_string());
    }
    let refresh_token = value
        .pointer("/tokens/refresh_token")
        .or_else(|| value.get("refresh_token"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|token| token.len() >= 20);
    if refresh_token.is_none() {
        return Err("Codex 账号缺少可续期登录凭据，不能保存到账号保险箱".to_string());
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

pub(super) fn encrypt_auth_json(plaintext: &str) -> anyhow::Result<(String, String)> {
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

pub(crate) fn decrypt_auth_json(ciphertext_b64: &str, nonce_b64: &str) -> anyhow::Result<String> {
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

pub(super) fn vault_master_key() -> anyhow::Result<[u8; 32]> {
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

pub(super) fn master_key_configured() -> bool {
    read_master_secret().is_some()
}

pub(super) fn read_master_secret() -> Option<String> {
    std::iter::once(MASTER_KEY_ENV)
        .chain(FALLBACK_MASTER_KEY_ENVS)
        .find_map(|name| {
            std::env::var(name)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
}

pub(crate) fn verify_node_proof(
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

pub(super) fn hash_hint(value: &str) -> String {
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
