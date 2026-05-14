use anyhow::Result;
use std::sync::Arc;

/// 全局状态，在各路由间共享
pub struct AppState {
    /// AI API Key（从环境变量读取）
    pub ai_api_key: String,
    /// AI API 地址（支持 OpenAI 兼容接口）
    pub ai_api_base: String,
    /// 使用的模型名称
    pub ai_model: String,
    /// 项目根目录（AI 代理操作文件的沙箱目录）
    pub project_root: std::path::PathBuf,
    /// HTTP 客户端（复用连接）
    pub http_client: reqwest::Client,
}

impl AppState {
    pub fn new() -> Result<Self> {
        let ai_api_key = std::env::var("AI_API_KEY")
            .expect("必须设置环境变量 AI_API_KEY");
        let ai_api_base = std::env::var("AI_API_BASE")
            .unwrap_or_else(|_| "https://api.openai.com/v1".into());
        let ai_model = std::env::var("AI_MODEL")
            .unwrap_or_else(|_| "gpt-4o".into());
        let project_root = std::env::var("PROJECT_ROOT")
            .unwrap_or_else(|_| "/home/ubuntu/Elon".into());

        Ok(Self {
            ai_api_key,
            ai_api_base,
            ai_model,
            project_root: std::path::PathBuf::from(project_root),
            http_client: reqwest::Client::new(),
        })
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
