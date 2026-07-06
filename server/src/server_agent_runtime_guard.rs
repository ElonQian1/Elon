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
    pub duplicate_request_debounce: &'static str,
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
    pub duplicate_request_window_secs: usize,
    pub recent_duplicate_fingerprints: usize,
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
    DuplicateRecent {
        retry_after_secs: u64,
    },
}

#[derive(Debug)]
pub(crate) struct ServerRuntimeAdmissionGuard {
    user_id: String,
    duplicate_window_secs: usize,
    released: bool,
}

pub(crate) fn protection_status() -> ServerRuntimeProtectionStatus {
    ServerRuntimeProtectionStatus {
        input_validation: "messages role/content/count/message_chars/total_chars",
        output_validation:
            "model JSON must be an object; actions must stay within count/action_chars/total_action_chars",
        agent_selection:
            "server_api_key usage_mode only; default server agent only unless ELON_SERVER_AGENT_RUNTIME_ALLOWED_AGENTS explicitly allows more",
        admission_control: "global and per-user concurrency plus rolling minute request limits",
        duplicate_request_debounce:
            "same user/request fingerprint is rejected inside ELON_SERVER_AGENT_RUNTIME_DUPLICATE_WINDOW_SECS",
        budget_gate:
            "optional ELON_SERVER_AGENT_RUNTIME_DAILY_CALL_LIMIT platform fuse plus ELON_SERVER_AGENT_RUNTIME_PER_USER_DAILY_CALL_LIMIT user fuse",
        operational_switch: "ELON_SERVER_AGENT_RUNTIME_ENABLED can disable Route C without redeploy",
        billing_gate: "shared with call_chat_llm_with_options",
        audit:
            "logs user_id, agent, model, message counts, char totals, and request fingerprint only",
        request_fingerprint:
            "sha256 over roles, content lengths, and content hashes; prompt/error text is not logged",
    }
}

pub(crate) fn try_acquire_runtime_admission_for_request(
    user_id: &str,
    limits: ServerAgentRuntimeLimits,
    request_fingerprint: Option<&str>,
) -> Result<ServerRuntimeAdmissionGuard, ServerRuntimeAdmissionError> {
    let user_id = user_id.trim().to_string();
    let request_fingerprint = request_fingerprint
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let mut state = admission_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let now = Instant::now();
    let window = Duration::from_secs(60);
    let duplicate_window = Duration::from_secs(limits.duplicate_request_window_secs as u64);
    if state.in_flight_global >= limits.max_concurrent_global {
        return Err(ServerRuntimeAdmissionError::TooManyGlobalConcurrent {
            max_concurrent_global: limits.max_concurrent_global,
        });
    }

    let entry = state.users.entry(user_id.clone()).or_default();
    entry
        .recent
        .retain(|recorded| now.duration_since(*recorded) < window);
    if limits.duplicate_request_window_secs > 0 {
        entry
            .recent_fingerprints
            .retain(|_, recorded| now.duration_since(*recorded) < duplicate_window);
    } else {
        entry.recent_fingerprints.clear();
    }

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
    if let Some(fingerprint) = request_fingerprint.as_ref() {
        if let Some(recorded) = entry.recent_fingerprints.get(fingerprint) {
            let retry_after_secs = duplicate_window
                .saturating_sub(now.duration_since(*recorded))
                .as_secs()
                .max(1);
            return Err(ServerRuntimeAdmissionError::DuplicateRecent { retry_after_secs });
        }
    }

    entry.in_flight += 1;
    entry.recent.push_back(now);
    if limits.duplicate_request_window_secs > 0 {
        if let Some(fingerprint) = request_fingerprint {
            entry.recent_fingerprints.insert(fingerprint, now);
        }
    }
    state.in_flight_global += 1;
    Ok(ServerRuntimeAdmissionGuard {
        user_id,
        duplicate_window_secs: limits.duplicate_request_window_secs,
        released: false,
    })
}

impl ServerRuntimeAdmissionError {
    pub(crate) fn retry_after_secs(&self) -> u64 {
        match self {
            Self::RateLimited {
                retry_after_secs, ..
            }
            | Self::DuplicateRecent { retry_after_secs } => *retry_after_secs,
            Self::TooManyGlobalConcurrent { .. } | Self::TooManyConcurrent { .. } => 1,
        }
    }

    pub(crate) fn public_message(&self) -> String {
        match self {
            Self::TooManyGlobalConcurrent {
                max_concurrent_global,
            } => format!(
                "平台AI当前全局任务过多：平台最多同时运行 {max_concurrent_global} 个请求，请稍后重试"
            ),
            Self::TooManyConcurrent {
                max_concurrent_per_user,
            } => format!(
                "平台AI任务并发过高：每个用户最多同时运行 {max_concurrent_per_user} 个请求"
            ),
            Self::RateLimited {
                max_requests_per_minute,
                retry_after_secs,
            } => format!(
                "平台AI请求过快：每分钟最多 {max_requests_per_minute} 次，请 {retry_after_secs} 秒后重试"
            ),
            Self::DuplicateRecent { retry_after_secs } => format!(
                "平台AI检测到相同请求刚刚提交，请 {retry_after_secs} 秒后重试或等待当前任务结果"
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
        let duplicate_window = Duration::from_secs(self.duplicate_window_secs as u64);
        entry
            .recent
            .retain(|recorded| now.duration_since(*recorded) < window);
        if self.duplicate_window_secs > 0 {
            entry
                .recent_fingerprints
                .retain(|_, recorded| now.duration_since(*recorded) < duplicate_window);
        } else {
            entry.recent_fingerprints.clear();
        }
        if entry.in_flight == 0 && entry.recent.is_empty() && entry.recent_fingerprints.is_empty() {
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
    let duplicate_window = Duration::from_secs(limits.duplicate_request_window_secs as u64);
    let (
        in_flight_for_user,
        recent_requests_per_minute,
        rate_limit_retry_after_secs,
        recent_duplicate_fingerprints,
    ) = if let Some(entry) = state.users.get_mut(user_id) {
        entry
            .recent
            .retain(|recorded| now.duration_since(*recorded) < window);
        if limits.duplicate_request_window_secs > 0 {
            entry
                .recent_fingerprints
                .retain(|_, recorded| now.duration_since(*recorded) < duplicate_window);
        } else {
            entry.recent_fingerprints.clear();
        }
        should_remove_user =
            entry.in_flight == 0 && entry.recent.is_empty() && entry.recent_fingerprints.is_empty();
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
        (
            entry.in_flight,
            entry.recent.len(),
            retry_after,
            entry.recent_fingerprints.len(),
        )
    } else {
        (0, 0, None, 0)
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
        duplicate_request_window_secs: limits.duplicate_request_window_secs,
        recent_duplicate_fingerprints,
    }
}

pub(crate) fn admission_availability(
    snapshot: &ServerRuntimeAdmissionSnapshot,
) -> ServerRuntimeAdmissionAvailability {
    if snapshot.remaining_concurrent_global == 0 {
        return ServerRuntimeAdmissionAvailability {
            ready: false,
            reason: Some("global_concurrency_limited"),
            public_message: Some("平台AI全局并发已满"),
            retry_after_secs: Some(1),
        };
    }
    if snapshot.remaining_concurrent_for_user == 0 {
        return ServerRuntimeAdmissionAvailability {
            ready: false,
            reason: Some("user_concurrency_limited"),
            public_message: Some("当前用户平台AI并发已满"),
            retry_after_secs: Some(1),
        };
    }
    if snapshot.remaining_requests_per_minute == 0 {
        return ServerRuntimeAdmissionAvailability {
            ready: false,
            reason: Some("rate_limited"),
            public_message: Some("当前用户平台AI请求频率已达上限"),
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
    recent_fingerprints: HashMap<String, Instant>,
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
#[path = "server_agent_runtime_guard_tests.rs"]
mod tests;
