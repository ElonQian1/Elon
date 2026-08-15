use serde::{Deserialize, Serialize};

/// Server-runner handoff. Intentionally non-Clone/non-Debug/non-Serde: callers cannot upload or
/// replay this value as evidence, and the canonical builder consumes it by value.
pub(crate) struct TaskProtocolConformanceRunEvidence {
    pub(crate) run_nonce_digest: String,
    pub(crate) source_capsule_sha256: String,
    pub(crate) source_capsule_size_bytes: u64,
    pub(crate) launch_image_sha256: String,
    pub(crate) launch_image_size_bytes: u64,
    pub(crate) public_fixture_delivery_root: String,
    pub(crate) session_roots_digest: String,
    pub(crate) session_transcript_digest: String,
    pub(crate) delivery_inventory_digest: String,
    pub(crate) exchange_inventory_digest: String,
    pub(crate) task_observation_root: String,
    pub(crate) run_started_at: String,
    pub(crate) run_completed_at: String,
    pub(crate) duration_ms: u64,
    pub(crate) exchanges: Vec<TaskProtocolConformanceExchangeObservation>,
    pub(crate) capabilities: Vec<TaskProtocolConformanceCapabilityObservation>,
    pub(crate) cleanup: TaskProtocolConformanceCleanupEvidence,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskProtocolConformanceExchangeObservation {
    pub exchange_ordinal: u64,
    pub scenario_id: String,
    pub operation_kind: String,
    pub capability_id: String,
    pub capability_revision: u64,
    pub replay_kind: String,
    pub command_digest: String,
    pub outbox_operation_digest: String,
    pub route_authorization_digest: String,
    pub synthetic_executor_digest: String,
    pub fence_digest: String,
    pub request_digest: String,
    pub delivery_attempt_digest: String,
    pub exchange_nonce_digest: String,
    pub upstream_request_bytes: u64,
    pub upstream_request_sha256: String,
    pub upstream_response_bytes: u64,
    pub upstream_response_sha256: String,
    pub semantic_observation_bytes: u64,
    pub semantic_observation_sha256: String,
    pub exchange_root: String,
    pub adapter_observation_id: String,
    pub response_outcome: String,
    pub remote_state_before: String,
    pub remote_state_after: String,
    pub terminality: String,
    pub remote_reference_digest: Option<String>,
    pub remote_sequence: Option<u64>,
    pub no_commit_tombstone_digest: Option<String>,
    pub event_cursor_before_digest: Option<String>,
    pub event_cursor_after_digest: Option<String>,
    pub event_count: u64,
    pub event_inventory_digest: Option<String>,
    pub commit_uncertainty_state_before: String,
    pub commit_uncertainty_state_after: String,
    pub commit_uncertainty_marker_digest: Option<String>,
    pub event_replay_classification: Option<String>,
    pub event_replay_batch_count: u64,
    pub event_replay_root: Option<String>,
    pub oracle_start_count_before: u64,
    pub oracle_start_count_after: u64,
    pub oracle_event_count_before: u64,
    pub oracle_event_count_after: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskProtocolConformanceCapabilityObservation {
    pub capability_id: String,
    pub capability_revision: u64,
    pub status: String,
    pub test_case_id: String,
    pub fixture_digest: String,
    pub exchange_ordinals: Vec<u64>,
    pub exchange_inventory_digest: String,
    pub assertion_inventory_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskProtocolConformanceCleanupEvidence {
    pub authenticated_shutdown_completed: bool,
    pub pidfd_reaped: bool,
    pub cgroup_cleaned: bool,
    pub scratch_cleaned: bool,
}
