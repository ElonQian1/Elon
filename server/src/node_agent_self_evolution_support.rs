//! Small policy and response helpers for the self-evolution coordinator.

use axum::{http::StatusCode, response::IntoResponse, response::Response, Json};
use homecli_proto::InterruptionSource;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{AdmissionError, PendingSelfEvolutionAction};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct SelfEvolutionGates {
    #[serde(default)]
    pub foreground_task_ids: Vec<String>,
    #[serde(default)]
    pub publish_active: bool,
    #[serde(default)]
    pub publish_status: String,
    #[serde(default)]
    pub publish_owner: Option<String>,
    #[serde(default)]
    pub publish_waiter_count: usize,
    #[serde(default)]
    pub update_active: bool,
    #[serde(default)]
    pub resource_pressure: bool,
    #[serde(default)]
    pub checked_at_ms: u128,
}

impl SelfEvolutionGates {
    pub(super) fn blocker(&self) -> Option<&'static str> {
        if !self.foreground_task_ids.is_empty() {
            Some("foreground_task")
        } else if self.publish_active {
            Some("global_publish")
        } else if self.update_active {
            Some("node_update")
        } else if self.resource_pressure {
            Some("resource_pressure")
        } else {
            None
        }
    }
}

pub(super) fn interruption_from_intent(
    intent: Option<&PendingSelfEvolutionAction>,
) -> Option<InterruptionSource> {
    match intent.map(|value| (value.action.as_str(), value.source.as_str())) {
        Some(("pause", "updater_apply" | "node_update")) => Some(InterruptionSource::UpdaterApply),
        Some(("pause", "local_pc_ui" | "supervisor")) => {
            Some(InterruptionSource::SupervisorIntervention)
        }
        _ => None,
    }
}

pub(super) fn admission(status: StatusCode, message: impl Into<String>) -> AdmissionError {
    AdmissionError {
        status,
        message: message.into(),
    }
}

pub(super) fn internal_admission(error: impl ToString) -> AdmissionError {
    admission(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

pub(super) fn error_response(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(json!({"ok": false, "error": message.into()}))).into_response()
}

pub(super) fn schema_version() -> u32 {
    2
}

pub(super) fn default_max_retries() -> u32 {
    3
}

pub(super) fn retry_at(retry_count: u32) -> u128 {
    now_ms() + 15_000u128.saturating_mul(1u128 << retry_count.min(6))
}

pub(super) fn retryable_failure(reason: &str) -> bool {
    let reason = reason.to_ascii_lowercase();
    [
        "quota",
        "rate limit",
        "429",
        "too many requests",
        "credit",
        "额度",
        "配额",
        "限流",
    ]
    .iter()
    .any(|needle| reason.contains(needle))
}

pub(super) fn same_gate_observation(left: &SelfEvolutionGates, right: &SelfEvolutionGates) -> bool {
    left.foreground_task_ids == right.foreground_task_ids
        && left.publish_active == right.publish_active
        && left.publish_status == right.publish_status
        && left.publish_owner == right.publish_owner
        && left.publish_waiter_count == right.publish_waiter_count
        && left.update_active == right.update_active
        && left.resource_pressure == right.resource_pressure
}

pub(super) fn now_ms() -> u128 {
    crate::node_agent_cli_sidecar::now_ms()
}
