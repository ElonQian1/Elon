use anyhow::{anyhow, bail, Context, Result};
use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use super::{
    AuthCacheInspection, CodexVaultLocalStatus, ManagedAuthSlotInspection, ManagedSlotMeta,
    MAX_AUTH_JSON_BYTES,
};

pub(super) fn local_status() -> CodexVaultLocalStatus {
    let legacy_home = managed_codex_home();
    let active_codex_home = crate::node_agent_codex_vault_active::current_valid_codex_home_env();
    let active_home_managed = active_codex_home
        .as_ref()
        .map(PathBuf::from)
        .is_some_and(|path| path_in_managed_vault(&path));
    let active_home = active_codex_home.as_ref().map(PathBuf::from);
    let active_meta = active_home
        .as_ref()
        .filter(|path| active_home_managed && path.is_dir())
        .and_then(|path| read_slot_meta(path).ok().flatten());
    let managed_home = active_home
        .as_ref()
        .filter(|_| active_home_managed)
        .cloned()
        .unwrap_or_else(|| legacy_home.clone());
    CodexVaultLocalStatus {
        managed_home: managed_home.to_string_lossy().to_string(),
        active_codex_home,
        active_home_managed,
        active_slot_id: active_meta.as_ref().map(|meta| meta.slot_id.clone()),
        active_account_hint_hash: active_meta
            .as_ref()
            .and_then(|meta| meta.account_hint_hash.clone()),
        managed_auth: inspect_auth_home(&managed_home),
        default_auth: default_codex_home()
            .map(|home| inspect_auth_home(&home))
            .unwrap_or_else(|| AuthCacheInspection {
                present: false,
                path: None,
                auth_mode: None,
                has_refresh_token: false,
                account_hint_hash: None,
                problem: Some("无法定位默认用户目录".to_string()),
            }),
        managed_slots: inspect_managed_slots(active_meta.as_ref()),
    }
}

pub(super) fn source_auth_json_path() -> Result<PathBuf> {
    let candidates = [
        std::env::var("CODEX_HOME")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from),
        default_codex_home(),
    ];
    candidates
        .into_iter()
        .flatten()
        .map(|home| home.join("auth.json"))
        .find(|path| path.is_file())
        .ok_or_else(|| {
            anyhow!("没有检测到可保存的 Codex 账号；当前只支持 Codex ChatGPT / Pro 登录态。")
        })
}

pub(super) fn inspect_auth_home(home: &Path) -> AuthCacheInspection {
    let path = home.join("auth.json");
    if !path.exists() {
        return AuthCacheInspection {
            present: false,
            path: Some(path.to_string_lossy().to_string()),
            auth_mode: None,
            has_refresh_token: false,
            account_hint_hash: None,
            problem: None,
        };
    }
    match read_auth_json_value(&path).map(|value| {
        let auth_mode = value
            .get("auth_mode")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let has_refresh_token = refresh_token(&value).is_some();
        let account_hint_hash = account_hint_hash(&value);
        (auth_mode, has_refresh_token, account_hint_hash)
    }) {
        Ok((auth_mode, has_refresh_token, account_hint_hash)) => AuthCacheInspection {
            present: true,
            path: Some(path.to_string_lossy().to_string()),
            auth_mode,
            has_refresh_token,
            account_hint_hash,
            problem: None,
        },
        Err(error) => AuthCacheInspection {
            present: true,
            path: Some(path.to_string_lossy().to_string()),
            auth_mode: None,
            has_refresh_token: false,
            account_hint_hash: None,
            problem: Some(error.to_string()),
        },
    }
}

pub(super) fn read_auth_json_value(path: &Path) -> Result<Value> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("无法读取 Codex 登录文件 {}", path.display()))?;
    if metadata.len() > MAX_AUTH_JSON_BYTES {
        bail!("Codex 登录文件过大，已拒绝读取");
    }
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("无法读取 Codex 登录文件 {}", path.display()))?;
    serde_json::from_str(&text).context("Codex 登录文件不是有效 JSON")
}

pub(crate) fn validate_chatgpt_auth_cache(value: &Value) -> Result<()> {
    let auth_mode = value
        .get("auth_mode")
        .and_then(Value::as_str)
        .unwrap_or("chatgpt");
    if auth_mode != "chatgpt" {
        bail!("账号保险箱只支持 ChatGPT / Pro 登录态，不支持 API key 模式");
    }
    if refresh_token(value).is_none() {
        bail!("Codex 账号缺少可续期登录凭据，不能作为共享账号使用");
    }
    Ok(())
}

pub(super) fn refresh_token(value: &Value) -> Option<&str> {
    value
        .pointer("/tokens/refresh_token")
        .or_else(|| value.get("refresh_token"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|token| token.len() >= 20)
}

pub(super) fn account_hint_hash(value: &Value) -> Option<String> {
    value
        .pointer("/tokens/account_id")
        .or_else(|| value.get("account_id"))
        .and_then(Value::as_str)
        .map(|value| {
            let digest = Sha256::digest(value.trim().as_bytes());
            hex::encode(&digest[..8])
        })
}

pub(crate) fn write_managed_auth_home(home: &Path, auth_json: &str) -> Result<()> {
    safe_remove_managed_home(home)?;
    std::fs::create_dir_all(home)
        .with_context(|| format!("无法创建临时 CODEX_HOME {}", home.display()))?;
    let auth_path = home.join("auth.json");
    std::fs::write(&auth_path, auth_json)
        .with_context(|| format!("无法写入临时 Codex 登录文件 {}", auth_path.display()))?;
    tighten_permissions(home, &auth_path);
    Ok(())
}

pub(crate) fn write_slot_meta(home: &Path, meta: &ManagedSlotMeta) -> Result<()> {
    let meta_path = home.join("elon-codex-vault-slot.json");
    std::fs::write(&meta_path, serde_json::to_string_pretty(meta)?)
        .with_context(|| format!("无法写入保险箱槽位元数据 {}", meta_path.display()))?;
    tighten_permissions(home, &meta_path);
    Ok(())
}

pub(super) fn read_slot_meta(home: &Path) -> Result<Option<ManagedSlotMeta>> {
    let path = home.join("elon-codex-vault-slot.json");
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("无法读取保险箱槽位元数据 {}", path.display()))?;
    Ok(Some(serde_json::from_str(&text)?))
}

pub(super) fn active_managed_slot_meta() -> Option<ManagedSlotMeta> {
    let active = std::env::var("CODEX_HOME").ok().map(PathBuf::from)?;
    if !path_in_managed_vault(&active) {
        return None;
    }
    read_slot_meta(&active).ok().flatten()
}

pub(super) fn inspect_managed_slots(
    active_meta: Option<&ManagedSlotMeta>,
) -> Vec<ManagedAuthSlotInspection> {
    let slots_root = managed_slots_root();
    let Ok(entries) = std::fs::read_dir(&slots_root) else {
        return Vec::new();
    };
    let mut slots = entries
        .flatten()
        .filter_map(|entry| {
            let home = entry.path().join("codex-home");
            if !home.is_dir() {
                return None;
            }
            let meta = read_slot_meta(&home).ok().flatten();
            let slot_id = meta
                .as_ref()
                .map(|value| value.slot_id.clone())
                .or_else(|| entry.file_name().to_str().map(ToOwned::to_owned))?;
            let active = active_meta.is_some_and(|active| active.slot_id == slot_id);
            Some(ManagedAuthSlotInspection {
                slot_id,
                account_hint_hash: meta
                    .as_ref()
                    .and_then(|value| value.account_hint_hash.clone()),
                active,
                home: home.to_string_lossy().to_string(),
                auth: inspect_auth_home(&home),
            })
        })
        .collect::<Vec<_>>();
    slots.sort_by(|left, right| left.slot_id.cmp(&right.slot_id));
    slots
}

pub(super) fn safe_remove_managed_home(home: &Path) -> Result<()> {
    if !home.exists() {
        return Ok(());
    }
    let full = std::fs::canonicalize(home).unwrap_or_else(|_| home.to_path_buf());
    let marker = format!(
        "{}codex-vault{}",
        std::path::MAIN_SEPARATOR,
        std::path::MAIN_SEPARATOR
    );
    let full_text = full.to_string_lossy().to_ascii_lowercase();
    if !full_text.contains(&marker) || !full_text.ends_with("codex-home") {
        bail!("拒绝清理非保险箱托管目录: {}", full.display());
    }
    std::fs::remove_dir_all(&full)
        .with_context(|| format!("无法清理临时 CODEX_HOME {}", full.display()))
}

pub(super) fn safe_remove_all_managed_homes() -> Result<()> {
    safe_remove_managed_home(&managed_codex_home())?;
    let slots = managed_slots_root();
    if slots.exists() {
        let full = std::fs::canonicalize(&slots).unwrap_or_else(|_| slots.clone());
        if !path_in_managed_vault(&full) {
            bail!("拒绝清理非保险箱槽位目录: {}", slots.display());
        }
        std::fs::remove_dir_all(&full)
            .with_context(|| format!("无法清理保险箱槽位目录 {}", full.display()))?;
    }
    Ok(())
}

pub(super) fn tighten_permissions(home: &Path, auth_path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(home, std::fs::Permissions::from_mode(0o700));
        let _ = std::fs::set_permissions(auth_path, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(windows)]
    {
        let _ = (home, auth_path);
    }
}

pub(super) fn managed_codex_home() -> PathBuf {
    data_base_dir()
        .join("Elon")
        .join("codex-vault")
        .join("codex-home")
}

pub(super) fn managed_slots_root() -> PathBuf {
    data_base_dir()
        .join("Elon")
        .join("codex-vault")
        .join("slots")
}

pub(crate) fn managed_slot_codex_home(slot_id: &str) -> PathBuf {
    managed_slots_root()
        .join(safe_slot_id(slot_id))
        .join("codex-home")
}

pub(super) fn safe_slot_id(slot_id: &str) -> String {
    let safe = slot_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .take(80)
        .collect::<String>();
    if safe.is_empty() {
        "legacy".to_string()
    } else {
        safe
    }
}

pub(super) fn default_codex_home() -> Option<PathBuf> {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .map(|home| home.join(".codex"))
}

pub(super) fn data_base_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("LOCALAPPDATA")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
    } else {
        std::env::var("XDG_DATA_HOME")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|home| PathBuf::from(home).join(".local").join("share"))
            })
            .unwrap_or_else(std::env::temp_dir)
    }
}

pub(super) fn path_in_managed_vault(path: &Path) -> bool {
    let full = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let root = data_base_dir().join("Elon").join("codex-vault");
    let root = std::fs::canonicalize(&root).unwrap_or(root);
    full.starts_with(root)
}

pub(super) async fn cloud_status(rt: &Arc<crate::NodeRuntime>) -> Result<Value> {
    let token = rt
        .user_token()
        .await
        .ok_or_else(|| anyhow!("本机节点尚未绑定云端账号"))?;
    let url = format!(
        "{}/api/me/codex-vault/status",
        rt.cloud_http_url().trim_end_matches('/')
    );
    cloud_get(&url, &token).await
}

pub(super) async fn cloud_get(url: &str, token: &str) -> Result<Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_default();
    let resp = client.get(url).bearer_auth(token).send().await?;
    decode_cloud_response(resp).await
}

pub(super) async fn cloud_post(url: &str, token: &str, body: &Value) -> Result<Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap_or_default();
    let resp = client
        .post(url)
        .bearer_auth(token)
        .json(body)
        .send()
        .await?;
    decode_cloud_response(resp).await
}

pub(super) async fn cloud_delete(url: &str, token: &str) -> Result<Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap_or_default();
    let resp = client.delete(url).bearer_auth(token).send().await?;
    decode_cloud_response(resp).await
}

pub(crate) async fn cloud_post_typed<T: serde::de::DeserializeOwned>(
    url: &str,
    token: &str,
    body: &Value,
) -> Result<T> {
    let value = cloud_post(url, token, body).await?;
    serde_json::from_value(value).context("云端响应格式不正确")
}

pub(super) async fn decode_cloud_response(resp: reqwest::Response) -> Result<Value> {
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

pub(super) fn error_response(
    status: StatusCode,
    message: impl ToString,
) -> (StatusCode, Json<Value>) {
    (
        status,
        Json(json!({
            "ok": false,
            "error": message.to_string(),
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::{account_hint_hash, validate_chatgpt_auth_cache};
    use serde_json::json;

    #[test]
    fn local_auth_cache_validation_accepts_chatgpt_refresh_token() {
        let value = json!({
            "auth_mode": "chatgpt",
            "tokens": {
                "refresh_token": "refresh-token-long-enough",
                "account_id": "acct_123"
            }
        });
        validate_chatgpt_auth_cache(&value).expect("valid auth cache");
        assert!(account_hint_hash(&value).is_some());
    }

    #[test]
    fn local_auth_cache_validation_rejects_api_key_mode() {
        let value = json!({
            "auth_mode": "api_key",
            "tokens": { "refresh_token": "refresh-token-long-enough" }
        });
        assert!(validate_chatgpt_auth_cache(&value).is_err());
    }
}
