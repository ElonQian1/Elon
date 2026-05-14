use anyhow::Result;
use std::collections::HashMap;

/// 单个 AI 代理的配置
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub name: String,
    pub api_base: String,
    pub api_key: String,
    pub model: String,
}

impl AgentConfig {
    /// 从环境变量前缀加载，如 AGENT_OPENAI_KEY / AGENT_OPENAI_MODEL
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

/// 全局状态，在各路由间共享
pub struct AppState {
    /// 已注册的 AI 代理，key 为代理名称（openai / deepseek / claude）
    pub agents: HashMap<String, AgentConfig>,
    /// 默认使用的代理名称
    pub default_agent: String,
    /// 项目根目录（AI 代理操作文件的沙箱目录）
    pub project_root: std::path::PathBuf,
    /// HTTP 客户端（复用连接）
    pub http_client: reqwest::Client,
}

impl AppState {
    pub fn new() -> Result<Self> {
        let mut agents: HashMap<String, AgentConfig> = HashMap::new();

        // 按优先级依次尝试加载各 AI 代理配置
        let providers = [
            ("OPENAI",   "https://api.openai.com/v1",     "gpt-4o"),
            ("DEEPSEEK", "https://api.deepseek.com/v1",   "deepseek-chat"),
            ("CLAUDE",   "https://api.anthropic.com/v1",  "claude-3-5-sonnet-20241022"),
            ("CUSTOM",   "",                               ""),
        ];

        for (prefix, default_base, default_model) in providers {
            if let Some(cfg) = AgentConfig::from_env(prefix, default_base, default_model) {
                tracing::info!("已加载 AI 代理: {} -> {}", cfg.name, cfg.api_base);
                agents.insert(cfg.name.clone(), cfg);
            }
        }

        if agents.is_empty() {
            anyhow::bail!("至少需要配置一个 AI 代理，请设置 AGENT_OPENAI_KEY 或 AGENT_DEEPSEEK_KEY 等环境变量");
        }

        // 默认代理：优先用环境变量指定，否则取第一个
        let default_agent = std::env::var("DEFAULT_AGENT")
            .unwrap_or_else(|_| agents.keys().next().unwrap().clone());

        let project_root = std::env::var("WORKSPACE_ROOT")
            .unwrap_or_else(|_| "/home/ubuntu/workspaces".into());

        tracing::info!("默认 AI 代理: {}", default_agent);
        tracing::info!("用户工作区根目录: {}", project_root);

        Ok(Self {
            agents,
            default_agent,
            project_root: std::path::PathBuf::from(project_root),
            http_client: reqwest::Client::new(),
        })
    }

    /// 获取指定名称的代理配置，不存在则返回默认代理
    pub fn get_agent(&self, name: Option<&str>) -> &AgentConfig {
        let key = name.unwrap_or(&self.default_agent);
        self.agents.get(key)
            .or_else(|| self.agents.get(&self.default_agent))
            .unwrap() // agents 非空已在 new() 中保证
    }

    /// 获取某个用户的工作区目录
    /// 路径格式： {workspace_root}/{user_id}/
    /// 每个用户在这里开发自己的项目，不与其他用户共享
    pub fn get_user_workspace(&self, user_id: &str) -> std::path::PathBuf {
        // 用户 ID 只允许字母数字和下划线，防止路径穿越
        let safe_id: String = user_id
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .take(32)
            .collect();
        self.project_root.join(if safe_id.is_empty() { "default".into() } else { safe_id })
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
