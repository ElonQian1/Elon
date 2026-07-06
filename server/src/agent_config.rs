//! AI 代理与模型配置类型。
//!
//! 包含 API 代理配置（[`AgentConfig`]）、图像生成模型配置（[`ImageModelConfig`]）、
//! AI 后端选择（[`AiBackend`]）以及用户自定义配置（[`UserAgentConfig`]）。
//!
//! 所有类型通过 `crate::types::*` 重新导出，调用方无需更改引用路径。

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::user_agent_secrets::{decrypt_api_key, encrypt_api_key};

// ── AI 代理配置 ───────────────────────────────────────────────────────────────

/// 单个 AI 代理的配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub api_base: String,
    pub api_key: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_mode: Option<String>,
}

impl AgentConfig {
    pub(crate) fn from_env(prefix: &str, default_base: &str, default_model: &str) -> Option<Self> {
        let key = std::env::var(format!("AGENT_{}_KEY", prefix)).ok()?;
        Some(Self {
            name: prefix.to_lowercase(),
            api_base: std::env::var(format!("AGENT_{}_BASE", prefix))
                .unwrap_or_else(|_| default_base.into()),
            api_key: key,
            model: std::env::var(format!("AGENT_{}_MODEL", prefix))
                .unwrap_or_else(|_| default_model.into()),
            embedding_model: std::env::var(format!("AGENT_{}_EMBEDDING_MODEL", prefix))
                .ok()
                .and_then(clean_optional),
            usage_mode: None,
        })
    }

    pub fn usage_mode(&self) -> &str {
        self.usage_mode.as_deref().unwrap_or("server_api_key")
    }
}

/// 可动态修改的 AI 代理配置集合（被 RwLock 包裹，支持运行时热更新）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsConfig {
    pub agents: HashMap<String, AgentConfig>,
    pub default_agent: String,
}

impl AgentsConfig {
    /// 获取指定名称的代理，不存在则返回默认代理
    pub fn get_agent(&self, name: Option<&str>) -> Option<&AgentConfig> {
        let key = name.unwrap_or(&self.default_agent);
        self.agents
            .get(key)
            .or_else(|| self.agents.get(&self.default_agent))
            .or_else(|| self.agents.values().next())
    }

    /// 从 JSON 文件加载配置
    pub fn load_from_file(path: &std::path::Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// 持久化配置到 JSON 文件
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

// ── 图像生成模型配置 ──────────────────────────────────────────────────────────

/// 文生图模型配置（TokenHub / 混元生图 3.0）
#[derive(Debug, Clone)]
pub struct ImageModelConfig {
    pub api_base: String,
    pub api_key: String,
    pub model: String,
    pub poll_interval_secs: u64,
    pub max_attempts: usize,
}

impl ImageModelConfig {
    pub(crate) fn from_env() -> Option<Self> {
        let api_key = std::env::var("IMAGE_API_KEY")
            .or_else(|_| std::env::var("TOKENHUB_IMAGE_KEY"))
            .ok()?;

        Some(Self {
            api_base: std::env::var("IMAGE_API_BASE")
                .unwrap_or_else(|_| "https://tokenhub.tencentmaas.com/v1/api/image".into()),
            api_key,
            model: std::env::var("IMAGE_MODEL").unwrap_or_else(|_| "hy-image-v3.0".into()),
            poll_interval_secs: std::env::var("IMAGE_POLL_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            max_attempts: std::env::var("IMAGE_MAX_ATTEMPTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(24),
        })
    }
}

// ── AI 后端选择 ───────────────────────────────────────────────────────────────

/// 默认处理用户消息的后端：本地 AI CLI 或原 OpenAI 兼容 API。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiBackend {
    LocalCli,
    Api,
}

impl AiBackend {
    pub fn from_env_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "api" | "llm" | "openai" | "remote" => Self::Api,
            _ => Self::LocalCli,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalCli => "local_cli",
            Self::Api => "api",
        }
    }
}

// ── 用户自定义 AI 代理配置 ────────────────────────────────────────────────────

/// 用户自定义 AI 代理配置（存储在用户工作区 agent_config.json）
/// 各字段均可为 None，表示回退到全局配置对应的值
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserAgentConfig {
    /// 使用指定名称的全局代理（None = 使用服务器默认）
    pub use_agent: Option<String>,
    /// 自定义 API 地址（None = 使用所选全局代理的地址）
    pub api_base: Option<String>,
    /// 自定义 API 密钥（None = 使用所选全局代理的密钥）
    #[serde(default, skip_serializing)]
    pub api_key: Option<String>,
    /// 加密后的用户 API Key。明文只在运行时内存中使用，不再写入配置文件。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_encrypted: Option<String>,
    /// 自定义模型名（None = 使用所选全局代理的模型）
    pub model: Option<String>,
    /// 自定义 embedding 模型（None = 使用 RAG 默认 local-hash-v1）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    /// 用户昵称（可选，仅用于管理后台展示）
    pub nickname: Option<String>,
    /// 最近一次保存为开发代理时的能力探测结果。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_ok: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_warning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_checked_at: Option<String>,
    /// 最后更新时间
    pub updated_at: Option<String>,
}

impl UserAgentConfig {
    /// 从用户工作区加载配置
    pub fn load(workspace: &std::path::Path) -> Option<Self> {
        let content = std::fs::read_to_string(workspace.join("agent_config.json")).ok()?;
        let mut config: Self = serde_json::from_str(&content).ok()?;
        if config.api_key.is_none() {
            if let Some(encrypted) = config.api_key_encrypted.as_deref() {
                match decrypt_api_key(encrypted) {
                    Ok(key) => config.api_key = Some(key),
                    Err(error) => {
                        tracing::warn!("用户 API Key 解密失败，将保留密文引用: {}", error);
                    }
                }
            }
        }
        Some(config)
    }

    /// 持久化到用户工作区
    pub fn save(&self, workspace: &std::path::Path) -> Result<()> {
        std::fs::create_dir_all(workspace)?;
        let mut persisted = self.clone();
        if let Some(api_key) = self
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            persisted.api_key_encrypted = Some(encrypt_api_key(api_key)?);
        }
        persisted.api_key = None;
        std::fs::write(
            workspace.join("agent_config.json"),
            serde_json::to_string_pretty(&persisted)?,
        )?;
        Ok(())
    }

    /// 用户是否设置了任何自定义配置
    pub fn has_config(&self) -> bool {
        self.use_agent.is_some()
            || self.api_base.is_some()
            || self.has_api_key_reference()
            || self.model.is_some()
            || self.embedding_model.is_some()
    }

    pub fn has_api_key_reference(&self) -> bool {
        self.api_key
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
            || self
                .api_key_encrypted
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
    }

    pub fn has_direct_custom_api(&self) -> bool {
        self.use_agent.is_none()
            && self
                .api_base
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
            && self
                .model
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
            && self
                .api_key
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
    }

    pub fn remember_capability_probe(
        &mut self,
        result: &crate::user_agent_probe::UserAgentProbeResult,
        checked_at: String,
    ) {
        self.tool_call_ok = Some(result.tool_call_ok);
        self.tool_call_name = result.tool_call_name.clone();
        self.capability = Some(result.capability.clone());
        self.capability_warning = result.warning.clone();
        self.capability_checked_at = Some(checked_at);
    }

    pub fn clear_capability_probe(&mut self) {
        self.tool_call_ok = None;
        self.tool_call_name = None;
        self.capability = None;
        self.capability_warning = None;
        self.capability_checked_at = None;
    }

    /// 解析为实际可使用的 AgentConfig（以全局代理为基础，用自定义值覆盖）
    pub fn resolve(&self, global: &AgentsConfig) -> Option<AgentConfig> {
        if self.has_direct_custom_api() {
            return Some(AgentConfig {
                name: "user-custom-api".to_string(),
                api_base: self.api_base.clone()?,
                api_key: self.api_key.clone()?,
                model: self.model.clone()?,
                embedding_model: self.embedding_model.clone(),
                usage_mode: Some("user_api_key_proxy".to_string()),
            });
        }

        let base = global.get_agent(self.use_agent.as_deref())?.clone();
        let user_key_override = self.api_key.is_some();
        Some(AgentConfig {
            name: format!("{}(用户自定义)", base.name),
            api_base: self.api_base.clone().unwrap_or(base.api_base),
            api_key: self.api_key.clone().unwrap_or(base.api_key),
            model: self.model.clone().unwrap_or(base.model),
            embedding_model: self.embedding_model.clone().or(base.embedding_model),
            usage_mode: if user_key_override {
                Some("user_api_key_proxy".to_string())
            } else {
                base.usage_mode
            },
        })
    }
}

fn clean_optional(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}


#[cfg(test)]
#[path = "agent_config_tests.rs"]
mod tests;
