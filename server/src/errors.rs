//! 统一 HTTP 错误类型
//!
//! 现有 handler 仍可使用 `project_auth::json_error`（保持向后兼容）；
//! 新 handler 推荐直接返回 `AppError`，编译器自动将其转为带状态码的 JSON 响应。
//!
//! # 示例
//! ```rust
//! async fn my_handler(...) -> Result<Json<MyResp>, AppError> {
//!     let user = state.store.get_user(&id)
//!         .map_err(|_| AppError::not_found("用户不存在"))?;
//!     Ok(Json(user))
//! }
//! ```

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

/// 应用级 HTTP 错误枚举，每个变体映射到标准 HTTP 状态码。
#[derive(Debug, Error)]
pub enum AppError {
    #[error("未找到：{0}")]
    NotFound(String),

    #[error("未授权：{0}")]
    Unauthorized(String),

    #[error("请求无效：{0}")]
    BadRequest(String),

    #[error("权限不足：{0}")]
    Forbidden(String),

    #[error("服务器内部错误：{0}")]
    Internal(String),

    #[error("资源已存在：{0}")]
    Conflict(String),

    #[error("需要升级客户端：{0}")]
    UpgradeRequired(String),
}

impl AppError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::Unauthorized(msg.into())
    }
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::Forbidden(msg.into())
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::UpgradeRequired(_) => StatusCode::UPGRADE_REQUIRED,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = Json(serde_json::json!({
            "error": self.to_string(),
            "code": status.as_u16(),
        }));
        (status, body).into_response()
    }
}

/// `anyhow::Error` → `AppError::Internal`（无需细分错误类型时使用）
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

// ── AI 错误分类（原 ai_error.rs，合并于此统一管理）────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiErrorCategory {
    TemporaryCapacity,
    ProviderConnection,
    Timeout,
    RateLimited,
    Quota,
    AuthConfig,
    Workspace,
    Unknown,
}

impl AiErrorCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            AiErrorCategory::TemporaryCapacity => "temporary_capacity",
            AiErrorCategory::ProviderConnection => "provider_connection",
            AiErrorCategory::Timeout => "timeout",
            AiErrorCategory::RateLimited => "rate_limited",
            AiErrorCategory::Quota => "quota",
            AiErrorCategory::AuthConfig => "auth_config",
            AiErrorCategory::Workspace => "workspace",
            AiErrorCategory::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassifiedAiError {
    pub code: &'static str,
    pub category: AiErrorCategory,
    pub retryable: bool,
    pub retry_after_secs: Option<u64>,
    pub message: String,
    pub operator_detail: Option<String>,
}

impl ClassifiedAiError {
    pub fn should_retry_local_cli(&self) -> bool {
        self.retryable
            && matches!(
                self.category,
                AiErrorCategory::TemporaryCapacity
                    | AiErrorCategory::ProviderConnection
                    | AiErrorCategory::Timeout
                    | AiErrorCategory::RateLimited
            )
    }
}

pub fn classify_ai_error(raw: &str) -> ClassifiedAiError {
    let compact = compact_detail(raw);
    let lower = compact.to_ascii_lowercase();
    let detail = (!compact.is_empty()).then_some(compact);

    if contains_codex_auth_json_error(&lower) {
        return ClassifiedAiError {
            code: "codex_auth_json_invalid",
            category: AiErrorCategory::AuthConfig,
            retryable: false,
            retry_after_secs: None,
            message:
                "当前 Codex 账号登录已失效，auth.json 无法刷新。请账号所有者在对应 PC 上重新登录 Codex，并重新备份到保险箱后再试。"
                    .into(),
            operator_detail: detail,
        };
    }

    if contains_codex_usage_limit_error(&lower) {
        return ClassifiedAiError {
            code: "codex_usage_limit_exhausted",
            category: AiErrorCategory::Quota,
            retryable: false,
            retry_after_secs: None,
            message:
                "当前 Codex 账号额度已用尽或被限流，本轮没有完成。系统已尝试可用的共享账号；如果共享账号也失败，请切换其他授权账号或等待额度恢复后重试。"
                    .into(),
            operator_detail: detail,
        };
    }

    if contains_auth_error(&lower) {
        return ClassifiedAiError {
            code: "ai_auth_config_error",
            category: AiErrorCategory::AuthConfig,
            retryable: false,
            retry_after_secs: None,
            message:
                "当前 AI 服务配置异常，密钥或权限校验没有通过。请管理员检查服务端 AI 配置后再重试。"
                    .into(),
            operator_detail: detail,
        };
    }

    if contains_model_config_error(&lower) {
        return ClassifiedAiError {
            code: "ai_model_config_unavailable",
            category: AiErrorCategory::AuthConfig,
            retryable: false,
            retry_after_secs: None,
            message:
                "当前平台 AI 模型已下线或模型服务 ID 配置不正确。请管理员迁移到 TokenHub 的可用模型，或切换到其他有效模型通道后再试。"
                    .into(),
            operator_detail: detail,
        };
    }

    if contains_quota_error(&lower) {
        return ClassifiedAiError {
            code: "ai_quota_unavailable",
            category: AiErrorCategory::Quota,
            retryable: false,
            retry_after_secs: None,
            message:
                "当前 AI 模型额度已用尽或接口不可用。请切换可用模型，或联系管理员补充额度后重试。"
                    .into(),
            operator_detail: detail,
        };
    }

    if contains_rate_limit_error(&lower) {
        return ClassifiedAiError {
            code: "ai_rate_limited",
            category: AiErrorCategory::RateLimited,
            retryable: true,
            retry_after_secs: Some(45),
            message: "服务器 AI 通道当前请求过多，本轮没有完成，也不会继续在后台处理。请稍后重新发送即可继续。".into(),
            operator_detail: detail,
        };
    }

    if contains_codex_capacity_error(&lower) {
        return ClassifiedAiError {
            code: "ai_service_busy",
            category: AiErrorCategory::TemporaryCapacity,
            retryable: true,
            retry_after_secs: Some(15),
            message: "服务器 AI 通道刚才拥堵或短暂断开，本轮没有完成，也不会继续在后台处理。稍后重新发送即可从当前项目记录继续。".into(),
            operator_detail: detail,
        };
    }

    if contains_timeout_error(&lower) {
        return ClassifiedAiError {
            code: "ai_service_timeout",
            category: AiErrorCategory::Timeout,
            retryable: true,
            retry_after_secs: Some(20),
            message: "服务器 AI 通道响应超时，本轮没有完成，也不会继续在后台处理。请稍后重新发送即可继续。".into(),
            operator_detail: detail,
        };
    }

    if contains_provider_connection_error(&lower) {
        return ClassifiedAiError {
            code: "ai_provider_connection_unstable",
            category: AiErrorCategory::ProviderConnection,
            retryable: true,
            retry_after_secs: Some(20),
            message: "服务器连接 AI 服务时出现短暂不稳定，本轮没有完成，也不会继续在后台处理。请稍后重新发送。".into(),
            operator_detail: detail,
        };
    }

    if contains_workspace_error(&lower) {
        return ClassifiedAiError {
            code: "project_workspace_error",
            category: AiErrorCategory::Workspace,
            retryable: false,
            retry_after_secs: None,
            message: dirty_conversation_worktree_message(&lower)
                .or_else(|| no_project_changes_message(&lower))
                .unwrap_or_else(|| {
                    detail
                        .as_deref()
                        .map(|value| truncate_chars(value, 180))
                        .filter(|value| !value.is_empty())
                        .unwrap_or_else(|| {
                            "项目工作区准备失败，请检查项目 Git/worktree 状态后重试。".into()
                        })
                }),
            operator_detail: detail,
        };
    }

    ClassifiedAiError {
        code: "ai_unknown_error",
        category: AiErrorCategory::Unknown,
        retryable: false,
        retry_after_secs: None,
        message: detail
            .as_deref()
            .map(|value| truncate_chars(value, 180))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "AI 服务暂时不可用，请稍后重试。".into()),
        operator_detail: detail,
    }
}

pub fn transient_retry_attempts() -> usize {
    std::env::var("AI_TRANSIENT_ERROR_MAX_RETRIES")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|attempts| *attempts <= 3)
        .unwrap_or(1)
}

pub fn transient_retry_delay_secs(attempt_index: usize) -> u64 {
    let base = std::env::var("AI_TRANSIENT_ERROR_RETRY_DELAY_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|secs| (1..=60).contains(secs))
        .unwrap_or(5);
    base.saturating_mul(attempt_index.max(1) as u64).min(60)
}

fn contains_codex_capacity_error(lower: &str) -> bool {
    lower.contains("codex cli network unhealthy")
        || lower.contains("responses websocket failed")
        || lower.contains("required provider endpoints are unreachable")
        || lower.contains("stream disconnected before completion")
        || lower.contains("reachability") && lower.contains("unreachable")
        || lower.contains("websocket") && (lower.contains("failed") || lower.contains("timeout"))
}

fn contains_rate_limit_error(lower: &str) -> bool {
    lower.contains("rate limit") || lower.contains("too many requests") || lower.contains("429")
}

fn contains_quota_error(lower: &str) -> bool {
    lower.contains("free_quota_exhausted")
        || lower.contains("payment required")
        || lower.contains("free trial quota")
        || lower.contains("postpaid billing is not enabled")
        || lower.contains("insufficient quota")
        || lower.contains("quota exceeded")
        || lower.contains("usage limit")
        || lower.contains("limit reached")
        || lower.contains("usage exhausted")
        || lower.contains("额度已用尽")
        || lower.contains("endpoint is inactive")
}

fn contains_model_config_error(lower: &str) -> bool {
    lower.contains("该模型已下线")
        || lower.contains("模型已下线")
        || lower.contains("当前 ai 模型已下线")
        || lower.contains("model has been discontinued")
        || lower.contains("model is discontinued")
        || lower.contains("\"code\":\"2030\"")
        || lower.contains("\"code\":2030")
        || lower.contains("model or service id") && lower.contains("does not exist")
}

fn contains_codex_usage_limit_error(lower: &str) -> bool {
    (lower.contains("codex") || lower.contains("openai"))
        && (lower.contains("hit your usage limit")
            || lower.contains("usage limit")
            || lower.contains("usage exhausted")
            || lower.contains("额度已用尽"))
}

fn contains_codex_auth_json_error(lower: &str) -> bool {
    lower.contains("refresh_token_reused")
        || lower.contains("token_expired")
        || lower.contains("failed to refresh token")
        || lower.contains("refresh token has already been used")
        || lower.contains("your refresh token")
        || lower.contains("codex") && lower.contains("401 unauthorized")
}

fn contains_auth_error(lower: &str) -> bool {
    lower.contains("unauthorized")
        || lower.contains("invalid api key")
        || lower.contains("incorrect api key")
        || lower.contains("api key") && lower.contains("invalid")
        || lower.contains("permission denied")
}

fn contains_timeout_error(lower: &str) -> bool {
    lower.contains("timed out")
        || lower.contains("timeout")
        || lower.contains("执行超时")
        || lower.contains("request timed out")
        || lower.contains("connection timed out")
}

fn contains_provider_connection_error(lower: &str) -> bool {
    lower.contains("network is unreachable")
        || lower.contains("failed to connect")
        || lower.contains("connection reset")
        || lower.contains("error sending request")
        || lower.contains("http/request failed")
        || lower.contains("proxy connect")
        || lower.contains("tls handshake eof")
        || lower.contains("ssl_error_syscall")
}

fn contains_workspace_error(lower: &str) -> bool {
    lower.contains("git pull")
        || lower.contains("git/local_path")
        || lower.contains("worktree")
        || lower.contains("not a git repository")
        || lower.contains("合并回项目主分支失败")
        || lower.contains("没有产生新提交")
        || lower.contains("没有实际修改项目")
        || lower.contains("conversation branch had no new commits")
}

fn dirty_conversation_worktree_message(lower: &str) -> Option<String> {
    lower
        .contains("conversation worktree still has uncommitted changes")
        .then(|| {
            "项目会话工作区里还有未提交改动，本轮改动已保留但暂时不能自动合并。请稍后重试；如果仍失败，需要在 PC 节点提交或清理该会话工作区。".into()
        })
}

fn no_project_changes_message(lower: &str) -> Option<String> {
    (lower.contains("没有产生新提交")
        || lower.contains("没有实际修改项目")
        || lower.contains("conversation branch had no new commits"))
    .then(|| "开发助手本轮没有实际修改项目，所以我没有把它标记为已完成。请重新发送需求，或切换可用 PC 节点后再试。".into())
}

fn compact_detail(raw: &str) -> String {
    truncate_chars(&raw.split_whitespace().collect::<Vec<_>>().join(" "), 700)
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut value = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        value.push_str("...");
    }
    value
}

#[cfg(test)]
#[path = "errors_tests.rs"]
mod errors_tests;
