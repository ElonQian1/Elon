//! 节点配置与凭证持久化。
//! 从 node_agent_main.rs 拆分，保持行为不变。

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

// ── 配置结构 ──────────────────────────────────────────────────────────────────

/// 静态运行配置（云端地址、本地模型地址、价格），均有合理默认值，普通用户无需配置。
#[derive(Clone)]
pub struct NodeConfig {
    pub cloud_url: String,
    /// 云端 HTTP/HTTPS 地址（用于 REST API 调用，如登录、注册节点、注册外部项目）。
    /// 默认从 cloud_url 派生：ws://X → http://X，wss://X → https://X。
    pub cloud_http_url: String,
    /// 本地 Ollama 地址
    pub ollama_url: String,
    /// 可选：LM Studio 地址
    pub lm_studio_url: Option<String>,
    /// 用户自定义 OpenAI-compatible 地址
    pub custom_url: Option<String>,
    /// 每 1k tokens 收取的平台积分（默认 0.1）
    pub price_per_1k: f64,
}

/// 节点凭证：由「一次登录」自动换取并持久化，普通用户永远不用手动填。
#[derive(Clone)]
pub struct Credentials {
    pub agent_id: String,
    pub agent_secret: String,
    pub owner_user_id: String,
    /// 用户的 elon 登录 token（用于代理调用云端 API，例如注册外部项目）
    pub user_token: Option<String>,
}

/// 持久化到磁盘的状态（`%APPDATA%\elon-node-agent\node.json` / `~/.config/elon-node-agent/node.json`）。
#[derive(Default, Serialize, Deserialize)]
pub(super) struct PersistedState {
    pub(super) install_id: Option<String>,
    pub(super) agent_id: Option<String>,
    pub(super) agent_secret: Option<String>,
    pub(super) owner_user_id: Option<String>,
    pub(super) user_token: Option<String>,
    /// Canonical root for large, reproducible node data. Credentials remain in
    /// this small APPDATA state file and are not moved with the data root.
    pub(super) node_data_root: Option<String>,
    pub(super) node_data_legacy_workspace_root: Option<String>,
    pub(super) node_data_legacy_storage_root: Option<String>,
    pub(super) storage_enabled: Option<bool>,
    pub(super) storage_root: Option<String>,
    pub(super) storage_git_base_url: Option<String>,
}

fn derive_http_url(ws_url: &str) -> String {
    if let Some(rest) = ws_url.strip_prefix("wss://") {
        format!("https://{}", rest.split('/').next().unwrap_or(rest))
    } else if let Some(rest) = ws_url.strip_prefix("ws://") {
        format!("http://{}", rest.split('/').next().unwrap_or(rest))
    } else {
        ws_url.to_string()
    }
}

/// 本机名，作为节点 label / 登录设备名。
pub fn machine_label() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "pc".into())
}

/// 凭证持久化文件路径。
pub fn state_path() -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    } else {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".config"))
            })
    };
    base.unwrap_or_else(|| PathBuf::from("."))
        .join("elon-node-agent")
        .join("node.json")
}

pub(super) fn load_persisted() -> PersistedState {
    std::fs::read_to_string(state_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub(super) fn save_persisted(s: &PersistedState) {
    let p = state_path();
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(s) {
        let _ = std::fs::write(&p, json);
    }
}

pub(super) fn ensure_install_id(persisted: &mut PersistedState) -> String {
    if let Some(existing) = persisted
        .install_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return existing.to_string();
    }
    let install_id = format!("ins_{}", uuid::Uuid::new_v4().simple());
    persisted.install_id = Some(install_id.clone());
    install_id
}

impl PersistedState {
    pub(super) fn from_parts(
        install_id: &str,
        c: Option<&Credentials>,
        storage: &super::pc_storage_repo::StorageSettings,
        node_data_root: &super::node_agent_data_root::NodeDataRootState,
    ) -> Self {
        Self {
            install_id: Some(install_id.to_string()),
            agent_id: c.map(|c| c.agent_id.clone()),
            agent_secret: c.map(|c| c.agent_secret.clone()),
            owner_user_id: c.map(|c| c.owner_user_id.clone()),
            user_token: c.and_then(|c| c.user_token.clone()),
            node_data_root: node_data_root
                .configured_root()
                .map(|path| path.to_string_lossy().to_string()),
            node_data_legacy_workspace_root: node_data_root
                .legacy_workspace_root
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            node_data_legacy_storage_root: node_data_root
                .legacy_storage_root
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            storage_enabled: Some(storage.enabled),
            storage_root: storage.root_path.clone(),
            storage_git_base_url: storage.git_base_url.clone(),
        }
    }
}

pub(super) fn initial_node_data_root(
    persisted: &PersistedState,
) -> super::node_agent_data_root::NodeDataRootState {
    let legacy_storage_root = ["NODE_STORAGE_ROOT", "ELON_STORAGE_ROOT"]
        .into_iter()
        .find_map(|key| {
            std::env::var(key)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .or_else(|| persisted.node_data_legacy_storage_root.clone())
        .or_else(|| persisted.storage_root.clone())
        .map(PathBuf::from)
        .or_else(|| {
            let default = super::pc_storage_repo::legacy_default_storage_root();
            default.exists().then_some(default)
        });
    super::node_agent_data_root::resolve(
        persisted.node_data_root.as_deref(),
        persisted
            .node_data_legacy_workspace_root
            .as_deref()
            .map(PathBuf::from),
        legacy_storage_root,
    )
}

/// 从环境变量 / 持久化文件解析已有凭证；都没有时返回 None（需登录）。
/// 环境变量优先（供高级用户/服务器覆盖），否则用上次持久化的结果。
pub(super) fn initial_credentials(persisted: &PersistedState) -> Option<Credentials> {
    let env_nonempty = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    let agent_id = env_nonempty("NODE_AGENT_ID").or_else(|| persisted.agent_id.clone())?;
    let agent_secret =
        env_nonempty("NODE_AGENT_SECRET").or_else(|| persisted.agent_secret.clone())?;
    let owner_user_id = env_nonempty("NODE_OWNER_USER_ID")
        .or_else(|| persisted.owner_user_id.clone())
        .unwrap_or_default();
    let user_token = env_nonempty("NODE_USER_TOKEN").or_else(|| persisted.user_token.clone());
    Some(Credentials {
        agent_id,
        agent_secret,
        owner_user_id,
        user_token,
    })
}

pub(super) fn initial_storage_settings(
    persisted: &PersistedState,
) -> super::pc_storage_repo::StorageSettings {
    let env_nonempty = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
    let explicit_root =
        env_nonempty("NODE_STORAGE_ROOT").or_else(|| env_nonempty("ELON_STORAGE_ROOT"));
    let data_root = initial_node_data_root(persisted);
    let root_path = explicit_root
        .or_else(|| {
            data_root
                .paths
                .as_ref()
                .map(|paths| paths.storage().to_string_lossy().to_string())
        })
        .or_else(|| persisted.storage_root.clone());
    let git_base_url = env_nonempty("NODE_STORAGE_GIT_BASE_URL")
        .or_else(|| env_nonempty("ELON_STORAGE_GIT_BASE_URL"))
        .or_else(|| persisted.storage_git_base_url.clone());
    let enabled = super::node_agent_env::env_flag("NODE_STORAGE_ENABLED")
        .or_else(|| super::node_agent_env::env_flag("ELON_STORAGE_ENABLED"))
        .or(persisted.storage_enabled)
        .unwrap_or(false);
    super::pc_storage_repo::StorageSettings {
        enabled,
        root_path: root_path.or_else(|| {
            enabled.then(|| {
                super::pc_storage_repo::default_storage_root()
                    .to_string_lossy()
                    .to_string()
            })
        }),
        git_base_url,
    }
}

/// 账号 + 密码登录云端，换取 token。
pub(super) async fn cloud_login(cfg: &NodeConfig, account: &str, password: &str) -> Result<String> {
    let url = format!(
        "{}/api/auth/login",
        cfg.cloud_http_url.trim_end_matches('/')
    );
    let client =
        super::node_agent_cloud_net::direct_cloud_client_or_default(Duration::from_secs(15));
    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "account": account,
            "password": password,
            "device_name": machine_label(),
        }))
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("登录失败 {}: {}", status, body));
    }
    let j: serde_json::Value = resp.json().await?;
    j.get("token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("登录响应缺少 token"))
}

impl NodeConfig {
    pub(super) fn from_env() -> Result<Self> {
        let cloud_url = std::env::var("NODE_CLOUD_URL")
            .unwrap_or_else(|_| "ws://43.139.149.158:8080/agent/ws".into());
        let cloud_http_url =
            std::env::var("NODE_CLOUD_HTTP_URL").unwrap_or_else(|_| derive_http_url(&cloud_url));
        let ollama_url =
            std::env::var("NODE_OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());
        let lm_studio_url = std::env::var("NODE_LM_STUDIO_URL")
            .ok()
            .filter(|v| !v.is_empty());
        let custom_url = std::env::var("NODE_CUSTOM_LLM_URL")
            .ok()
            .filter(|v| !v.is_empty());
        let price_per_1k = std::env::var("NODE_PRICE_PER_1K")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.1f64);

        Ok(Self {
            cloud_url,
            cloud_http_url,
            ollama_url,
            lm_studio_url,
            custom_url,
            price_per_1k,
        })
    }
}
