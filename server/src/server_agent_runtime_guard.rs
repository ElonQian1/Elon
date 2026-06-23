// server/src/server_agent_runtime_guard.rs

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, VecDeque},
    fmt,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

pub(crate) use crate::agent_runtime_error_summary::operational_error_summary;
use crate::server_agent_runtime_limits::ServerAgentRuntimeLimits;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerRuntimeProtectionStatus {
    pub input_validation: &'static str,
    pub output_validation: &'static str,
    pub agent_selection: &'static str,
    pub admission_control: &'static str,
    pub budget_gate: &'static str,
    pub operational_switch: &'static str,
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
    pub limit_max_message_chars: usize,
    pub limit_max_total_chars: usize,
    pub limit_max_output_tokens: usize,
    pub limit_max_actions: usize,
    pub limit_max_action_chars: usize,
    pub limit_max_actions_total_chars: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerRuntimeAdmissionSnapshot {
    pub in_flight_global: usize,
    pub max_concurrent_global: usize,
    pub remaining_concurrent_global: usize,
    pub in_flight_for_user: usize,
    pub max_concurrent_per_user: usize,
    pub remaining_concurrent_for_user: usize,
    pub recent_requests_per_minute: usize,
    pub max_requests_per_minute: usize,
    pub remaining_requests_per_minute: usize,
    pub rate_limit_retry_after_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerRuntimeAdmissionAvailability {
    pub ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_message: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ServerRuntimeAdmissionError {
    TooManyGlobalConcurrent {
        max_concurrent_global: usize,
    },
    TooManyConcurrent {
        max_concurrent_per_user: usize,
    },
    RateLimited {
        max_requests_per_minute: usize,
        retry_after_secs: u64,
    },
}

#[derive(Debug)]
pub(crate) struct ServerRuntimeAdmissionGuard {
    user_id: String,
    released: bool,
}

pub(crate) fn protection_status() -> ServerRuntimeProtectionStatus {
    ServerRuntimeProtectionStatus {
        input_validation: "messages role/content/count/message_chars/total_chars",
        output_validation:
            "model JSON must be an object; actions must stay within count/action_chars/total_action_chars",
        agent_selection:
            "default server agent only unless ELON_SERVER_AGENT_RUNTIME_ALLOWED_AGENTS explicitly allows more",
        admission_control: "global and per-user concurrency plus rolling minute request limits",
        budget_gate:
            "optional ELON_SERVER_AGENT_RUNTIME_DAILY_CALL_LIMIT daily platform call fuse",
        operational_switch: "ELON_SERVER_AGENT_RUNTIME_ENABLED can disable Route C without redeploy",
        billing_gate: "shared with call_chat_llm_with_options",
        audit:
            "logs user_id, agent, model, message counts, char totals, and request fingerprint only",
        request_fingerprint:
            "sha256 over roles, content lengths, and content hashes; prompt/error text is not logged",
    }
}

pub(crate) fn try_acquire_runtime_admission(
    user_id: &str,
    limits: ServerAgentRuntimeLimits,
) -> Result<ServerRuntimeAdmissionGuard, ServerRuntimeAdmissionError> {
    let user_id = user_id.trim().to_string();
    let mut state = admission_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let now = Instant::now();
    let window = Duration::from_secs(60);
    if state.in_flight_global >= limits.max_concurrent_global {
        return Err(ServerRuntimeAdmissionError::TooManyGlobalConcurrent {
            max_concurrent_global: limits.max_concurrent_global,
        });
    }

    let entry = state.users.entry(user_id.clone()).or_default();
    entry
        .recent
        .retain(|recorded| now.duration_since(*recorded) < window);

    if entry.in_flight >= limits.max_concurrent_per_user {
        return Err(ServerRuntimeAdmissionError::TooManyConcurrent {
            max_concurrent_per_user: limits.max_concurrent_per_user,
        });
    }
    if entry.recent.len() >= limits.max_requests_per_minute {
        let retry_after_secs = entry
            .recent
            .front()
            .map(|oldest| {
                window
                    .saturating_sub(now.duration_since(*oldest))
                    .as_secs()
                    .max(1)
            })
            .unwrap_or(60);
        return Err(ServerRuntimeAdmissionError::RateLimited {
            max_requests_per_minute: limits.max_requests_per_minute,
            retry_after_secs,
        });
    }

    entry.in_flight += 1;
    entry.recent.push_back(now);
    state.in_flight_global += 1;
    Ok(ServerRuntimeAdmissionGuard {
        user_id,
        released: false,
    })
}

impl ServerRuntimeAdmissionError {
    pub(crate) fn retry_after_secs(&self) -> u64 {
        match self {
            Self::RateLimited {
                retry_after_secs, ..
            } => *retry_after_secs,
            Self::TooManyGlobalConcurrent { .. } | Self::TooManyConcurrent { .. } => 1,
        }
    }

    pub(crate) fn public_message(&self) -> String {
        match self {
            Self::TooManyGlobalConcurrent {
                max_concurrent_global,
            } => format!(
                "Route C 远程模型当前全局任务过多：平台最多同时运行 {max_concurrent_global} 个请求，请稍后重试"
            ),
            Self::TooManyConcurrent {
                max_concurrent_per_user,
            } => format!(
                "Route C 远程模型任务并发过高：每个用户最多同时运行 {max_concurrent_per_user} 个请求"
            ),
            Self::RateLimited {
                max_requests_per_minute,
                retry_after_secs,
            } => format!(
                "Route C 远程模型请求过快：每分钟最多 {max_requests_per_minute} 次，请 {retry_after_secs} 秒后重试"
            ),
        }
    }
}

impl fmt::Display for ServerRuntimeAdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.public_message())
    }
}

impl Drop for ServerRuntimeAdmissionGuard {
    fn drop(&mut self) {
        if self.released {
            return;
        }
        self.released = true;
        let mut state = admission_state()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.in_flight_global = state.in_flight_global.saturating_sub(1);
        let Some(entry) = state.users.get_mut(&self.user_id) else {
            return;
        };
        entry.in_flight = entry.in_flight.saturating_sub(1);
        let now = Instant::now();
        let window = Duration::from_secs(60);
        entry
            .recent
            .retain(|recorded| now.duration_since(*recorded) < window);
        if entry.in_flight == 0 && entry.recent.is_empty() {
            state.users.remove(&self.user_id);
        }
    }
}

pub(crate) fn admission_snapshot(
    user_id: &str,
    limits: ServerAgentRuntimeLimits,
) -> ServerRuntimeAdmissionSnapshot {
    let user_id = user_id.trim();
    let mut state = admission_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let now = Instant::now();
    let window = Duration::from_secs(60);

    let mut should_remove_user = false;
    let (in_flight_for_user, recent_requests_per_minute, rate_limit_retry_after_secs) =
        if let Some(entry) = state.users.get_mut(user_id) {
            entry
                .recent
                .retain(|recorded| now.duration_since(*recorded) < window);
            should_remove_user = entry.in_flight == 0 && entry.recent.is_empty();
            let retry_after = if entry.recent.len() >= limits.max_requests_per_minute {
                entry.recent.front().map(|oldest| {
                    window
                        .saturating_sub(now.duration_since(*oldest))
                        .as_secs()
                        .max(1)
                })
            } else {
                None
            };
            (entry.in_flight, entry.recent.len(), retry_after)
        } else {
            (0, 0, None)
        };
    if should_remove_user {
        state.users.remove(user_id);
    }

    let in_flight_global = state.in_flight_global;
    ServerRuntimeAdmissionSnapshot {
        in_flight_global,
        max_concurrent_global: limits.max_concurrent_global,
        remaining_concurrent_global: limits
            .max_concurrent_global
            .saturating_sub(in_flight_global),
        in_flight_for_user,
        max_concurrent_per_user: limits.max_concurrent_per_user,
        remaining_concurrent_for_user: limits
            .max_concurrent_per_user
            .saturating_sub(in_flight_for_user),
        recent_requests_per_minute,
        max_requests_per_minute: limits.max_requests_per_minute,
        remaining_requests_per_minute: limits
            .max_requests_per_minute
            .saturating_sub(recent_requests_per_minute),
        rate_limit_retry_after_secs,
    }
}

pub(crate) fn admission_availability(
    snapshot: &ServerRuntimeAdmissionSnapshot,
) -> ServerRuntimeAdmissionAvailability {
    if snapshot.remaining_concurrent_global == 0 {
        return ServerRuntimeAdmissionAvailability {
            ready: false,
            reason: Some("global_concurrency_limited"),
            public_message: Some("Route C 远程模型全局并发已满"),
            retry_after_secs: Some(1),
        };
    }
    if snapshot.remaining_concurrent_for_user == 0 {
        return ServerRuntimeAdmissionAvailability {
            ready: false,
            reason: Some("user_concurrency_limited"),
            public_message: Some("当前用户 Route C 远程模型并发已满"),
            retry_after_secs: Some(1),
        };
    }
    if snapshot.remaining_requests_per_minute == 0 {
        return ServerRuntimeAdmissionAvailability {
            ready: false,
            reason: Some("rate_limited"),
            public_message: Some("当前用户 Route C 远程模型请求频率已达上限"),
            retry_after_secs: snapshot.rate_limit_retry_after_secs,
        };
    }
    ServerRuntimeAdmissionAvailability {
        ready: true,
        reason: None,
        public_message: None,
        retry_after_secs: None,
    }
}

#[derive(Default)]
struct RuntimeAdmissionState {
    users: HashMap<String, UserRuntimeAdmission>,
    in_flight_global: usize,
}

#[derive(Default)]
struct UserRuntimeAdmission {
    in_flight: usize,
    recent: VecDeque<Instant>,
}

fn admission_state() -> &'static Mutex<RuntimeAdmissionState> {
    static STATE: OnceLock<Mutex<RuntimeAdmissionState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(RuntimeAdmissionState::default()))
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
        limit_max_message_chars: limits.max_message_chars,
        limit_max_total_chars: limits.max_total_chars,
        limit_max_output_tokens: limits.max_output_tokens,
        limit_max_actions: limits.max_actions,
        limit_max_action_chars: limits.max_action_chars,
        limit_max_actions_total_chars: limits.max_actions_total_chars,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        admission_availability, admission_snapshot, audit_summary, operational_error_summary,
        protection_status, try_acquire_runtime_admission, ServerRuntimeAdmissionError,
        ServerRuntimeAdmissionSnapshot,
    };
    use crate::server_agent_runtime_limits::ServerAgentRuntimeLimits;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn audit_summary_uses_shape_not_prompt_text() {
        let limits = ServerAgentRuntimeLimits::current();
        let left = vec![json!({"role": "user", "content": "secret prompt A"})];
        let right = vec![json!({"role": "user", "content": "secret prompt B"})];

        let left_summary = audit_summary(&left, limits);
        let right_summary = audit_summary(&right, limits);

        assert_eq!(left_summary.message_count, 1);
        assert_eq!(left_summary.total_chars, "secret prompt A".chars().count());
        assert_eq!(
            left_summary.limit_max_message_chars,
            limits.max_message_chars
        );
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
        assert!(status.output_validation.contains("actions"));
        assert!(status
            .agent_selection
            .contains("ELON_SERVER_AGENT_RUNTIME_ALLOWED_AGENTS"));
        assert!(status.admission_control.contains("global"));
        assert!(status.admission_control.contains("concurrency"));
        assert!(status
            .budget_gate
            .contains("ELON_SERVER_AGENT_RUNTIME_DAILY_CALL_LIMIT"));
        assert!(status
            .operational_switch
            .contains("ELON_SERVER_AGENT_RUNTIME_ENABLED"));
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

    #[test]
    fn admission_gate_limits_per_user_concurrency() {
        let user_id = unique_user("concurrent");
        let limits = ServerAgentRuntimeLimits {
            max_messages: 24,
            max_message_chars: 32_000,
            max_total_chars: 80_000,
            max_output_tokens: 3000,
            max_actions: 24,
            max_action_chars: 64_000,
            max_actions_total_chars: 96_000,
            max_requests_per_minute: 10,
            max_concurrent_per_user: 1,
            max_concurrent_global: 10,
            temperature: 0.2,
        };

        let first = try_acquire_runtime_admission(&user_id, limits).unwrap();
        let second = try_acquire_runtime_admission(&user_id, limits).unwrap_err();
        assert_eq!(
            second,
            ServerRuntimeAdmissionError::TooManyConcurrent {
                max_concurrent_per_user: 1
            }
        );

        drop(first);
        let after_release = try_acquire_runtime_admission(&user_id, limits).unwrap();
        drop(after_release);
    }

    #[test]
    fn admission_gate_limits_per_user_rate() {
        let user_id = unique_user("rate");
        let limits = ServerAgentRuntimeLimits {
            max_messages: 24,
            max_message_chars: 32_000,
            max_total_chars: 80_000,
            max_output_tokens: 3000,
            max_actions: 24,
            max_action_chars: 64_000,
            max_actions_total_chars: 96_000,
            max_requests_per_minute: 1,
            max_concurrent_per_user: 10,
            max_concurrent_global: 10,
            temperature: 0.2,
        };

        let first = try_acquire_runtime_admission(&user_id, limits).unwrap();
        drop(first);
        let second = try_acquire_runtime_admission(&user_id, limits).unwrap_err();
        assert!(matches!(
            second,
            ServerRuntimeAdmissionError::RateLimited {
                max_requests_per_minute: 1,
                retry_after_secs: 1..=60
            }
        ));
        assert!(second.public_message().contains("每分钟最多 1 次"));
    }

    #[test]
    fn admission_gate_limits_global_concurrency_across_users() {
        let open_limits = ServerAgentRuntimeLimits {
            max_messages: 24,
            max_message_chars: 32_000,
            max_total_chars: 80_000,
            max_output_tokens: 3000,
            max_actions: 24,
            max_action_chars: 64_000,
            max_actions_total_chars: 96_000,
            max_requests_per_minute: 10,
            max_concurrent_per_user: 10,
            max_concurrent_global: usize::MAX,
            temperature: 0.2,
        };
        let capped_limits = ServerAgentRuntimeLimits {
            max_messages: 24,
            max_message_chars: 32_000,
            max_total_chars: 80_000,
            max_output_tokens: 3000,
            max_actions: 24,
            max_action_chars: 64_000,
            max_actions_total_chars: 96_000,
            max_requests_per_minute: 10,
            max_concurrent_per_user: 10,
            max_concurrent_global: 1,
            temperature: 0.2,
        };

        let first_user = unique_user("global-a");
        let second_user = unique_user("global-b");
        let first = try_acquire_runtime_admission(&first_user, open_limits).unwrap();
        let second = try_acquire_runtime_admission(&second_user, capped_limits).unwrap_err();
        assert_eq!(
            second,
            ServerRuntimeAdmissionError::TooManyGlobalConcurrent {
                max_concurrent_global: 1
            }
        );
        assert!(second.public_message().contains("全局任务过多"));

        drop(first);
        let after_release = try_acquire_runtime_admission(&second_user, open_limits).unwrap();
        drop(after_release);
    }

    #[test]
    fn admission_snapshot_reports_current_user_capacity() {
        let user_id = unique_user("snapshot");
        let limits = ServerAgentRuntimeLimits {
            max_messages: 24,
            max_message_chars: 32_000,
            max_total_chars: 80_000,
            max_output_tokens: 3000,
            max_actions: 24,
            max_action_chars: 64_000,
            max_actions_total_chars: 96_000,
            max_requests_per_minute: 1,
            max_concurrent_per_user: 1,
            max_concurrent_global: 10,
            temperature: 0.2,
        };

        let guard = try_acquire_runtime_admission(&user_id, limits).unwrap();
        let snapshot = admission_snapshot(&user_id, limits);
        assert_eq!(snapshot.in_flight_for_user, 1);
        assert_eq!(snapshot.remaining_concurrent_for_user, 0);
        assert_eq!(snapshot.recent_requests_per_minute, 1);
        assert_eq!(snapshot.remaining_requests_per_minute, 0);
        assert!(matches!(snapshot.rate_limit_retry_after_secs, Some(1..=60)));
        assert!(!serde_json::to_string(&snapshot).unwrap().contains(&user_id));

        drop(guard);
        let released = admission_snapshot(&user_id, limits);
        assert_eq!(released.in_flight_for_user, 0);
    }

    #[test]
    fn admission_availability_reports_capacity_reason() {
        let mut snapshot = ServerRuntimeAdmissionSnapshot {
            in_flight_global: 0,
            max_concurrent_global: 24,
            remaining_concurrent_global: 24,
            in_flight_for_user: 0,
            max_concurrent_per_user: 2,
            remaining_concurrent_for_user: 2,
            recent_requests_per_minute: 0,
            max_requests_per_minute: 12,
            remaining_requests_per_minute: 12,
            rate_limit_retry_after_secs: None,
        };

        assert!(admission_availability(&snapshot).ready);

        snapshot.remaining_concurrent_global = 0;
        let global = admission_availability(&snapshot);
        assert!(!global.ready);
        assert_eq!(global.reason, Some("global_concurrency_limited"));

        snapshot.remaining_concurrent_global = 1;
        snapshot.remaining_concurrent_for_user = 0;
        let user = admission_availability(&snapshot);
        assert!(!user.ready);
        assert_eq!(user.reason, Some("user_concurrency_limited"));

        snapshot.remaining_concurrent_for_user = 1;
        snapshot.remaining_requests_per_minute = 0;
        snapshot.rate_limit_retry_after_secs = Some(17);
        let rate = admission_availability(&snapshot);
        assert!(!rate.ready);
        assert_eq!(rate.reason, Some("rate_limited"));
        assert_eq!(rate.retry_after_secs, Some(17));
    }

    #[test]
    fn admission_error_exposes_retry_after_for_clients() {
        let rate_limited = ServerRuntimeAdmissionError::RateLimited {
            max_requests_per_minute: 1,
            retry_after_secs: 17,
        };
        assert_eq!(rate_limited.retry_after_secs(), 17);

        let concurrent = ServerRuntimeAdmissionError::TooManyConcurrent {
            max_concurrent_per_user: 1,
        };
        assert_eq!(concurrent.retry_after_secs(), 1);
    }

    fn unique_user(label: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        format!("user-{label}-{nanos}")
    }
}
