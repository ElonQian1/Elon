use serde::{Deserialize, Serialize};

use crate::node_agent_compute_plugin_host::identity::ComputePluginReleaseRef;

pub(crate) const COMPUTE_ATTEMPT_COMMAND_SCHEMA: &str = "compute_federation.attempt_command.v1";
pub(crate) const COMPUTE_ATTEMPT_COMMAND_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const COMPUTE_ATTEMPT_COMMAND_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const COMPUTE_ATTEMPT_COMMAND_DIGEST_DOMAIN: &str = "ELON-COMPUTE-ATTEMPT-COMMAND-V1";

/// Local Host contract only. It is not advertised as a cloud wire capability yet. Digest bytes
/// are `sha256(DOMAIN || 0x00 || RFC8785_JCS(envelope_without_command_digest))`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptCommandEnvelope {
    pub schema: String,
    pub command_id: String,
    pub command_digest: String,
    pub issued_at: String,
    pub command: ComputeAttemptCommand,
}

/// Non-serializable companion held by the Host. It is never forwarded to a Runner sidecar.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ComputeAttemptCommandAuthContext {
    pub lease_credential_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "command_type", content = "payload", rename_all = "snake_case")]
pub(crate) enum ComputeAttemptCommand {
    Start(ComputeAttemptStart),
    RenewLease(ComputeAttemptRenewLease),
    Cancel(ComputeAttemptCancel),
}

/// A node executes an immutable Attempt, never a mutable Job or Reservation.
/// Effective stop time is the earliest workload deadline, hard deadline or runtime limit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptStart {
    pub identity: ComputeAttemptIdentity,
    pub provider_id: String,
    pub executor_id: String,
    pub offer: ComputeAttemptOfferBinding,
    pub selected_runtime: ComputeAttemptRuntimeBinding,
    pub selected_model: Option<ComputeAttemptModelBinding>,
    pub workload: ComputeAttemptWorkload,
    pub latest_checkpoint: Option<ComputeAttemptCheckpointRef>,
    pub lease_expires_at: String,
    pub hard_deadline_at: String,
}

/// A renewal may only extend the live soft deadline within the immutable hard deadline.
/// Expired attempts cannot be revived; retries require a new lease and fencing generation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptRenewLease {
    pub lease: ComputeAttemptLeaseRef,
    pub lease_expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptCancel {
    pub lease: ComputeAttemptLeaseRef,
    pub reason_code: String,
    pub grace_deadline_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptIdentity {
    pub job_id: String,
    pub reservation_id: String,
    pub attempt_lease_id: String,
    pub attempt_no: i64,
    pub shard_id: Option<String>,
    pub fencing_generation: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptLeaseRef {
    pub attempt_lease_id: String,
    pub attempt_no: i64,
    pub fencing_generation: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptOfferBinding {
    pub offer_id: String,
    pub offer_version: i64,
    pub offer_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptRuntimeBinding {
    pub runner_id: String,
    pub runner_digest: String,
    /// `manifest_digest` inside this release is the canonical plugin digest used by receipts.
    pub plugin_release: Option<ComputePluginReleaseRef>,
    pub runtime_family: String,
    pub runtime_version: String,
    pub runtime_digest: String,
    pub precision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptModelBinding {
    pub model_id: String,
    pub model_family: String,
    pub model_digest: String,
    pub tokenizer_digest: Option<String>,
    pub adapter_digests: Vec<String>,
}

/// Execution-only projection. Price, budget, retry and verification policy stay server-side.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptWorkload {
    pub workload_schema: String,
    pub workload_spec_digest: String,
    pub canonical_input_digest: String,
    pub task_kind: String,
    pub data_class: String,
    pub shard: Option<ComputeAttemptShardSpec>,
    pub input_artifacts: Vec<ComputeAttemptArtifactRef>,
    pub output: ComputeAttemptOutputContract,
    pub resources: ComputeAttemptResourceLimits,
    pub usage_limits: Vec<ComputeAttemptUsageLimit>,
    pub checkpoint_policy: ComputeAttemptCheckpointPolicy,
    pub workload_deadline_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptArtifactRef {
    pub artifact_id: String,
    pub digest_algorithm: String,
    pub digest: String,
    pub media_type: String,
    pub size_bytes: i64,
    pub access_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptCheckpointRef {
    pub checkpoint_id: String,
    pub artifact: ComputeAttemptArtifactRef,
    pub source_attempt_no: i64,
    pub source_fencing_generation: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptShardSpec {
    pub shard_id: String,
    pub shard_index: i64,
    pub shard_count: i64,
    pub merge_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptOutputContract {
    pub media_type: String,
    pub max_output_bytes: i64,
    pub streaming: bool,
    pub result_artifact_required: bool,
    pub deterministic_digest_expected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptResourceLimits {
    pub accelerator_count: i64,
    pub max_cpu_millicores: i64,
    pub max_memory_bytes: i64,
    pub max_vram_bytes: i64,
    pub max_disk_bytes: i64,
    pub max_runtime_seconds: i64,
    pub allow_network_egress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptUsageLimit {
    pub meter: String,
    pub max_quantity: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptCheckpointPolicy {
    pub mode: String,
    pub interval_seconds: Option<i64>,
    pub maximum_checkpoints: i64,
    pub checkpoint_media_type: Option<String>,
}
