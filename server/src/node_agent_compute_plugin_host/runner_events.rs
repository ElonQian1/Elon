use serde::{Deserialize, Serialize};

use super::attempt_contract::{ComputeAttemptArtifactRef, ComputeAttemptCheckpointRef};

pub(crate) const COMPUTE_RUNNER_EVENT_SCHEMA: &str = "elon.compute_plugin.runner_event.v1";
pub(crate) const COMPUTE_ATTEMPT_EVENT_SCHEMA: &str = "compute_federation.attempt_event.v1";
pub(crate) const COMPUTE_STREAM_PAYLOAD_ENCODING_UTF8: &str = "utf8";
pub(crate) const COMPUTE_STREAM_PAYLOAD_ENCODING_BASE64: &str = "base64";
pub(crate) const COMPUTE_JSON_SAFE_SEQUENCE_MAX: i64 = 9_007_199_254_740_991;

/// Runner-originated IPC event. Attempt identity is deliberately absent and added by the Host.
/// The future IPC framing layer must reject oversized frames before deserializing this payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRunnerEventEnvelope {
    pub schema: String,
    pub runner_execution_id: String,
    pub event_id: String,
    /// Non-negative, monotonic and capped at `COMPUTE_JSON_SAFE_SEQUENCE_MAX`.
    pub sequence_no: i64,
    pub emitted_at: String,
    pub event: ComputeRunnerEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "event_type", content = "payload", rename_all = "snake_case")]
pub(crate) enum ComputeRunnerEvent {
    Started,
    Heartbeat,
    Progress(ComputeRunnerProgress),
    StreamChunk(ComputeRunnerStreamChunk),
    CheckpointReady(ComputeRunnerArtifactClaim),
    UsageSnapshot(ComputeRunnerUsageSnapshot),
    Terminal(ComputeRunnerTerminal),
}

impl ComputeRunnerEvent {
    pub(crate) fn is_terminal(&self) -> bool {
        matches!(self, Self::Terminal(_))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRunnerProgress {
    pub phase: String,
    pub completed_units: Option<i64>,
    pub total_units: Option<i64>,
    pub message_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRunnerStreamChunk {
    pub stream_id: String,
    pub chunk_index: i64,
    pub media_type: String,
    pub payload_encoding: String,
    pub payload: String,
    pub payload_size_bytes: i64,
    pub claimed_digest: Option<String>,
}

/// A sandbox handle is not a CAS location. The Host must import and hash the artifact itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRunnerArtifactClaim {
    pub sandbox_handle: String,
    pub digest_algorithm: String,
    pub claimed_digest: String,
    pub media_type: String,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRunnerUsageSnapshot {
    /// Quantities are non-negative, cumulative and monotonic for each meter.
    pub cumulative_declared_usage: Vec<ComputeRunnerMeterReading>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRunnerMeterReading {
    pub meter: String,
    pub cumulative_quantity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRunnerTerminal {
    pub outcome: ComputeRunnerOutcome,
    pub reason_code: String,
    /// IPC framing and the Host must bound and sanitize this before persistent logging.
    pub diagnostic: Option<String>,
    pub claimed_output_digest: Option<String>,
    pub result_artifacts: Vec<ComputeRunnerArtifactClaim>,
    pub final_cumulative_declared_usage: Vec<ComputeRunnerMeterReading>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ComputeRunnerOutcome {
    Succeeded,
    Failed,
    Canceled,
}

/// Host-stamped event. The Host derives identity from the active command, never from the Runner.
/// `event_id` and `sequence_no` are newly allocated by the Host; the source id is audit-only.
/// Only the first terminal event for one lease generation may become a candidate result; later
/// events remain audit evidence and can never overwrite it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptEventEnvelope {
    pub schema: String,
    pub event_id: String,
    pub source_runner_event_id: Option<String>,
    pub sequence_no: i64,
    pub attempt_lease_id: String,
    pub attempt_no: i64,
    pub fencing_generation: i64,
    pub host_observed_at: String,
    pub event: ComputeAttemptEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "event_type", content = "payload", rename_all = "snake_case")]
pub(crate) enum ComputeAttemptEvent {
    Started,
    Heartbeat,
    Progress(ComputeRunnerProgress),
    StreamChunk(ComputeAttemptStreamChunk),
    CheckpointAvailable(ComputeAttemptCheckpointRef),
    UsageDeclared(ComputeRunnerUsageSnapshot),
    Terminal(ComputeAttemptTerminal),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptStreamChunk {
    pub stream_id: String,
    pub chunk_index: i64,
    pub media_type: String,
    pub payload_encoding: String,
    pub payload: String,
    pub payload_size_bytes: i64,
    pub digest: String,
}

/// This is still an execution event, never an ExecutionReceipt or settlement decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptTerminal {
    pub outcome: ComputeRunnerOutcome,
    pub reason_code: String,
    pub diagnostic: Option<String>,
    pub output_digest: Option<String>,
    pub result_artifacts: Vec<ComputeAttemptArtifactRef>,
    pub final_cumulative_declared_usage: Vec<ComputeRunnerMeterReading>,
}

/// Object-safe synchronous boundary. Implementations own buffering and backpressure policy.
pub(crate) trait ComputeRunnerEventSink: Send + Sync {
    fn emit(&self, event: ComputeRunnerEventEnvelope) -> Result<(), ComputeRunnerEventSinkError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ComputeRunnerEventSinkError {
    pub code: String,
    pub message: String,
}
