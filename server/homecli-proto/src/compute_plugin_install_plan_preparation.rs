use serde::{Deserialize, Serialize};

use crate::ComputePluginSharingAuthorizationBindingV1;

/// A node binary that can inspect one InstallPlan preparation request without side effects.
pub const CAP_COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_V1: &str =
    "compute_plugin_install_plan_preparation_v1";
pub const COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_PROTO_VERSION: u32 = 10;
pub const COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_V1_SCHEMA: &str =
    "elon.compute_plugin.install_plan_preparation_request.v1";
pub const COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_CONTEXT_V1_SCHEMA: &str =
    "elon.compute_plugin.install_plan_preparation_context.v1";
pub const COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_OBSERVED_V1_SCHEMA: &str =
    "elon.compute_plugin.install_plan_preparation_observed.v1";

/// Exact immutable-consent binding for a future signed InstallPlan preparation.
/// Receiving this request does not authorize opening local authority state or downloading bytes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComputePluginInstallPlanPreparationRequestV1 {
    pub schema: String,
    pub preparation_id: String,
    pub node_id: String,
    pub owner_user_id: String,
    pub installation_identity_digest: String,
    pub policy_revision: u64,
    pub policy_digest: String,
    pub policy_snapshot_digest: String,
    pub authorization: ComputePluginSharingAuthorizationBindingV1,
}

/// One exact keyring revision/digest pair. The node cannot produce this until production trust
/// bootstrap has installed and revalidated the corresponding durable keyring snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComputePluginInstallPlanKeyringBindingV1 {
    pub revision: u64,
    pub digest: String,
}

/// Future no-side-effect input facts for cloud InstallPlan generation. It is deliberately optional
/// in V1: nodes must return `None` until every field comes from one coherent local authority read.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComputePluginInstallPlanPreparationContextV1 {
    pub schema: String,
    pub expected_inventory_revision: u64,
    pub expected_inventory_digest: String,
    pub node_profile_digest: String,
    pub manifest_catalog_revision: u64,
    pub publisher_keyring: ComputePluginInstallPlanKeyringBindingV1,
    pub control_keyring: ComputePluginInstallPlanKeyringBindingV1,
}

/// Node observation of one preparation request. `accepted` only means that its identity and
/// sharing bindings match current dormant Bootstrap intent. `context_ready` is the sole signal
/// that a generation context exists, and remains false until production authority is connected.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComputePluginInstallPlanPreparationObservedV1 {
    pub schema: String,
    pub preparation_id: String,
    pub node_id: String,
    pub owner_user_id: String,
    pub installation_identity_digest: Option<String>,
    pub accepted: bool,
    pub replayed: bool,
    pub context_ready: bool,
    pub context: Option<ComputePluginInstallPlanPreparationContextV1>,
    pub observed_policy_revision: Option<u64>,
    pub observed_policy_digest: Option<String>,
    pub observed_policy_snapshot_digest: Option<String>,
    pub observed_authorization: Option<ComputePluginSharingAuthorizationBindingV1>,
    /// Random process-local identity. Historical ACKs from another process cannot reconstruct
    /// preparation state after a node restart.
    pub bootstrap_instance_id: String,
    pub phase: String,
    pub configuration_generation: u64,
    pub cancellation_generation: u64,
    pub compute_plugin_root_lock_acquired: bool,
    pub trusted_time_authority_configured: bool,
    pub rollback_anchor_witness_configured: bool,
    pub root_pinned: bool,
    pub authority_opened: bool,
    pub process_fence_acquired: bool,
    pub new_work_admission_enabled: bool,
    pub downloads_allowed: bool,
    pub side_effects_started: bool,
    pub blocked_reasons: Vec<String>,
    pub error_code: Option<String>,
}
