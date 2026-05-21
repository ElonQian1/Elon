use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};
use tokio::sync::RwLock;

/// 单个 AI 代理的配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub api_base: String,
    pub api_key: String,
    pub model: String,
}

impl AgentConfig {
    fn from_env(prefix: &str, default_base: &str, default_model: &str) -> Option<Self> {
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
    pub fn get_agent(&self, name: Option<&str>) -> &AgentConfig {
        let key = name.unwrap_or(&self.default_agent);
        self.agents
            .get(key)
            .or_else(|| self.agents.get(&self.default_agent))
            .or_else(|| self.agents.values().next())
            .expect("至少有一个 AI 代理")
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

/// 全局状态，在各路由间共享
pub struct AppState {
    /// AI 代理配置（RwLock 支持运行时通过管理后台修改，无需重启）
    pub agents_config: RwLock<AgentsConfig>,
    /// 用户工作区根目录（每个用户在此独立目录开发自己的项目）
    pub project_root: std::path::PathBuf,
    /// 用户工作区根目录字符串（冗余保存，方便直接传给工具层）
    pub workspace_root: String,
    /// 服务器对外公开的 URL（用于生成 APK 下载链接）
    pub public_url: String,
    /// HTTP 客户端（复用连接）
    pub http_client: reqwest::Client,
    /// 管理后台访问令牌（对应 .env 中的 ADMIN_TOKEN）
    pub admin_token: String,
    /// agents.json 持久化路径
    pub config_path: std::path::PathBuf,
}

impl AppState {
    pub fn new() -> Result<Self> {
        let mut agents: HashMap<String, AgentConfig> = HashMap::new();

        let providers = [
            ("OPENAI",   "https://api.openai.com/v1",                    "gpt-4o"),
            ("DEEPSEEK", "https://api.deepseek.com/v1",                  "deepseek-chat"),
            ("CLAUDE",   "https://api.anthropic.com/v1",                 "claude-3-5-sonnet-20241022"),
            ("HUNYUAN",  "https://api.hunyuan.cloud.tencent.com/v1",     "hunyuan-turbo"),
            ("CUSTOM",   "",                                              ""),
        ];

        for (prefix, default_base, default_model) in providers {
            if let Some(cfg) = AgentConfig::from_env(prefix, default_base, default_model) {
                tracing::info!("已加载 AI 代理: {} -> {}", cfg.name, cfg.api_base);
                agents.insert(cfg.name.clone(), cfg);
            }
        }

        let config_path = std::path::PathBuf::from(
            std::env::var("CONFIG_PATH").unwrap_or_else(|_| "./agents.json".into()),
        );

        // 优先从 agents.json 加载（管理后台保存的配置），否则用 .env
        let agents_config = if let Some(mut saved) = AgentsConfig::load_from_file(&config_path) {
            tracing::info!("从 {} 加载代理配置", config_path.display());
            // 将 .env 中有但 agents.json 没有的代理补充进来
            for (k, v) in &agents {
                saved.agents.entry(k.clone()).or_insert_with(|| v.clone());
            }
            saved
        } else {
            if agents.is_empty() {
                anyhow::bail!(
                    "至少需要配置一个 AI 代理，请设置 AGENT_OPENAI_KEY / AGENT_DEEPSEEK_KEY / AGENT_HUNYUAN_KEY 等"
                );
            }
            let default_agent = std::env::var("DEFAULT_AGENT")
                .unwrap_or_else(|_| agents.keys().next().unwrap().clone());
            AgentsConfig { agents, default_agent }
        };

        let project_root_str = std::env::var("WORKSPACE_ROOT")
            .unwrap_or_else(|_| "/home/ubuntu/workspaces".into());

        let public_url = std::env::var("PUBLIC_URL")
            .unwrap_or_else(|_| "http://182.254.168.75:8080".into());

        let admin_token = std::env::var("ADMIN_TOKEN").unwrap_or_else(|_| {
            tracing::warn!("未设置 ADMIN_TOKEN，使用默认值 'elon-admin'，生产环境请修改！");
            "elon-admin".into()
        });

        tracing::info!("默认 AI 代理: {}", agents_config.default_agent);
        tracing::info!("用户工作区根目录: {}", project_root_str);
        tracing::info!("公开 URL: {}", public_url);

        let http_client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(120))
            .build()?;

        Ok(Self {
            agents_config: RwLock::new(agents_config),
            project_root: std::path::PathBuf::from(&project_root_str),
            workspace_root: project_root_str,
            public_url,
            http_client,
            admin_token,
            config_path,
        })
    }

    /// 获取某个用户的工作区目录（路径: {workspace_root}/{user_id}/）
    pub fn get_user_workspace(&self, user_id: &str) -> std::path::PathBuf {
        get_user_workspace(&self.workspace_root, user_id)
    }
}

/// 独立辅助函数：根据 workspace_root 和 user_id 计算用户工作区路径
pub fn get_user_workspace(workspace_root: &str, user_id: &str) -> std::path::PathBuf {
    let safe_id: String = user_id
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .take(32)
        .collect();
    std::path::PathBuf::from(workspace_root)
        .join(if safe_id.is_empty() { "default".into() } else { safe_id })
}


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
    pub fn resolve(&self, global: &AgentsConfig) -> AgentConfig {
        let base = global.get_agent(self.use_agent.as_deref()).clone();
        AgentConfig {
            name: format!("{}(用户自定义)", base.name),
            api_base: self.api_base.clone().unwrap_or(base.api_base),
            api_key: self.api_key.clone().unwrap_or(base.api_key),
            model: self.model.clone().unwrap_or(base.model),
        }
    }
}

/// WebSocket 消息格式（发给 APK）
#[derive(serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// AI 思考/操作进度
    Progress { message: String },
    /// AI 正在执行的工具
    ToolCall { tool: String, args: serde_json::Value },
    /// 工具执行结果
    ToolResult { tool: String, result: String },
    /// 最终回复
    Done { message: String, apk_url: Option<String> },
    /// 发生错误
    Error { message: String },
}

impl WsMessage {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| r#"{"type":"error","message":"序列化失败"}"#.into())
    }
}
