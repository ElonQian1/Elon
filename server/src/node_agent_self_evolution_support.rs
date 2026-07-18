//! Small policy and response helpers for the self-evolution coordinator.

use axum::{http::StatusCode, response::IntoResponse, response::Response, Json};
use serde_json::json;

use super::{AdmissionError, SelfEvolutionGates};

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
