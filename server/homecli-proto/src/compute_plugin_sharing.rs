use serde::{Deserialize, Serialize};

/// A node binary that understands the versioned compute-plugin sharing policy snapshot and ACK.
pub const CAP_COMPUTE_PLUGIN_SHARING_V1: &str = "compute_plugin_sharing_v1";
pub const COMPUTE_PLUGIN_SHARING_PROTO_VERSION: u32 = 9;
pub const COMPUTE_PLUGIN_SHARING_POLICY_SNAPSHOT_V1_SCHEMA: &str =
    "elon.compute_plugin.sharing_policy_snapshot.v1";
pub const COMPUTE_PLUGIN_SHARING_POLICY_OBSERVED_V1_SCHEMA: &str =
    "elon.compute_plugin.sharing_policy_observed.v1";

/// Immutable authorization facts that a future signed InstallPlan must match exactly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComputePluginSharingAuthorizationBindingV1 {
    pub authorization_ref: String,
    pub revision: u64,
    pub digest: String,
}

/// One complete desired snapshot. The policy digest commits to the durable resolved policy;
/// policy details are deliberately not duplicated on the node-control wire.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComputePluginSharingPolicySnapshotV1 {
    pub schema: String,
    pub node_id: String,
    pub owner_user_id: String,
    pub installation_identity_digest: String,
    pub policy_revision: u64,
    pub policy_digest: String,
    pub plugin_runtime_requested: bool,
    pub authorization: Option<ComputePluginSharingAuthorizationBindingV1>,
}

/// Node observation of the locally accepted desired snapshot. Acceptance only updates dormant
/// in-memory Bootstrap state; `side_effects_started=false` remains authoritative until a later,
/// independently authorized initialization implementation exists.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComputePluginSharingPolicyObservedV1 {
    pub schema: String,
    pub node_id: String,
    pub owner_user_id: String,
    pub installation_identity_digest: Option<String>,
    pub accepted: bool,
    pub replayed: bool,
    pub observed_policy_revision: Option<u64>,
    pub observed_policy_digest: Option<String>,
    pub observed_snapshot_digest: Option<String>,
    pub phase: String,
    pub configuration_generation: u64,
    pub cancellation_generation: u64,
    pub side_effects_started: bool,
    pub blocked_reasons: Vec<String>,
    pub error_code: Option<String>,
}
