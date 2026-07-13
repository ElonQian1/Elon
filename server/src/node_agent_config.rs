//! 节点配置与凭证持久化，以及启动阶段的数据根 fail-closed 校验。

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
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

pub(super) fn load_persisted() -> Result<PersistedState> {
    load_persisted_from(&state_path())
}

fn load_persisted_from(path: &Path) -> Result<PersistedState> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(PersistedState::default()),
        Err(error) => {
            return Err(error).with_context(|| format!("无法读取节点状态文件 {}", path.display()));
        }
    };
    serde_json::from_str(&contents)
        .with_context(|| format!("节点状态文件损坏，拒绝用默认值覆盖: {}", path.display()))
}

pub(super) fn save_persisted(state: &PersistedState) -> Result<()> {
    save_persisted_to(&state_path(), state)
}

fn save_persisted_to(path: &Path, state: &PersistedState) -> Result<()> {
    let json = serde_json::to_vec_pretty(state).context("无法序列化节点状态")?;
    crate::node_agent_atomic_file::write(path, &json)
        .with_context(|| format!("无法持久化节点状态 {}", path.display()))
}

#[cfg(test)]
mod persistence_tests {
    use super::*;

    #[test]
    fn missing_state_file_is_the_only_default_case() {
        let root =
            std::env::temp_dir().join(format!("elon-node-state-missing-{}", uuid::Uuid::new_v4()));
        let state = load_persisted_from(&root.join("node.json")).expect("missing means default");
        assert!(state.install_id.is_none());
    }

    #[test]
    fn corrupted_state_file_fails_closed() {
        let root =
            std::env::temp_dir().join(format!("elon-node-state-corrupt-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp state root");
        let path = root.join("node.json");
        std::fs::write(&path, b"{not-json").expect("write corrupt state");

        let error = match load_persisted_from(&path) {
            Ok(_) => panic!("corrupt state must fail"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("状态文件损坏"));
        assert_eq!(
            std::fs::read(&path).expect("state remains untouched"),
            b"{not-json"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn non_not_found_read_error_is_not_treated_as_default() {
        let root = std::env::temp_dir().join(format!(
            "elon-node-state-read-error-{}",
            uuid::Uuid::new_v4()
        ));
        let path = root.join("node.json");
        std::fs::create_dir_all(&path).expect("make state path a directory");

        let result = load_persisted_from(&path);

        assert!(result.is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn state_save_replaces_file_atomically() {
        let root =
            std::env::temp_dir().join(format!("elon-node-state-save-{}", uuid::Uuid::new_v4()));
        let path = root.join("node.json");
        let mut state = PersistedState::default();
        state.install_id = Some("ins_test".to_string());

        save_persisted_to(&path, &state).expect("save state");
        let loaded = load_persisted_from(&path).expect("reload state");

        assert_eq!(loaded.install_id.as_deref(), Some("ins_test"));
        assert_eq!(
            std::fs::read_dir(&root)
                .expect("read state root")
                .filter_map(|entry| entry.ok())
                .count(),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn valid_unified_root_owns_storage_path() {
        let root =
            std::env::temp_dir().join(format!("elon-node-storage-root-{}", uuid::Uuid::new_v4()));
        let data_root = super::super::node_agent_data_root::NodeDataRootState::from_prepared_paths(
            elon_pc_dev_runtime::NodeDataPaths::new(&root),
            super::super::node_agent_data_root::NodeDataRootSource::Persisted,
            None,
            Some(PathBuf::from(r"C:\legacy-storage")),
        );
        let persisted = PersistedState {
            storage_enabled: Some(true),
            storage_root: Some(r"C:\legacy-storage".to_string()),
            ..PersistedState::default()
        };

        let settings = initial_storage_settings(&persisted, &data_root);
        let expected = root.join("storage").to_string_lossy().to_string();

        assert_eq!(settings.root_path.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn invalid_explicit_data_root_blocks_legacy_storage_fallback() {
        let root =
            std::env::temp_dir().join(format!("elon-node-invalid-root-{}", uuid::Uuid::new_v4()));
        let data_root = super::super::node_agent_data_root::NodeDataRootState::from_prepared_paths(
            elon_pc_dev_runtime::NodeDataPaths::new(root),
            super::super::node_agent_data_root::NodeDataRootSource::Persisted,
            None,
            Some(PathBuf::from(r"C:\legacy-storage")),
        )
        .block_invalid_root("marker mismatch");
        let persisted = PersistedState {
            storage_enabled: Some(true),
            storage_root: Some(r"C:\legacy-storage".to_string()),
            ..PersistedState::default()
        };

        let settings = initial_storage_settings(&persisted, &data_root);

        assert!(!settings.enabled);
        assert!(settings.root_path.is_none());
    }

    #[test]
    fn invalid_explicit_root_survives_unrelated_state_persistence() {
        let root = std::env::temp_dir().join(format!(
            "elon-node-invalid-persisted-root-{}",
            uuid::Uuid::new_v4()
        ));
        let expected = root.to_string_lossy().to_string();
        let mut persisted = PersistedState {
            node_data_root: Some(expected.clone()),
            storage_enabled: Some(true),
            storage_root: Some(r"C:\legacy-storage".to_string()),
            ..PersistedState::default()
        };

        persisted.set_install_id("ins_test");
        persisted.set_credentials(None);

        assert_eq!(persisted.node_data_root.as_deref(), Some(expected.as_str()));
        assert_eq!(persisted.storage_enabled, Some(true));
        assert_eq!(
            persisted.storage_root.as_deref(),
            Some(r"C:\legacy-storage")
        );
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
    pub(super) fn set_install_id(&mut self, install_id: &str) {
        self.install_id = Some(install_id.to_string());
    }

    pub(super) fn set_credentials(&mut self, credentials: Option<&Credentials>) {
        self.agent_id = credentials.map(|credentials| credentials.agent_id.clone());
        self.agent_secret = credentials.map(|credentials| credentials.agent_secret.clone());
        self.owner_user_id = credentials.map(|credentials| credentials.owner_user_id.clone());
        self.user_token = credentials.and_then(|credentials| credentials.user_token.clone());
    }

    pub(super) fn set_storage_settings(
        &mut self,
        storage: &super::pc_storage_repo::StorageSettings,
    ) {
        self.storage_enabled = Some(storage.enabled);
        self.storage_root = storage.root_path.clone();
        self.storage_git_base_url = storage.git_base_url.clone();
    }

    /// Persist a data root only after marker/path validation has succeeded.
    /// Returning false lets startup keep an invalid environment bootstrap out
    /// of node.json so a corrected environment can recover on the next start.
    pub(super) fn set_validated_node_data_root(
        &mut self,
        node_data_root: &super::node_agent_data_root::NodeDataRootState,
    ) -> bool {
        let Some(paths) = node_data_root.paths.as_ref() else {
            return false;
        };
        self.node_data_root = Some(paths.root().to_string_lossy().to_string());
        self.node_data_legacy_workspace_root = node_data_root
            .legacy_workspace_root
            .as_ref()
            .map(|path| path.to_string_lossy().to_string());
        self.node_data_legacy_storage_root = node_data_root
            .legacy_storage_root
            .as_ref()
            .map(|path| path.to_string_lossy().to_string());
        true
    }
}

pub(super) fn initial_node_data_root(
    persisted: &PersistedState,
    install_id: &str,
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
    let mut state = super::node_agent_data_root::resolve(
        persisted.node_data_root.as_deref(),
        persisted
            .node_data_legacy_workspace_root
            .as_deref()
            .map(PathBuf::from),
        legacy_storage_root,
    );
    let Some(configured) = state.paths.as_ref().map(|paths| paths.root().to_path_buf()) else {
        return state;
    };
    match super::node_agent_data_root::validate_and_prepare(
        configured.to_string_lossy().as_ref(),
        install_id,
    ) {
        Ok(paths) => {
            state.paths = Some(paths);
            state
        }
        Err(error) => state.block_invalid_root(error),
    }
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
    data_root: &super::node_agent_data_root::NodeDataRootState,
) -> super::pc_storage_repo::StorageSettings {
    let env_nonempty = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
    let explicit_root =
        env_nonempty("NODE_STORAGE_ROOT").or_else(|| env_nonempty("ELON_STORAGE_ROOT"));
    let git_base_url = env_nonempty("NODE_STORAGE_GIT_BASE_URL")
        .or_else(|| env_nonempty("ELON_STORAGE_GIT_BASE_URL"))
        .or_else(|| persisted.storage_git_base_url.clone());
    let requested_enabled = super::node_agent_env::env_flag("NODE_STORAGE_ENABLED")
        .or_else(|| super::node_agent_env::env_flag("ELON_STORAGE_ENABLED"))
        .or(persisted.storage_enabled)
        .unwrap_or(false);
    let root_is_configured =
        data_root.source != super::node_agent_data_root::NodeDataRootSource::Unconfigured;
    let (enabled, root_path) = if let Some(paths) = data_root.paths.as_ref() {
        // A valid unified root owns storage as well. Legacy overrides are
        // migration inputs only and must not keep writing new data to C:.
        (
            requested_enabled,
            Some(paths.storage().to_string_lossy().to_string()),
        )
    } else if root_is_configured {
        // Explicit root configured but failed validation: block storage rather
        // than silently falling back to the legacy user-profile directory.
        (false, None)
    } else {
        (
            // A legacy storage path may be retained for migration metadata,
            // but it is never effective until a unified data root validates.
            false,
            explicit_root.or_else(|| persisted.storage_root.clone()),
        )
    };
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
