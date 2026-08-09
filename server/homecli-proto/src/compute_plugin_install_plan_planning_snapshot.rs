use serde::{Deserialize, Serialize};

use crate::{ComputePluginInstallPlanKeyringBindingV1, ComputePluginSharingAuthorizationBindingV1};

mod validation;

/// A node binary that understands the planning-only V2 snapshot exchange.
pub const CAP_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_V2: &str =
    "compute_plugin_install_plan_planning_snapshot_v2";
pub const COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_PROTO_VERSION: u32 = 12;
pub const COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_REQUEST_V2_SCHEMA: &str =
    "elon.compute_plugin.install_plan_planning_snapshot_request.v2";
pub const COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_V2_SCHEMA: &str =
    "elon.compute_plugin.install_plan_planning_snapshot.v2";
pub const HASHED_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_V2_SCHEMA: &str =
    "elon.compute_plugin.hashed_install_plan_planning_snapshot.v2";
pub const COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_OBSERVED_V2_SCHEMA: &str =
    "elon.compute_plugin.install_plan_planning_snapshot_observed.v2";
pub const MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_INSTALLED_RECORDS: usize = 256;
pub const MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_BYTES: usize = 512 * 1024;
/// All V2 integer facts must fit the exact interoperable JSON integer range.
pub const MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
pub const MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_LIFETIME_MS: u64 = 5 * 60 * 1_000;

/// Requests a report bound to one exact, previously observed V1 preparation delivery.
/// Receiving this value never authorizes opening local state or performing installation work.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComputePluginInstallPlanPlanningSnapshotRequestV2 {
    pub schema: String,
    pub preparation_id: String,
    pub cloud_session_id: String,
    pub source_preparation_delivery_id: String,
    pub source_preparation_observation_digest: String,
    pub node_id: String,
    pub owner_user_id: String,
    pub installation_identity_digest: String,
    pub policy_revision: u64,
    pub policy_digest: String,
    pub policy_snapshot_digest: String,
    pub authorization: ComputePluginSharingAuthorizationBindingV1,
}

/// Exact release identity used only to explain an already durable inventory projection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComputePluginInstallPlanPlanningReleaseV2 {
    pub plugin_id: String,
    pub plugin_version: String,
    pub target_id: String,
    pub manifest_digest: String,
    pub package_digest: String,
}

/// A planning-visible candidate. Local slot paths and candidate bearer tokens never cross the
/// control wire.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComputePluginInstallPlanPlanningCandidateV2 {
    pub release: ComputePluginInstallPlanPlanningReleaseV2,
    pub phase: String,
    pub signed_manifest_envelope_digest: String,
}

/// Content-addressed identity of the work-admission head that still exactly matches the current
/// active release. Absence is represented by `None` on the installed record; historical or stale
/// local heads must never be projected as current.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComputePluginInstallPlanPlanningWorkAdmissionV2 {
    pub generation: u64,
    pub receipt_digest: String,
}

/// Bounded projection sufficient to choose an InstallPlan action. It deliberately excludes local
/// paths, health logs, download source references and runtime secrets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComputePluginInstallPlanPlanningInstalledRecordV2 {
    pub plugin_id: String,
    pub install_generation: u64,
    pub active_slot_ref: Option<String>,
    pub active_release: Option<ComputePluginInstallPlanPlanningReleaseV2>,
    pub active_install_receipt_digest: Option<String>,
    pub active_promotion_receipt_digest: Option<String>,
    pub active_signed_manifest_envelope_digest: Option<String>,
    pub candidate_slot_ref: Option<String>,
    pub candidate: Option<ComputePluginInstallPlanPlanningCandidateV2>,
    pub desired_presence: String,
    pub desired_activation: String,
    pub admission: String,
    pub runtime_phase: String,
    pub runtime_generation: u64,
    pub active_attempts: u64,
    pub permission_grant_digest: Option<String>,
    pub work_admission: Option<ComputePluginInstallPlanPlanningWorkAdmissionV2>,
}

/// One coherent, planning-only authority report. This is evidence for a future cloud planner, not
/// a local capability: no field can be converted into root, PlanApply, download or Sidecar access.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComputePluginInstallPlanPlanningSnapshotV2 {
    pub schema: String,
    pub preparation_id: String,
    pub cloud_session_id: String,
    pub source_preparation_delivery_id: String,
    pub source_preparation_observation_digest: String,
    pub node_id: String,
    pub owner_user_id: String,
    pub installation_identity_digest: String,
    pub policy_revision: u64,
    pub policy_digest: String,
    pub policy_snapshot_digest: String,
    pub sharing_enabled: bool,
    pub authorization: ComputePluginSharingAuthorizationBindingV1,
    pub bootstrap_instance_id: String,
    pub configuration_generation: u64,
    pub cancellation_generation: u64,
    pub policy_binding_receipt_digest: String,
    pub policy_capability_revocation_receipt_digest: String,
    pub policy_binding_source_preparation_id: String,
    pub policy_binding_authority_epoch: u64,
    pub policy_binding_process_owner_epoch: u64,
    pub authority_state_revision: u64,
    pub authority_epoch: u64,
    pub process_owner_epoch: u64,
    pub clock_epoch_digest: String,
    pub trusted_time_high_water_ms: u64,
    pub captured_at_ms: u64,
    pub expires_at_ms: u64,
    pub rollback_anchor_witness_digest: String,
    pub inventory_revision: u64,
    pub inventory_digest: String,
    pub node_profile_digest: String,
    pub manifest_catalog_revision: u64,
    pub manifest_catalog_digest: String,
    pub keyring_bundle_revision: u64,
    pub publisher_keyring: ComputePluginInstallPlanKeyringBindingV1,
    pub control_keyring: ComputePluginInstallPlanKeyringBindingV1,
    pub target_id: String,
    pub host_api_protocol_id: String,
    pub host_api_revision: u32,
    pub installed_records: Vec<ComputePluginInstallPlanPlanningInstalledRecordV2>,
}

/// The digest covers canonical `snapshot` bytes only; metadata makes the digest contract explicit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HashedComputePluginInstallPlanPlanningSnapshotV2 {
    pub schema: String,
    pub snapshot: ComputePluginInstallPlanPlanningSnapshotV2,
    pub canonicalization: String,
    pub snapshot_digest_algorithm: String,
    pub snapshot_digest: String,
}

/// Planning-only observation. In the initial V2 node implementation `snapshot_ready` is always
/// false and `snapshot` is always `None`; the explicit gates prevent an ACK from implying work.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComputePluginInstallPlanPlanningSnapshotObservedV2 {
    pub schema: String,
    pub preparation_id: String,
    pub cloud_session_id: String,
    pub source_preparation_delivery_id: String,
    pub source_preparation_observation_digest: String,
    pub node_id: String,
    pub owner_user_id: String,
    pub installation_identity_digest: Option<String>,
    pub accepted: bool,
    pub replayed: bool,
    pub snapshot_ready: bool,
    pub snapshot: Option<HashedComputePluginInstallPlanPlanningSnapshotV2>,
    pub observed_policy_revision: Option<u64>,
    pub observed_policy_digest: Option<String>,
    pub observed_policy_snapshot_digest: Option<String>,
    pub observed_authorization: Option<ComputePluginSharingAuthorizationBindingV1>,
    pub bootstrap_instance_id: String,
    pub phase: String,
    pub configuration_generation: u64,
    pub cancellation_generation: u64,
    pub local_confirmation_available: bool,
    pub compute_plugin_root_lock_acquired: bool,
    pub trusted_time_authority_configured: bool,
    pub rollback_anchor_witness_configured: bool,
    pub root_pinned: bool,
    pub authority_opened: bool,
    pub process_fence_acquired: bool,
    pub plan_apply_allowed: bool,
    pub new_work_admission_enabled: bool,
    pub downloads_allowed: bool,
    pub sidecar_launch_allowed: bool,
    pub side_effects_started: bool,
    pub blocked_reasons: Vec<String>,
    pub error_code: Option<String>,
}
