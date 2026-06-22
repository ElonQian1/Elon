// server/src/server_agent_runtime_limits.rs

use serde::Serialize;
use serde_json::Value;

const MAX_MESSAGES: usize = 24;
const MAX_MESSAGE_CHARS: usize = 32_000;
const MAX_TOTAL_CHARS: usize = 80_000;
const MAX_OUTPUT_TOKENS: usize = 3000;
const MAX_REQUESTS_PER_MINUTE: usize = 12;
const MAX_CONCURRENT_PER_USER: usize = 2;
const TEMPERATURE: f64 = 0.2;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerAgentRuntimeLimits {
    pub max_messages: usize,
    pub max_message_chars: usize,
    pub max_total_chars: usize,
    pub max_output_tokens: usize,
    pub max_requests_per_minute: usize,
    pub max_concurrent_per_user: usize,
    pub temperature: f64,
}

impl ServerAgentRuntimeLimits {
    pub(crate) const fn current() -> Self {
        Self {
            max_messages: MAX_MESSAGES,
            max_message_chars: MAX_MESSAGE_CHARS,
            max_total_chars: MAX_TOTAL_CHARS,
            max_output_tokens: MAX_OUTPUT_TOKENS,
            max_requests_per_minute: MAX_REQUESTS_PER_MINUTE,
            max_concurrent_per_user: MAX_CONCURRENT_PER_USER,
            temperature: TEMPERATURE,
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

#[cfg(test)]
mod tests {
    use super::ServerAgentRuntimeLimits;
    use serde_json::json;

    #[test]
    fn accepts_normal_runtime_messages() {
        let messages = vec![
            json!({"role": "system", "content": "Return JSON"}),
            json!({"role": "user", "content": "Read README"}),
        ];

        ServerAgentRuntimeLimits::current()
            .validate_messages(&messages)
            .unwrap();
    }

    #[test]
    fn rejects_tool_role_messages() {
        let messages = vec![json!({"role": "tool", "content": "result"})];

        assert!(ServerAgentRuntimeLimits::current()
            .validate_messages(&messages)
            .is_err());
    }

    #[test]
    fn rejects_empty_messages() {
        assert!(ServerAgentRuntimeLimits::current()
            .validate_messages(&[])
            .is_err());
    }

    #[test]
    fn rejects_messages_over_operational_limits() {
        let limits = ServerAgentRuntimeLimits::current();
        let too_many = (0..=limits.max_messages)
            .map(|_| json!({"role": "user", "content": "x"}))
            .collect::<Vec<_>>();
        assert!(limits.validate_messages(&too_many).is_err());

        let too_long = vec![json!({
            "role": "user",
            "content": "x".repeat(limits.max_total_chars + 1)
        })];
        assert!(limits.validate_messages(&too_long).is_err());

        let too_long_single_message = vec![json!({
            "role": "user",
            "content": "x".repeat(limits.max_message_chars + 1)
        })];
        assert!(limits.validate_messages(&too_long_single_message).is_err());
    }
}
