use serde::{Deserialize, Serialize};

pub(crate) const COMPUTE_WORKLOAD_SCHEMA: &str = "compute_federation.workload.v1";

pub(crate) const TASK_KIND_LLM_CHAT: &str = "llm_chat";
pub(crate) const TASK_KIND_EMBEDDING: &str = "embedding";
pub(crate) const TASK_KIND_RERANK: &str = "rerank";
pub(crate) const TASK_KIND_IMAGE_GENERATION: &str = "image_generation";
pub(crate) const TASK_KIND_VIDEO_GENERATION: &str = "video_generation";
pub(crate) const TASK_KIND_EVALUATION_SHARD: &str = "evaluation_shard";
pub(crate) const TASK_KIND_GPU_BATCH: &str = "gpu_batch";

pub(crate) const DATA_CLASS_PUBLIC: &str = "public";
pub(crate) const DATA_CLASS_LOW_SENSITIVITY: &str = "low_sensitivity";
pub(crate) const DATA_CLASS_RESTRICTED: &str = "restricted";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeArtifactRef {
    pub artifact_id: String,
    pub digest_algorithm: String,
    pub digest: String,
    pub media_type: String,
    pub size_bytes: i64,
    pub location_ref: String,
    pub encryption_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeModelRef {
    pub model_id: String,
    pub model_family: String,
    pub model_digest: String,
    pub tokenizer_digest: Option<String>,
    pub adapter_digests: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeRuntimeRef {
    pub runtime_family: String,
    pub runtime_version: String,
    pub precision: String,
    pub runner_digest: String,
    pub plugin_id: Option<String>,
    pub plugin_version: Option<String>,
    pub plugin_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeResourceRequirements {
    pub accelerator_kinds: Vec<String>,
    pub min_accelerator_count: i64,
    pub min_vram_bytes: i64,
    pub min_ram_bytes: i64,
    pub min_disk_bytes: i64,
    pub max_runtime_seconds: i64,
    pub allow_network_egress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeOutputContract {
    pub media_type: String,
    pub max_output_bytes: i64,
    pub streaming: bool,
    pub result_artifact_required: bool,
    pub deterministic_digest_expected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeRetryPolicy {
    pub max_attempts: i64,
    pub initial_backoff_ms: i64,
    pub max_backoff_ms: i64,
    pub retryable_error_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeCheckpointPolicy {
    pub mode: String,
    pub interval_seconds: Option<i64>,
    pub max_checkpoints: i64,
    pub checkpoint_media_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeVerificationPolicy {
    pub verification_tier: String,
    pub minimum_independent_receipts: i64,
    pub duplicate_sample_rate_basis_points: i64,
    pub challenge_profile_id: Option<String>,
    pub require_server_metering: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeUsageLimit {
    pub meter: String,
    pub max_quantity: i64,
}

/// Optional shard identity for independently retryable work.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeShardSpec {
    pub shard_id: String,
    pub shard_index: i64,
    pub shard_count: i64,
    pub merge_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeWorkloadSpec {
    pub schema: String,
    pub task_kind: String,
    pub input_artifacts: Vec<ComputeArtifactRef>,
    pub model: Option<ComputeModelRef>,
    pub runtime: Option<ComputeRuntimeRef>,
    pub resources: ComputeResourceRequirements,
    pub output: ComputeOutputContract,
    pub usage_limits: Vec<ComputeUsageLimit>,
    pub data_class: String,
    pub shard: Option<ComputeShardSpec>,
    pub retry_policy: ComputeRetryPolicy,
    pub checkpoint_policy: ComputeCheckpointPolicy,
    pub verification_policy: ComputeVerificationPolicy,
    pub deadline_at: String,
}
