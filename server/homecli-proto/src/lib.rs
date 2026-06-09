use serde::{Deserialize, Serialize};

pub const PROTO_VERSION: u32 = 1;

/// PC 节点上报的单个模型能力描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapability {
    /// 模型唯一 ID，如 "llama3:8b" 或 "lm_studio/qwen2"
    pub model_id: String,
    /// 用户可见显示名称
    pub display_name: String,
    /// 上下文长度（token 数）
    pub context_len: u32,
    /// 提供方："ollama" | "lm_studio" | "custom"
    pub provider: String,
    /// 每 1000 tokens 消耗的平台积分（节点所有者自定义）
    pub price_per_1k_credits: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliProjectContext {
    pub project_id: String,
    pub conversation_id: String,
}

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
        /// 可选工作目录；为空时由 PC relay 使用自身默认工作目录
        #[serde(default)]
        cwd: Option<String>,
        /// 项目会话上下文。新版 PC 节点会用它把 CLI cwd 切到会话隔离 worktree；
        /// 老版节点忽略未知字段后仍可直接在 cwd 执行。
        #[serde(default)]
        project_context: Option<CliProjectContext>,
        /// 完整的提示内容
        prompt: String,
    },
    /// 云端向 PC 节点发起 LLM 推理请求（流式）
    LlmStreamRequest {
        req_id: String,
        model: String,
        messages: Vec<serde_json::Value>,
        #[serde(default)]
        max_tokens: Option<u32>,
    },
    /// 云端要求 PC 节点在受控根目录下创建一个新项目工作区。
    ProvisionProjectWorkspace {
        req_id: String,
        project_id: String,
        user_id: String,
        name: String,
        template: String,
    },
    /// 云端把 TTS 合成请求转发给有 GPU 的 PC 节点处理
    TtsSynthesizeRequest {
        req_id: String,
        text: String,
        #[serde(default)]
        voice_id: Option<String>,
        #[serde(default)]
        emotion_id: Option<String>,
        #[serde(default)]
        intensity: Option<String>,
        #[serde(default)]
        provider: Option<String>,
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
        /// 节点归属用户 ID（分布式节点功能，旧版 homecli 不发此字段）
        #[serde(default)]
        owner_user_id: Option<String>,
        /// PC 设备名，仅用于展示
        #[serde(default)]
        device_name: Option<String>,
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
        #[serde(default)]
        prompt_tokens: Option<u64>,
        #[serde(default)]
        cached_input_tokens: Option<u64>,
        #[serde(default)]
        completion_tokens: Option<u64>,
        #[serde(default)]
        reasoning_tokens: Option<u64>,
        #[serde(default)]
        total_tokens: Option<u64>,
        #[serde(default)]
        model: Option<String>,
    },
    /// PC 节点上报本机支持的模型列表
    RegisterCapabilities {
        models: Vec<ModelCapability>,
        /// 本机 TTS Worker HTTP 地址（如 http://127.0.0.1:5011），为空表示无 TTS 能力
        #[serde(default)]
        tts_worker_url: Option<String>,
    },
    /// LLM 推理流式输出片段
    LlmStreamChunk {
        req_id: String,
        delta: String,
    },
    /// LLM 推理完成（最后一帧）
    LlmStreamEnd {
        req_id: String,
        prompt_tokens: u32,
        completion_tokens: u32,
        finish_reason: String,
    },
    /// LLM 推理出错
    LlmStreamError {
        req_id: String,
        message: String,
    },
    /// PC 节点已创建或复用项目工作区。
    ProjectWorkspaceProvisioned {
        req_id: String,
        project_id: String,
        workspace_path: String,
        #[serde(default)]
        git_head: Option<String>,
        created: bool,
    },
    /// PC 节点创建项目工作区失败。
    ProjectWorkspaceProvisionError {
        req_id: String,
        project_id: String,
        message: String,
    },
    /// PC 节点 TTS 合成完成
    TtsSynthesizeResponse {
        req_id: String,
        /// base64 编码的音频字节
        audio_b64: String,
        /// MIME 类型，如 "audio/wav"
        mime: String,
        #[serde(default)]
        worker_voice: Option<String>,
    },
    /// PC 节点 TTS 合成失败
    TtsSynthesizeError {
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
            Self::Register { .. }
            | Self::Pong { .. }
            | Self::HttpResponse { .. }
            | Self::HttpError { .. }
            | Self::CliChunk { .. }
            | Self::CliDone { .. }
            | Self::RegisterCapabilities { .. }
            | Self::LlmStreamChunk { .. }
            | Self::LlmStreamEnd { .. }
            | Self::LlmStreamError { .. }
            | Self::ProjectWorkspaceProvisioned { .. }
            | Self::ProjectWorkspaceProvisionError { .. }
            | Self::TtsSynthesizeResponse { .. }
            | Self::TtsSynthesizeError { .. } => None,
        }
    }

    pub fn req_id(&self) -> Option<&str> {
        match self {
            Self::HttpResponse { req_id, .. }
            | Self::HttpError { req_id, .. }
            | Self::CliChunk { req_id, .. }
            | Self::CliDone { req_id, .. }
            | Self::LlmStreamChunk { req_id, .. }
            | Self::LlmStreamEnd { req_id, .. }
            | Self::LlmStreamError { req_id, .. }
            | Self::ProjectWorkspaceProvisioned { req_id, .. }
            | Self::ProjectWorkspaceProvisionError { req_id, .. }
            | Self::TtsSynthesizeResponse { req_id, .. }
            | Self::TtsSynthesizeError { req_id, .. } => Some(req_id.as_str()),
            _ => None,
        }
    }

    /// 流式消息需要保留在 pending map 中（还有后续），其余 req_id 消息在发送后删除。
    pub fn is_final_req_msg(&self) -> bool {
        !matches!(self, Self::CliChunk { .. } | Self::LlmStreamChunk { .. })
    }
}
