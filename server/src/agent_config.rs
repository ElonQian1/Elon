//! AI 代理与模型配置类型。
//!
//! 包含 API 代理配置（[`AgentConfig`]）、图像生成模型配置（[`ImageModelConfig`]）、
//! AI 后端选择（[`AiBackend`]）以及用户自定义配置（[`UserAgentConfig`]）。
//!
//! 所有类型通过 `crate::types::*` 重新导出，调用方无需更改引用路径。

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── AI 代理配置 ───────────────────────────────────────────────────────────────

/// 单个 AI 代理的配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub api_base: String,
    pub api_key: String,
    pub model: String,
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
        })
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
    pub api_key: Option<String>,
    /// 自定义模型名（None = 使用所选全局代理的模型）
    pub model: Option<String>,
    /// 用户昵称（可选，仅用于管理后台展示）
    pub nickname: Option<String>,
    /// 最后更新时间
    pub updated_at: Option<String>,
}

impl UserAgentConfig {
    /// 从用户工作区加载配置
    pub fn load(workspace: &std::path::Path) -> Option<Self> {
        let content = std::fs::read_to_string(workspace.join("agent_config.json")).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// 持久化到用户工作区
    pub fn save(&self, workspace: &std::path::Path) -> Result<()> {
        std::fs::create_dir_all(workspace)?;
        std::fs::write(
            workspace.join("agent_config.json"),
            serde_json::to_string_pretty(self)?,
        )?;
        Ok(())
    }

    /// 用户是否设置了任何自定义配置
    pub fn has_config(&self) -> bool {
        self.use_agent.is_some()
            || self.api_base.is_some()
            || self.api_key.is_some()
            || self.model.is_some()
    }

    /// 解析为实际可使用的 AgentConfig（以全局代理为基础，用自定义值覆盖）
    pub fn resolve(&self, global: &AgentsConfig) -> Option<AgentConfig> {
        let base = global.get_agent(self.use_agent.as_deref())?.clone();
        Some(AgentConfig {
            name: format!("{}(用户自定义)", base.name),
            api_base: self.api_base.clone().unwrap_or(base.api_base),
            api_key: self.api_key.clone().unwrap_or(base.api_key),
            model: self.model.clone().unwrap_or(base.model),
        })
    }
}
