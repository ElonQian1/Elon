/// 当前服务器 WS 协议版本号（单调递增）
pub const SERVER_PROTOCOL_VERSION: u32 = 1;
/// 服务器要求客户端支持的最低协议版本
pub const MIN_CLIENT_PROTOCOL_VERSION: u32 = 1;

/// WebSocket 消息格式（发给 APK）
#[derive(serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// AI 思考/操作进度
    Progress {
        message: String,
        /// 当前步骤编号（1 起），仅开发任务携带
        #[serde(skip_serializing_if = "Option::is_none")]
        step_current: Option<u32>,
        /// 总步骤数（开发任务固定 5），仅开发任务携带
        #[serde(skip_serializing_if = "Option::is_none")]
        step_total: Option<u32>,
        /// 阶段标识：ai_thinking / code_editing / code_committing / building / deploying
        #[serde(skip_serializing_if = "Option::is_none")]
        phase: Option<String>,
    },
    /// AI 给用户的中间发言（来自 Codex CLI 的 agent_message item）。
    /// 与最终 `done.message` 不同，本类型支持任务过程中多次出现，
    /// APK 端会渲染为白底主气泡，让用户感受到"AI 正在说话"。
    AssistantMessage { text: String },
    /// AI 正在执行的工具
    ToolCall {
        tool: String,
        args: serde_json::Value,
    },
    /// 工具执行结果
    ToolResult { tool: String, result: String },
    /// 本次 CLI 调用消耗的 token / 费用统计（来自 codex --json 的 token_count / turn.completed.usage）
    Usage {
        #[serde(skip_serializing_if = "Option::is_none")]
        input_tokens: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output_tokens: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        total_tokens: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cached_input_tokens: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reasoning_output_tokens: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        total_cost_usd: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
    /// 最终回复
    Done {
        message: String,
        apk_url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        image_url: Option<String>,
    },
    /// 发生错误
    Error {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        category: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        retryable: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        retry_after_secs: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        operator_detail: Option<String>,
    },
    /// WS 连接建立后第一帧握手：告知客户端服务器协议版本
    /// 容错方案：客户端若不认识此类型，忽略即可，不影响功能
    ProtocolHello {
        server_version: u32,
        min_client_version: u32,
    },
}

impl WsMessage {
    pub fn error(message: impl ToString) -> Self {
        WsMessage::Error {
            message: message.to_string(),
            code: None,
            category: None,
            retryable: None,
            retry_after_secs: None,
            operator_detail: None,
        }
    }

    pub fn classified_error(error: crate::errors::ClassifiedAiError) -> Self {
        WsMessage::Error {
            message: error.message,
            code: Some(error.code.to_string()),
            category: Some(error.category.as_str().to_string()),
            retryable: Some(error.retryable),
            retry_after_secs: error.retry_after_secs,
            operator_detail: error.operator_detail,
        }
    }

    /// 不携带步骤信息的普通进度消息（等效于旧 Progress { message }）
    pub fn progress(message: impl ToString) -> Self {
        WsMessage::Progress {
            message: message.to_string(),
            step_current: None,
            step_total: None,
            phase: None,
        }
    }

    /// 携带步骤编号的结构化进度消息
    pub fn progress_step(message: impl ToString, step: u32, total: u32, phase: &str) -> Self {
        WsMessage::Progress {
            message: message.to_string(),
            step_current: Some(step),
            step_total: Some(total),
            phase: Some(phase.to_string()),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self)
            .unwrap_or_else(|_| r#"{"type":"error","message":"序列化失败"}"#.into())
    }
}
