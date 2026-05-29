use serde::{Deserialize, Serialize};

pub const PROTO_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerToAgent {
    Exec {
        task_id: String,
        cli: String,
        args: Vec<String>,
        cwd: String,
        #[serde(default)]
        env: Vec<(String, String)>,
    },
    Cancel {
        task_id: String,
    },
    Ping {
        nonce: Option<String>,
    },
    /// 云端把 APK 的 HTTP 请求转发给 PC 端本地 server 处理
    HttpRequest {
        /// 唯一请求 ID，用于匹配响应
        req_id: String,
        method: String,
        /// 相对路径，如 "/health" 或 "/api/me"
        path: String,
        #[serde(default)]
        headers: Vec<(String, String)>,
        /// base64 编码的 body（GET 等无 body 时为 None）
        body_b64: Option<String>,
    },
    /// 云端把 AI 提示发给 PC，让 PC 用本地 CLI（copilot/codex）执行，流式返回结果
    CliPrompt {
        /// 唯一请求 ID，用于匹配 CliChunk/CliDone
        req_id: String,
        /// CLI 可执行文件名，如 "copilot" 或 "codex"
        cli: String,
        /// 额外参数（在 -p/prompt 之前），可为空
        #[serde(default)]
        extra_args: Vec<String>,
        /// 完整的提示内容
        prompt: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentToServer {
    Register {
        agent_id: String,
        version: String,
        proto_version: u32,
        #[serde(default)]
        allowed_clis: Vec<String>,
        #[serde(default)]
        allowed_cwds: Vec<String>,
    },
    TaskStarted {
        task_id: String,
        pid: u32,
    },
    TaskStdout {
        task_id: String,
        data: String,
    },
    TaskStderr {
        task_id: String,
        data: String,
    },
    TaskExit {
        task_id: String,
        code: Option<i32>,
    },
    TaskError {
        task_id: String,
        message: String,
    },
    Pong {
        nonce: Option<String>,
    },
    /// PC 端本地 server 处理完 HTTP 请求后回传响应
    HttpResponse {
        req_id: String,
        status: u16,
        #[serde(default)]
        headers: Vec<(String, String)>,
        /// base64 编码的响应 body
        body_b64: Option<String>,
    },
    /// PC 端处理失败（如本地 server 未启动）
    HttpError {
        req_id: String,
        message: String,
    },
    /// PC CLI 执行的流式输出片段
    CliChunk {
        req_id: String,
        text: String,
    },
    /// PC CLI 执行完毕（最后一帧）
    CliDone {
        req_id: String,
        exit_ok: bool,
        #[serde(default)]
        error: Option<String>,
    },
}

impl AgentToServer {
    pub fn task_id(&self) -> Option<&str> {
        match self {
            Self::TaskStarted { task_id, .. }
            | Self::TaskStdout { task_id, .. }
            | Self::TaskStderr { task_id, .. }
            | Self::TaskExit { task_id, .. }
            | Self::TaskError { task_id, .. } => Some(task_id.as_str()),
            Self::Register { .. }
            | Self::Pong { .. }
            | Self::HttpResponse { .. }
            | Self::HttpError { .. }
            | Self::CliChunk { .. }
            | Self::CliDone { .. } => None,
        }
    }

    pub fn req_id(&self) -> Option<&str> {
        match self {
            Self::HttpResponse { req_id, .. }
            | Self::HttpError { req_id, .. }
            | Self::CliChunk { req_id, .. }
            | Self::CliDone { req_id, .. } => Some(req_id.as_str()),
            _ => None,
        }
    }

    /// CliChunk 需要保留在 pending map 中（还有后续），其余 req_id 消息在发送后删除。
    pub fn is_final_req_msg(&self) -> bool {
        !matches!(self, Self::CliChunk { .. })
    }
}
