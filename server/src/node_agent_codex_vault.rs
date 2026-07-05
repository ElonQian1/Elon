//! 本机节点 Codex Pro 凭据保险箱桥接。浏览器触发操作，但不接触 `auth.json`
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
struct RestoreRequest {
    purpose: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CloudLeaseResponse {
    auth_json: String,
    lease_id: Option<String>,
    slot_id: Option<String>,
    account_hint_hash: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ManagedSlotMeta { pub(crate) slot_id: String, pub(crate) account_hint_hash: Option<String>, pub(crate) lease_id: Option<String>, pub(crate) lease_expires_at: Option<String> }

pub(crate) fn routes() -> Router<Arc<crate::NodeRuntime>> {
    Router::new()
        .route("/api/codex-vault/status", get(status_handler))
        .route("/api/codex-vault/backup", post(backup_handler))
        .route("/api/codex-vault/restore", post(restore_handler))
        .route("/api/codex-vault/delete-cloud", post(delete_cloud_handler))
        .route("/api/codex-vault/clear", post(clear_handler))
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

async fn backup_handler(State(rt): State<Arc<crate::NodeRuntime>>) -> impl IntoResponse {
    let creds = match rt.creds().await {
        Some(creds) => creds,
        None => return error_response(StatusCode::UNAUTHORIZED, "请先绑定本机节点账号"),
    };
    let token = match creds
        .user_token
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        Some(token) => token.to_string(),
        None => {
            return error_response(
                StatusCode::UNAUTHORIZED,
                "本机节点缺少云端登录 token，请重新绑定",
            )
        }
    };
    let auth_path = match source_auth_json_path() {
        Ok(path) => path,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, error.to_string()),
    };
    if path_in_managed_vault(&auth_path) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "当前使用的是保险箱临时凭据，不能反向覆盖云端备份。",
        );
    }
    let auth_json = match read_auth_json_value(&auth_path) {
        Ok(value) => value,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, error.to_string()),
    };
    if let Err(error) = validate_chatgpt_auth_cache(&auth_json) {
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
        Ok(value) => (
            StatusCode::OK,
            Json(json!({
                "ok": true,
                "message": "已把本机 Codex Pro 凭据加密备份到云端保险箱。",
                "cloud": value,
                "local": local_status(),
            })),
        ),
        Err(error) => error_response(StatusCode::BAD_GATEWAY, error.to_string()),
    }
}

async fn restore_handler(
    State(rt): State<Arc<crate::NodeRuntime>>,
    Json(req): Json<RestoreRequest>,
) -> impl IntoResponse {
    match restore_from_cloud(
        &rt,
        req.purpose
            .unwrap_or_else(|| "temporary_local_codex_cli".to_string()),
        None,
        None,
    )
    .await
    {
        Ok(lease) => (
            StatusCode::OK,
            Json(json!({
                "ok": true,
                "message": lease.message.unwrap_or_else(|| "已恢复为本机临时 Codex Pro 会话。".to_string()),
                "lease_id": lease.lease_id,
                "slot_id": lease.slot_id,
                "account_hint_hash": lease.account_hint_hash,
                "local": local_status(),
            })),
        ),
        Err(error) => error_response(StatusCode::BAD_GATEWAY, error.to_string()),
    }
}

pub(crate) async fn try_auto_switch_after_codex_failure(
    rt: &Arc<crate::NodeRuntime>,
    stdout_text: &str,
    stderr_text: &str,
) -> Result<Option<String>> {
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
    if lease.account_hint_hash.as_deref() == Some(previous_hint) {
        return Ok(None);
    }
    Ok(Some(format!(
        "Codex 当前账号额度或认证不可用，已自动切换到保险箱备用账号{}，正在重试本轮任务。",
        lease
            .account_hint_hash
            .as_deref()
            .map(|hint| format!(" ({hint})"))
            .unwrap_or_default()
    )))
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
        Err(_) => bail!("云端返回的 auth_json 不是有效 JSON"),
    };
    if let Err(error) = validate_chatgpt_auth_cache(&auth_value) {
        bail!("云端保险箱凭据校验失败: {error}");
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
            lease_id: lease.lease_id.clone(),
            lease_expires_at: None,
        },
    )?;
    std::env::set_var("CODEX_HOME", &home);
    rt.refresh_cli_probe_now().await;
    Ok(lease)
}

async fn clear_handler(State(rt): State<Arc<crate::NodeRuntime>>) -> impl IntoResponse {
    let active = std::env::var("CODEX_HOME").ok().map(PathBuf::from);
    let active_meta = active
        .as_ref()
        .filter(|path| path_in_managed_vault(path))
        .and_then(|path| read_slot_meta(path).ok().flatten());
    let cloud_clear =
        crate::node_agent_codex_vault_emergency::clear_cloud_emergency_lease(&rt, active_meta.as_ref())
            .await;
    if let Err(error) = safe_remove_all_managed_homes() {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
    }
    if active
        .as_ref()
        .is_some_and(|path| path_in_managed_vault(path))
    {
        std::env::remove_var("CODEX_HOME");
    }
    rt.refresh_cli_probe_now().await;
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

async fn delete_cloud_handler(State(rt): State<Arc<crate::NodeRuntime>>) -> impl IntoResponse {
    let creds = match rt.creds().await {
        Some(creds) => creds,
        None => return error_response(StatusCode::UNAUTHORIZED, "请先绑定本机节点账号"),
    };
    let token = match creds
        .user_token
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        Some(token) => token.to_string(),
        None => {
            return error_response(
                StatusCode::UNAUTHORIZED,
                "本机节点缺少云端登录 token，请重新绑定",
            )
        }
    };
    let url = format!(
        "{}/api/me/codex-vault/auth-cache",
        rt.cloud_http_url().trim_end_matches('/')
    );
    match cloud_delete(&url, &token).await {
        Ok(value) => (
            StatusCode::OK,
            Json(json!({
                "ok": true,
                "message": "已删除云端 Codex Pro 保险箱备份。",
                "cloud": value,
                "local": local_status(),
            })),
        ),
        Err(error) => error_response(StatusCode::BAD_GATEWAY, error.to_string()),
    }
}

fn local_status() -> CodexVaultLocalStatus {
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

fn source_auth_json_path() -> Result<PathBuf> {
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
            anyhow!(
                "没有找到可备份的 Codex auth.json；当前只支持备份 Codex ChatGPT / Pro auth.json。"
            )
        })
}

fn inspect_auth_home(home: &Path) -> AuthCacheInspection {
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

fn read_auth_json_value(path: &Path) -> Result<Value> {
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
        bail!("只支持 ChatGPT / Pro 登录态，不支持 API key 凭据");
    }
    if refresh_token(value).is_none() {
        bail!("缺少 refresh_token，不能作为 Codex Pro 凭据使用");
    }
    Ok(())
}

fn refresh_token(value: &Value) -> Option<&str> {
    value
        .pointer("/tokens/refresh_token")
        .or_else(|| value.get("refresh_token"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|token| token.len() >= 20)
}

fn account_hint_hash(value: &Value) -> Option<String> {
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

fn read_slot_meta(home: &Path) -> Result<Option<ManagedSlotMeta>> {
    let path = home.join("elon-codex-vault-slot.json");
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("无法读取保险箱槽位元数据 {}", path.display()))?;
    Ok(Some(serde_json::from_str(&text)?))
}

fn active_managed_slot_meta() -> Option<ManagedSlotMeta> {
    let active = std::env::var("CODEX_HOME").ok().map(PathBuf::from)?;
    if !path_in_managed_vault(&active) {
        return None;
    }
    read_slot_meta(&active).ok().flatten()
}

fn inspect_managed_slots(active_meta: Option<&ManagedSlotMeta>) -> Vec<ManagedAuthSlotInspection> {
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

fn safe_remove_managed_home(home: &Path) -> Result<()> {
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

fn safe_remove_all_managed_homes() -> Result<()> {
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

fn tighten_permissions(home: &Path, auth_path: &Path) {
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

fn managed_codex_home() -> PathBuf {
    data_base_dir()
        .join("Elon")
        .join("codex-vault")
        .join("codex-home")
}

fn managed_slots_root() -> PathBuf {
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

fn safe_slot_id(slot_id: &str) -> String {
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

fn default_codex_home() -> Option<PathBuf> {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .map(|home| home.join(".codex"))
}

fn data_base_dir() -> PathBuf {
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

fn path_in_managed_vault(path: &Path) -> bool {
    let full = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let root = data_base_dir().join("Elon").join("codex-vault");
    let root = std::fs::canonicalize(&root).unwrap_or(root);
    full.starts_with(root)
}

async fn cloud_status(rt: &Arc<crate::NodeRuntime>) -> Result<Value> {
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

async fn cloud_get(url: &str, token: &str) -> Result<Value> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_default();
    let resp = client.get(url).bearer_auth(token).send().await?;
    decode_cloud_response(resp).await
}

async fn cloud_post(url: &str, token: &str, body: &Value) -> Result<Value> {
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

async fn cloud_delete(url: &str, token: &str) -> Result<Value> {
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

async fn decode_cloud_response(resp: reqwest::Response) -> Result<Value> {
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
