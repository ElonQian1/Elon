use serde::{Deserialize, Serialize};

pub(crate) const COMPUTE_PROVIDER_SCHEMA: &str = "compute_federation.provider.v1";

pub(crate) const PROVIDER_KIND_USER_NODE: &str = "user_node";
pub(crate) const PROVIDER_KIND_MANAGED_CLUSTER: &str = "managed_cluster";
pub(crate) const PROVIDER_KIND_EXTERNAL_POOL: &str = "external_pool";

pub(crate) const PROVIDER_STATUS_REGISTERING: &str = "registering";
pub(crate) const PROVIDER_STATUS_ACTIVE: &str = "active";
pub(crate) const PROVIDER_STATUS_DRAINING: &str = "draining";
pub(crate) const PROVIDER_STATUS_DISABLED: &str = "disabled";
pub(crate) const PROVIDER_STATUS_QUARANTINED: &str = "quarantined";

/// Stable provider identity. Volatile capacity belongs to versioned offers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeProvider {
    pub schema: String,
    pub provider_id: String,
    pub provider_kind: String,
    pub owner_account_id: String,
    pub settlement_account_id: Option<String>,
    pub display_name: String,
    pub status: String,
    pub trust_tier: String,
    pub home_region: Option<String>,
    pub policy_revision: i64,
    pub capabilities: ComputeProviderCapabilities,
    pub endpoint: Option<ComputeProviderEndpointRef>,
    pub adapter: Option<ComputeProviderAdapterRef>,
    pub evidence_profile: ComputeProviderEvidenceProfile,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeProviderCapabilities {
    pub task_kinds: Vec<String>,
    pub accelerator_kinds: Vec<String>,
    pub regions: Vec<String>,
    pub allowed_data_classes: Vec<String>,
    pub supports_streaming: bool,
    pub supports_checkpointing: bool,
    pub max_parallel_attempts: i64,
}

/// Contains routing references only. Provider credentials never enter this contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeProviderEndpointRef {
    pub endpoint_id: String,
    pub transport: String,
    pub address_hint: Option<String>,
    pub gateway_id: Option<String>,
    pub credential_ref: Option<String>,
}

/// Server-side adapter for a managed cluster or an external compute pool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeProviderAdapterRef {
    pub adapter_id: String,
    pub adapter_version: String,
    pub config_revision: i64,
    pub config_digest: String,
}

/// Declared, observed and verified profiles remain separate trust facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeProviderEvidenceProfile {
    pub declared_hardware_digest: Option<String>,
    pub observed_hardware_digest: Option<String>,
    pub verified_hardware_digest: Option<String>,
    pub last_observed_at: Option<String>,
    pub last_verified_at: Option<String>,
}
