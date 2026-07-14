// server/src/server_agent_runtime_limits.rs

use serde::Serialize;
use serde_json::Value;

const MAX_MESSAGES: usize = 24;
const MAX_MESSAGE_CHARS: usize = 32_000;
const MAX_TOTAL_CHARS: usize = 80_000;
const MAX_OUTPUT_TOKENS: usize = 3000;
const MAX_ACTIONS: usize = 24;
const MAX_ACTION_CHARS: usize = 64_000;
const MAX_ACTIONS_TOTAL_CHARS: usize = 96_000;
const MAX_REQUESTS_PER_MINUTE: usize = 12;
const MAX_CONCURRENT_PER_USER: usize = 2;
const MAX_CONCURRENT_GLOBAL: usize = 24;
const DUPLICATE_REQUEST_WINDOW_SECS: usize = 5;
const TEMPERATURE: f64 = 0.2;

const MAX_MESSAGES_ENV: &str = "ELON_SERVER_AGENT_RUNTIME_MAX_MESSAGES";
const MAX_MESSAGE_CHARS_ENV: &str = "ELON_SERVER_AGENT_RUNTIME_MAX_MESSAGE_CHARS";
const MAX_TOTAL_CHARS_ENV: &str = "ELON_SERVER_AGENT_RUNTIME_MAX_TOTAL_CHARS";
const MAX_OUTPUT_TOKENS_ENV: &str = "ELON_SERVER_AGENT_RUNTIME_MAX_OUTPUT_TOKENS";
const MAX_ACTIONS_ENV: &str = "ELON_SERVER_AGENT_RUNTIME_MAX_ACTIONS";
const MAX_ACTION_CHARS_ENV: &str = "ELON_SERVER_AGENT_RUNTIME_MAX_ACTION_CHARS";
const MAX_ACTIONS_TOTAL_CHARS_ENV: &str = "ELON_SERVER_AGENT_RUNTIME_MAX_ACTIONS_TOTAL_CHARS";
const MAX_REQUESTS_PER_MINUTE_ENV: &str = "ELON_SERVER_AGENT_RUNTIME_MAX_REQUESTS_PER_MINUTE";
const MAX_CONCURRENT_PER_USER_ENV: &str = "ELON_SERVER_AGENT_RUNTIME_MAX_CONCURRENT_PER_USER";
const MAX_CONCURRENT_GLOBAL_ENV: &str = "ELON_SERVER_AGENT_RUNTIME_MAX_CONCURRENT_GLOBAL";
const DUPLICATE_REQUEST_WINDOW_SECS_ENV: &str = "ELON_SERVER_AGENT_RUNTIME_DUPLICATE_WINDOW_SECS";
const TEMPERATURE_ENV: &str = "ELON_SERVER_AGENT_RUNTIME_TEMPERATURE";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerAgentRuntimeLimits {
    pub max_messages: usize,
    pub max_message_chars: usize,
    pub max_total_chars: usize,
    pub max_output_tokens: usize,
    pub max_actions: usize,
    pub max_action_chars: usize,
    pub max_actions_total_chars: usize,
    pub max_requests_per_minute: usize,
    pub max_concurrent_per_user: usize,
    pub max_concurrent_global: usize,
    pub duplicate_request_window_secs: usize,
    pub temperature: f64,
}

impl ServerAgentRuntimeLimits {
    pub(crate) fn current() -> Self {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    fn defaults() -> Self {
        Self {
            max_messages: MAX_MESSAGES,
            max_message_chars: MAX_MESSAGE_CHARS,
            max_total_chars: MAX_TOTAL_CHARS,
            max_output_tokens: MAX_OUTPUT_TOKENS,
            max_actions: MAX_ACTIONS,
            max_action_chars: MAX_ACTION_CHARS,
            max_actions_total_chars: MAX_ACTIONS_TOTAL_CHARS,
            max_requests_per_minute: MAX_REQUESTS_PER_MINUTE,
            max_concurrent_per_user: MAX_CONCURRENT_PER_USER,
            max_concurrent_global: MAX_CONCURRENT_GLOBAL,
            duplicate_request_window_secs: DUPLICATE_REQUEST_WINDOW_SECS,
            temperature: TEMPERATURE,
        }
    }

    fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Self {
        let defaults = Self::defaults();
        Self {
            max_messages: env_usize(&mut lookup, MAX_MESSAGES_ENV, defaults.max_messages, 1, 64),
            max_message_chars: env_usize(
                &mut lookup,
                MAX_MESSAGE_CHARS_ENV,
                defaults.max_message_chars,
                1_000,
                128_000,
            ),
            max_total_chars: env_usize(
                &mut lookup,
                MAX_TOTAL_CHARS_ENV,
                defaults.max_total_chars,
                1_000,
                240_000,
            ),
            max_output_tokens: env_usize(
                &mut lookup,
                MAX_OUTPUT_TOKENS_ENV,
                defaults.max_output_tokens,
                256,
                12_000,
            ),
            max_actions: env_usize(&mut lookup, MAX_ACTIONS_ENV, defaults.max_actions, 0, 64),
            max_action_chars: env_usize(
                &mut lookup,
                MAX_ACTION_CHARS_ENV,
                defaults.max_action_chars,
                1_000,
                240_000,
            ),
            max_actions_total_chars: env_usize(
                &mut lookup,
                MAX_ACTIONS_TOTAL_CHARS_ENV,
                defaults.max_actions_total_chars,
                1_000,
                500_000,
            ),
            max_requests_per_minute: env_usize(
                &mut lookup,
                MAX_REQUESTS_PER_MINUTE_ENV,
                defaults.max_requests_per_minute,
                1,
                120,
            ),
            max_concurrent_per_user: env_usize(
                &mut lookup,
                MAX_CONCURRENT_PER_USER_ENV,
                defaults.max_concurrent_per_user,
                1,
                16,
            ),
            max_concurrent_global: env_usize(
                &mut lookup,
                MAX_CONCURRENT_GLOBAL_ENV,
                defaults.max_concurrent_global,
                1,
                128,
            ),
            duplicate_request_window_secs: env_usize(
                &mut lookup,
                DUPLICATE_REQUEST_WINDOW_SECS_ENV,
                defaults.duplicate_request_window_secs,
                0,
                300,
            ),
            temperature: env_f64(&mut lookup, TEMPERATURE_ENV, defaults.temperature, 0.0, 2.0),
        }
    }

    pub(crate) fn validate_messages(self, messages: &[Value]) -> Result<(), &'static str> {
        if messages.is_empty() {
            return Err("messages 不能为空");
        }
        if messages.len() > self.max_messages {
            return Err("messages 过多");
        }

        let mut total_chars = 0usize;
        for message in messages {
            let Some(object) = message.as_object() else {
                return Err("message 必须是对象");
            };
            let role = object
                .get("role")
                .and_then(Value::as_str)
                .ok_or("message.role 不能为空")?;
            if !matches!(role, "system" | "user" | "assistant") {
                return Err("message.role 只允许 system/user/assistant");
            }
            let content = object
                .get("content")
                .and_then(Value::as_str)
                .ok_or("message.content 不能为空")?;
            if content.trim().is_empty() {
                return Err("message.content 不能为空");
            }
            let content_chars = content.chars().count();
            if content_chars > self.max_message_chars {
                return Err("message.content 过长");
            }
            total_chars += content_chars;
            if total_chars > self.max_total_chars {
                return Err("messages 内容过长");
            }
        }

        Ok(())
    }
}

fn env_usize(
    lookup: &mut impl FnMut(&str) -> Option<String>,
    name: &str,
    default: usize,
    min: usize,
    max: usize,
) -> usize {
    lookup(name)
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| (*value >= min) && (*value <= max))
        .unwrap_or(default)
}

fn env_f64(
    lookup: &mut impl FnMut(&str) -> Option<String>,
    name: &str,
    default: f64,
    min: f64,
    max: f64,
) -> f64 {
    lookup(name)
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite() && (*value >= min) && (*value <= max))
        .unwrap_or(default)
}

#[cfg(test)]
#[path = "server_agent_runtime_limits_tests.rs"]
mod tests;
