// server/src/server_agent_runtime_guard.rs

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub(crate) use crate::agent_runtime_error_summary::operational_error_summary;
use crate::server_agent_runtime_limits::ServerAgentRuntimeLimits;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerRuntimeProtectionStatus {
    pub input_validation: &'static str,
    pub billing_gate: &'static str,
    pub audit: &'static str,
    pub request_fingerprint: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerRuntimeAuditSummary {
    pub request_fingerprint: String,
    pub message_count: usize,
    pub total_chars: usize,
    pub max_message_chars: usize,
    pub roles: Vec<String>,
    pub limit_max_messages: usize,
    pub limit_max_total_chars: usize,
    pub limit_max_output_tokens: usize,
}

pub(crate) fn protection_status() -> ServerRuntimeProtectionStatus {
    ServerRuntimeProtectionStatus {
        input_validation: "messages role/content/count/total_chars",
        billing_gate: "shared with call_chat_llm_with_options",
        audit:
            "logs user_id, agent, model, message counts, char totals, and request fingerprint only",
        request_fingerprint:
            "sha256 over roles, content lengths, and content hashes; prompt/error text is not logged",
    }
}

pub(crate) fn audit_summary(
    messages: &[Value],
    limits: ServerAgentRuntimeLimits,
) -> ServerRuntimeAuditSummary {
    let mut roles = Vec::with_capacity(messages.len());
    let mut total_chars = 0usize;
    let mut max_message_chars = 0usize;
    let mut hasher = Sha256::new();

    for message in messages {
        let role = message
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("<invalid>")
            .trim()
            .to_string();
        let content_chars = message
            .get("content")
            .and_then(Value::as_str)
            .map(|content| content.chars().count())
            .unwrap_or_default();
        let content_digest = message
            .get("content")
            .and_then(Value::as_str)
            .map(|content| Sha256::digest(content.as_bytes()));
        total_chars += content_chars;
        max_message_chars = max_message_chars.max(content_chars);
        roles.push(role.clone());

        hasher.update(role.as_bytes());
        hasher.update([0]);
        hasher.update(content_chars.to_le_bytes());
        hasher.update([0]);
        if let Some(digest) = content_digest {
            hasher.update(digest);
        }
        hasher.update([0]);
    }

    ServerRuntimeAuditSummary {
        request_fingerprint: hex::encode(hasher.finalize()),
        message_count: messages.len(),
        total_chars,
        max_message_chars,
        roles,
        limit_max_messages: limits.max_messages,
        limit_max_total_chars: limits.max_total_chars,
        limit_max_output_tokens: limits.max_output_tokens,
    }
}

#[cfg(test)]
mod tests {
    use super::{audit_summary, operational_error_summary, protection_status};
    use crate::server_agent_runtime_limits::ServerAgentRuntimeLimits;
    use serde_json::json;

    #[test]
    fn audit_summary_uses_shape_not_prompt_text() {
        let limits = ServerAgentRuntimeLimits::current();
        let left = vec![json!({"role": "user", "content": "secret prompt A"})];
        let right = vec![json!({"role": "user", "content": "secret prompt B"})];

        let left_summary = audit_summary(&left, limits);
        let right_summary = audit_summary(&right, limits);

        assert_eq!(left_summary.message_count, 1);
        assert_eq!(left_summary.total_chars, "secret prompt A".chars().count());
        assert_eq!(left_summary.roles, vec!["user"]);
        assert_ne!(
            left_summary.request_fingerprint,
            right_summary.request_fingerprint
        );
        let serialized = serde_json::to_string(&left_summary).unwrap();
        assert!(!serialized.contains("secret prompt"));
    }

    #[test]
    fn fingerprint_changes_when_shape_changes() {
        let limits = ServerAgentRuntimeLimits::current();
        let one = audit_summary(&[json!({"role": "user", "content": "abc"})], limits);
        let two = audit_summary(&[json!({"role": "assistant", "content": "abc"})], limits);
        let three = audit_summary(&[json!({"role": "user", "content": "abcd"})], limits);

        assert_ne!(one.request_fingerprint, two.request_fingerprint);
        assert_ne!(one.request_fingerprint, three.request_fingerprint);
    }

    #[test]
    fn status_describes_operational_protections() {
        let status = protection_status();
        assert!(status.input_validation.contains("total_chars"));
        assert!(status.billing_gate.contains("call_chat_llm"));
        assert!(status.audit.contains("fingerprint"));
    }

    #[test]
    fn operational_error_summary_omits_error_body() {
        let body = "provider returned secret-token and user prompt text";
        let summary = operational_error_summary(body);

        assert!(summary.contains("provider_error"));
        assert!(summary.contains("chars="));
        assert!(summary.contains("fingerprint="));
        assert!(!summary.contains("secret-token"));
        assert!(!summary.contains("user prompt text"));
    }
}
