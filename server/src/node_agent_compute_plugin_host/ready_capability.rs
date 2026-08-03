use serde::{Deserialize, Serialize};

use super::{
    identity::ComputePluginReleaseRef,
    lifecycle::{
        local_record_shape_is_valid, ComputePluginLocalRecord, ACTIVATION_ENABLED,
        ADMISSION_ALLOWED, RUNTIME_READY,
    },
};

pub(crate) const COMPUTE_READY_CAPABILITY_SCHEMA: &str = "elon.compute_plugin.ready_capability.v1";
pub(crate) const HASHED_COMPUTE_READY_CAPABILITY_SCHEMA: &str =
    "elon.compute_plugin.hashed_ready_capability.v1";
pub(crate) const COMPUTE_READY_HEALTHY: &str = "healthy";

/// Short-lived technical evidence. Price, reservable capacity and account policy belong to Offer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeReadyCapability {
    pub schema: String,
    pub capability_id: String,
    pub executor_id: String,
    pub inventory_revision: i64,
    pub install_generation: i64,
    pub activation_generation: i64,
    pub runtime_generation: i64,
    pub slot_ref: String,
    pub release: ComputePluginReleaseRef,
    pub runner_id: String,
    pub runner_digest: String,
    pub runtime_digest: String,
    pub health_observation_digest: String,
    pub task_kinds: Vec<String>,
    pub model_bindings: Vec<ComputeReadyModelBinding>,
    pub supported_precisions: Vec<String>,
    pub resource_profile_digest: String,
    /// Local technical ceiling only; the versioned Offer owns market concurrency.
    pub technical_concurrency_limit: i64,
    pub observed_at: String,
    pub expires_at: String,
}

/// Digest is outside the payload and covers canonical capability bytes only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct HashedComputeReadyCapability {
    pub schema: String,
    pub capability: ComputeReadyCapability,
    pub canonicalization: String,
    pub capability_digest_algorithm: String,
    pub capability_digest: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeReadyModelBinding {
    pub model_id: String,
    pub model_digest: String,
    pub tokenizer_digest: Option<String>,
}

/// Caller supplies freshness after comparing the health expiry against a trusted clock.
pub(crate) fn local_record_can_publish_ready_capability(
    record: &ComputePluginLocalRecord,
    sharing_enabled: bool,
    health_is_fresh: bool,
) -> bool {
    let Some(health) = record.health.as_ref() else {
        return false;
    };
    sharing_enabled
        && health_is_fresh
        && local_record_shape_is_valid(record)
        && record.desired_activation == ACTIVATION_ENABLED
        && record.admission == ADMISSION_ALLOWED
        && record.runtime.phase == RUNTIME_READY
        && record.permission_grant_digest.is_some()
        && record.runtime.slot_ref.as_deref() == Some(health.slot_ref.as_str())
        && record.runtime.runtime_generation == health.runtime_generation
        && record.runtime.runner_digest.as_deref() == Some(health.runner_digest.as_str())
        && health.status == COMPUTE_READY_HEALTHY
}
