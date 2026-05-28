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
            message: "服务器 AI 通道当前请求过多，系统会先自动重试。若最终仍看到这条提示，说明本轮已暂停，稍后重新发送即可继续。".into(),
            operator_detail: detail,
        };
    }

    if contains_codex_capacity_error(&lower) {
        return ClassifiedAiError {
            code: "ai_service_busy",
            category: AiErrorCategory::TemporaryCapacity,
            retryable: true,
            retry_after_secs: Some(15),
            message: "服务器 AI 通道刚才拥堵或短暂断开，系统会先自动重试。手机 WebSocket 临时断开会自动重连并同步进度；如果连续重试后仍看到这条提示，说明本轮已暂停，稍后重新发送即可从当前项目记录继续。".into(),
            operator_detail: detail,
        };
    }

    if contains_timeout_error(&lower) {
        return ClassifiedAiError {
            code: "ai_service_timeout",
            category: AiErrorCategory::Timeout,
            retryable: true,
            retry_after_secs: Some(20),
            message: "服务器 AI 通道响应超时，系统会先自动重试。若最终仍看到这条提示，说明本轮没有完成，稍后重新发送即可继续。".into(),
            operator_detail: detail,
        };
    }

    if contains_provider_connection_error(&lower) {
        return ClassifiedAiError {
            code: "ai_provider_connection_unstable",
            category: AiErrorCategory::ProviderConnection,
            retryable: true,
            retry_after_secs: Some(20),
            message: "服务器连接 AI 服务时出现短暂不稳定，系统会先自动重试。手机连接会继续自动恢复；若最终仍失败，请稍后重新发送。".into(),
            operator_detail: detail,
        };
    }

    if contains_workspace_error(&lower) {
        return ClassifiedAiError {
            code: "project_workspace_error",
            category: AiErrorCategory::Workspace,
            retryable: false,
            retry_after_secs: None,
            message: detail
                .as_deref()
                .map(|value| truncate_chars(value, 180))
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| {
                    "项目工作区准备失败，请检查项目 Git/worktree 状态后重试。".into()
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
        || lower.contains("insufficient quota")
        || lower.contains("quota exceeded")
        || lower.contains("endpoint is inactive")
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
mod tests {
    use super::*;

    #[test]
    fn classifies_codex_websocket_failure_as_retryable_capacity() {
        let classified =
            classify_ai_error("Codex CLI network unhealthy: Responses WebSocket failed");
        assert_eq!(classified.code, "ai_service_busy");
        assert!(classified.retryable);
        assert!(classified.should_retry_local_cli());
    }

    #[test]
    fn classifies_provider_reachability_as_retryable_capacity() {
        let classified = classify_ai_error(
            "reachability one or more required provider endpoints are unreachable over HTTP",
        );
        assert_eq!(classified.code, "ai_service_busy");
        assert_eq!(classified.category, AiErrorCategory::TemporaryCapacity);
    }

    #[test]
    fn classifies_auth_error_as_non_retryable() {
        let classified = classify_ai_error("invalid api key");
        assert_eq!(classified.code, "ai_auth_config_error");
        assert!(!classified.retryable);
    }
}
