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
}

impl AgentToServer {
    pub fn task_id(&self) -> Option<&str> {
        match self {
            Self::TaskStarted { task_id, .. }
            | Self::TaskStdout { task_id, .. }
            | Self::TaskStderr { task_id, .. }
            | Self::TaskExit { task_id, .. }
            | Self::TaskError { task_id, .. } => Some(task_id.as_str()),
            Self::Register { .. } | Self::Pong { .. } | Self::HttpResponse { .. } | Self::HttpError { .. } => None,
        }
    }

    pub fn req_id(&self) -> Option<&str> {
        match self {
            Self::HttpResponse { req_id, .. } | Self::HttpError { req_id, .. } => Some(req_id.as_str()),
            _ => None,
        }
    }
}
