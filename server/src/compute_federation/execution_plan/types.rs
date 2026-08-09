use serde::{Deserialize, Serialize};

use crate::compute_federation::{
    attempt::{
        ComputeAttemptIdentity, ComputeAttemptModelBinding, ComputeAttemptRuntimeBinding,
        ComputeAttemptStart, ComputeAttemptUsageLimit,
    },
    capacity::ComputeCapacityClaimBinding,
    execution::{ComputeJobVersionBinding, ComputeOfferBinding},
};

pub(crate) const COMPUTE_ATTEMPT_EXECUTION_PLAN_SCHEMA: &str =
    "compute_federation.attempt_execution_plan.v1";
pub(crate) const COMPUTE_EXECUTION_CAPABILITY_SCHEMA: &str =
    "compute_federation.execution_capability.v1";
pub(crate) const COMPUTE_ARTIFACT_ACCESS_SCHEMA: &str = "compute_federation.artifact_access.v1";
pub(crate) const COMPUTE_EXECUTION_RESOURCE_GRANT_SCHEMA: &str =
    "compute_federation.execution_resource_grant.v1";
pub(crate) const COMPUTE_ATTEMPT_EXECUTION_PLAN_SEAL_SCHEMA: &str =
    "compute_federation.attempt_execution_plan_seal.v1";
pub(crate) const COMPUTE_EXECUTION_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const COMPUTE_EXECUTION_DIGEST_ALGORITHM: &str = "sha256";

pub(crate) const EXECUTION_CAPABILITY_NODE_READY: &str = "node_ready";
pub(crate) const EXECUTION_CAPABILITY_PROVIDER_ENDPOINT: &str = "provider_endpoint";
pub(crate) const EXECUTION_CAPABILITY_ADAPTER_EXECUTION: &str = "adapter_execution";

pub(crate) const ARTIFACT_ACCESS_READ: &str = "read";
pub(crate) const ARTIFACT_ACCESS_WRITE: &str = "write";

pub(crate) const RESOURCE_GRANT_NODE_HOST: &str = "node_host";
pub(crate) const RESOURCE_GRANT_PROVIDER_RUNTIME: &str = "provider_runtime";
pub(crate) const RESOURCE_GRANT_SERVER_ADAPTER: &str = "server_adapter";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptExecutionPlanEnvelope {
    pub schema: String,
    pub plan_id: String,
    pub plan_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub plan: ComputeAttemptExecutionPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptExecutionPlanSealEnvelope {
    pub schema: String,
    pub seal_id: String,
    pub seal_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub plan_id: String,
    pub plan_digest: String,
    pub capability_digest: String,
    pub artifact_access_count: i64,
    pub artifact_access_set_digest: String,
    pub resource_grant_digest: String,
    pub sealed_at: String,
}

/// Immutable, provider-neutral material from which an Adapter can prepare one exact Start.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptExecutionPlan {
    pub sources: ComputeAttemptExecutionSourceBindings,
    pub attempt: ComputeAttemptIdentity,
    pub route_binding_digest: String,
    pub capability: ComputeExecutionCapabilityBinding,
    pub start: ComputeAttemptStart,
    pub artifact_accesses: Vec<ComputeArtifactAccessBinding>,
    pub resource_grant: ComputeExecutionResourceGrant,
    pub lease_authority: ComputeLeaseAuthorityRequirement,
    pub required_route_capabilities: Vec<ComputeRequiredRouteCapability>,
    pub planned_at: String,
    pub not_after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeAttemptExecutionSourceBindings {
    pub consumer_account_id: String,
    pub provider: ComputeExecutionProviderVersionBinding,
    pub offer: ComputeOfferBinding,
    pub job: ComputeJobVersionBinding,
    pub reservation: ComputeExecutionReservationVersionBinding,
    pub capacity_claim: ComputeCapacityClaimBinding,
    pub price_snapshot: ComputeExecutionPriceSnapshotBinding,
    pub budget: ComputeExecutionBudgetReservationBinding,
    pub broker_request_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExecutionProviderVersionBinding {
    pub provider_id: String,
    pub provider_kind: String,
    pub provider_owner_account_id: String,
    pub policy_revision: i64,
    pub provider_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExecutionReservationVersionBinding {
    pub reservation_id: String,
    pub reservation_revision: i64,
    pub reservation_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExecutionPriceSnapshotBinding {
    pub price_snapshot_id: String,
    pub price_snapshot_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExecutionBudgetReservationBinding {
    pub budget_reservation_id: String,
    pub budget_reserved_fen: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExecutionCapabilityBinding {
    pub capability_id: String,
    pub capability_digest: String,
    pub capability_kind: String,
    pub provider_id: String,
    pub executor_id: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeArtifactAccessBinding {
    pub ordinal: i64,
    pub access_id: String,
    pub access_digest: String,
    pub access_kind: String,
    pub target_id: String,
    pub target_digest: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeLeaseAuthorityRequirement {
    pub authority_kind: String,
    pub delivery_mode: String,
    pub audience: String,
    pub attempt_lease_id: String,
    pub fencing_generation: i64,
    pub required_scopes: Vec<String>,
    pub valid_until: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeRequiredRouteCapability {
    pub capability_id: String,
    pub minimum_revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExecutionCapabilityEnvelope {
    pub schema: String,
    pub capability_id: String,
    pub capability_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub capability: ComputeExecutionCapability,
}

/// Normalized technical authority. Provider declarations and Offers cannot construct this fact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExecutionCapability {
    pub capability_kind: String,
    pub provider_id: String,
    pub provider_kind: String,
    pub executor_id: String,
    pub route: ComputeExecutionCapabilityRoute,
    pub provenance: ComputeExecutionCapabilityProvenance,
    pub runtime: ComputeAttemptRuntimeBinding,
    pub model: Option<ComputeAttemptModelBinding>,
    pub resource_ceiling: ComputeExecutionNumericResourceCeiling,
    pub node_ready: Option<ComputeNodeReadyCapabilityBinding>,
    pub observed_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExecutionCapabilityRoute {
    pub route_kind: String,
    pub route_binding_digest: String,
    pub endpoint_id: Option<String>,
    pub endpoint_transport: Option<String>,
    pub adapter_id: String,
    pub adapter_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExecutionCapabilityProvenance {
    pub source_schema: String,
    pub source_id: String,
    pub source_digest: String,
    pub verification_kind: String,
    pub verifier_id: String,
    pub verification_digest: String,
    pub authenticated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeNodeReadyCapabilityBinding {
    pub installation_identity_digest: String,
    pub inventory_revision: i64,
    pub install_generation: i64,
    pub activation_generation: i64,
    pub runtime_generation: i64,
    pub slot_ref: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExecutionNumericResourceCeiling {
    pub accelerator_count: i64,
    pub max_cpu_millicores: i64,
    pub max_memory_bytes: i64,
    pub max_vram_bytes: i64,
    pub max_disk_bytes: i64,
    pub max_processes: i64,
    pub max_runtime_seconds: i64,
    pub max_output_bytes: i64,
    pub max_concurrent_attempts: i64,
    pub allow_network_egress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeExecutionResourceGrant {
    pub schema: String,
    pub grant_id: String,
    pub grant_digest: String,
    pub enforcement_kind: String,
    pub accelerator_count: i64,
    pub cpu_millicores: i64,
    pub memory_bytes: i64,
    pub vram_bytes: i64,
    pub disk_bytes: i64,
    pub max_processes: i64,
    pub max_runtime_seconds: i64,
    pub max_output_bytes: i64,
    pub concurrency_units: i64,
    pub allow_network_egress: bool,
    pub usage_limits: Vec<ComputeAttemptUsageLimit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeArtifactAccessEnvelope {
    pub schema: String,
    pub access_id: String,
    pub access_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub access: ComputeArtifactAccess,
}

/// Authorization only. The reference is non-bearer; credentials and signed URLs stay out of JSON.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeArtifactAccess {
    pub non_bearer_access_ref: String,
    pub authorization_digest: String,
    pub audience: ComputeArtifactAccessAudience,
    pub target: ComputeArtifactAccessTarget,
    pub issued_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeArtifactAccessAudience {
    pub job_id: String,
    pub reservation_id: String,
    pub attempt_lease_id: String,
    pub provider_id: String,
    pub executor_id: String,
    pub fencing_generation: i64,
    pub route_binding_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "access_kind",
    content = "target",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(crate) enum ComputeArtifactAccessTarget {
    Read(ComputeArtifactReadAccess),
    Write(ComputeArtifactWriteAccess),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeArtifactReadAccess {
    pub artifact_id: String,
    pub digest_algorithm: String,
    pub artifact_digest: String,
    pub media_type: String,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeArtifactWriteAccess {
    pub namespace_id: String,
    pub namespace_digest: String,
    pub purpose: String,
    pub media_type: String,
    pub max_bytes: i64,
}
