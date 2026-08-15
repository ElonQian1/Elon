use serde::{Deserialize, Serialize};

use crate::compute_federation::external_pool_adapter_release::ComputeExternalPoolAdapterReleaseCapability;

use super::{
    TaskProtocolConformanceCapabilityObservation, TaskProtocolConformanceCleanupEvidence,
    TaskProtocolConformanceExchangeObservation,
};

pub(crate) const TASK_PROTOCOL_CONFORMANCE_PROFILE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_protocol_conformance_profile.v1";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_PROFILE_ENVELOPE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_protocol_conformance_profile_envelope.v1";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_FIXTURE_CATALOG_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_protocol_conformance_fixture_catalog.v1";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_FIXTURE_CATALOG_ENVELOPE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_protocol_conformance_fixture_catalog_envelope.v1";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_RUN_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_protocol_conformance_run_receipt.v1";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_REVOCATION_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_protocol_conformance_revocation_receipt.v1";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_protocol_conformance_currentness.v1";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_MAX_RECEIPT_JSON_BYTES: usize = 2 * 1024 * 1024;

pub(crate) const TASK_PROTOCOL_CONFORMANCE_PROFILE_ID: &str =
    "external_pool_adapter_task_protocol_conformance_profile_v1";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_PROFILE_REVISION: u64 = 1;
pub(crate) const TASK_PROTOCOL_CONFORMANCE_FIXTURE_CATALOG_ID: &str =
    "external_pool_adapter_task_protocol_conformance_fixture_catalog_v1";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_FIXTURE_CATALOG_REVISION: u64 = 1;
pub(crate) const TASK_PROTOCOL_CONFORMANCE_FIXTURE_LANE_ID: &str =
    "external_pool_adapter_task_protocol_conformance_fixture_lane_v1";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_FIXTURE_EXECUTOR_ID: &str =
    "external_pool_adapter_task_protocol_conformance_fixture_executor_v1";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_NON_PRODUCTION_AUTHORITY: &str =
    "non_production_no_v213_authority";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_RUN_STATUS: &str =
    "server_run_completed_no_production_authority";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_REVOCATION_STATUS: &str =
    "task_protocol_conformance_revoked_historical_only";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_RELATIONAL_CURRENT_STATUS: &str =
    "relationally_current_requires_process_custody_and_prepared_reproof";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_HISTORICAL_STATUS: &str = "historical_only";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_EVIDENCE_SCOPE: &str =
    "v272_server_owned_stateful_eight_exchange_six_capability_task_protocol_conformance_v1";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_CONFIRMATION: &str =
    "confirm_external_pool_adapter_task_protocol_conformance_run";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_REVOCATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_task_protocol_conformance_revocation";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_NO_EFFECT: &str = "none";
pub(crate) const TASK_PROTOCOL_CONFORMANCE_EXPIRY_SECONDS: i64 = 15;
pub(crate) const TASK_PROTOCOL_CONFORMANCE_SESSION_ROOT_COUNT: usize = 14;
pub(crate) const TASK_PROTOCOL_CONFORMANCE_EXCHANGE_COUNT: usize = 8;
pub(crate) const TASK_PROTOCOL_CONFORMANCE_CAPABILITY_COUNT: usize = 6;
pub(crate) const TASK_PROTOCOL_CONFORMANCE_MAX_ORDINAL: u64 = 64;
pub(crate) const TASK_PROTOCOL_CONFORMANCE_EXCHANGE_TIMEOUT_MS: u64 = 15_000;
pub(crate) const TASK_PROTOCOL_CONFORMANCE_MAX_REQUEST_BYTES: u64 = 262_144;
pub(crate) const TASK_PROTOCOL_CONFORMANCE_MAX_UPSTREAM_REQUEST_BYTES: u64 = 65_536;
pub(crate) const TASK_PROTOCOL_CONFORMANCE_MAX_RESPONSE_BYTES: u64 = 262_144;
pub(crate) const TASK_PROTOCOL_CONFORMANCE_MAX_OBSERVATION_BYTES: u64 = 262_144;
pub(crate) const TASK_PROTOCOL_CONFORMANCE_MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceEffects {
    pub credential_effect: String,
    pub adapter_effect: String,
    pub provider_effect: String,
    pub route_effect: String,
    pub activation_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceReadiness {
    pub process_spawn_ready: bool,
    pub ipc_session_ready: bool,
    pub secret_delivery_ready: bool,
    pub broker_connect_ready: bool,
    pub upstream_probe_ready: bool,
    pub runtime_launch_ready: bool,
    pub route_ready: bool,
    pub execution_ready: bool,
    pub activation_ready: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceCodePoint {
    pub name: String,
    pub code: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceProfile {
    pub schema: String,
    pub profile_id: String,
    pub profile_revision: u64,
    pub host_os: String,
    pub host_arch: String,
    pub wire_prefix: String,
    pub wire_version: u64,
    pub control_kinds: Vec<ExternalPoolAdapterTaskProtocolConformanceCodePoint>,
    pub operations: Vec<ExternalPoolAdapterTaskProtocolConformanceCodePoint>,
    pub required_capabilities: Vec<ComputeExternalPoolAdapterReleaseCapability>,
    pub session_root_names: Vec<String>,
    pub session_roots_domain: String,
    pub session_kdf_salt_domain: String,
    pub request_digest_domain: String,
    pub exchange_digest_domain: String,
    pub first_ordinal: u64,
    pub max_ordinal: u64,
    pub exchange_timeout_ms: u64,
    pub max_request_bytes: u64,
    pub max_upstream_request_bytes: u64,
    pub max_response_bytes: u64,
    pub max_observation_bytes: u64,
    pub framing_policy: String,
    pub reserved_policy: String,
    pub authenticated_ack_policy: String,
    pub cleanup_policy: String,
    pub authority_status: String,
    pub effects: ExternalPoolAdapterTaskProtocolConformanceEffects,
    pub readiness: ExternalPoolAdapterTaskProtocolConformanceReadiness,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceProfileEnvelope {
    pub schema: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub profile_digest: String,
    pub profile: ExternalPoolAdapterTaskProtocolConformanceProfile,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceFixtureExchange {
    pub exchange_ordinal: u64,
    pub scenario_id: String,
    pub operation_kind: String,
    pub capability_id: String,
    pub replay_kind: String,
    pub allowed_state_before: Vec<String>,
    pub allowed_state_after: Vec<String>,
    pub terminality: String,
    pub reference_required: bool,
    pub remote_sequence: Option<u64>,
    pub tombstone_required: bool,
    pub event_kinds: Vec<String>,
    pub commit_uncertainty_state_before: String,
    pub commit_uncertainty_state_after: String,
    pub commit_uncertainty_marker_required: bool,
    pub event_replay_classification: Option<String>,
    pub expected_event_replay_batch_count: u64,
    pub event_replay_root_required: bool,
    pub expected_start_count: u64,
    pub expected_event_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceFixtureCatalog {
    pub schema: String,
    pub catalog_id: String,
    pub catalog_revision: u64,
    pub scenario_ids: Vec<String>,
    pub exchanges: Vec<ExternalPoolAdapterTaskProtocolConformanceFixtureExchange>,
    pub capability_order: Vec<String>,
    pub capability_exchange_ordinals: Vec<Vec<u64>>,
    pub capability_evidence_policy: String,
    pub response_policy: String,
    pub authority_status: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceFixtureCatalogEnvelope {
    pub schema: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub catalog_digest: String,
    pub catalog: ExternalPoolAdapterTaskProtocolConformanceFixtureCatalog,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceRegistryReleaseRoots {
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub registry_release_material_digest: String,
    pub admission_id: String,
    pub admission_digest: String,
    pub package_receipt_id: String,
    pub package_receipt_digest: String,
    pub package_material_digest: String,
    pub source_receipt_id: String,
    pub source_receipt_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub route_kind: String,
    pub implementation_digest: String,
    pub declared_implementation_sha256: String,
    pub entrypoint_path: String,
    pub entrypoint_sha256: String,
    pub entrypoint_size_bytes: u64,
    pub installation_content_digest: String,
    pub supported_capabilities: Vec<ComputeExternalPoolAdapterReleaseCapability>,
    pub capability_set_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceVulnerabilityRoots {
    pub reattestation_receipt_id: String,
    pub reattestation_receipt_digest: String,
    pub reattestation_material_digest: String,
    pub intelligence_snapshot_digest: String,
    pub intelligence_expires_at: String,
    pub blocking_finding_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceSandboxRoots {
    pub reattestation_receipt_id: String,
    pub reattestation_receipt_digest: String,
    pub reattestation_material_digest: String,
    pub sandbox_policy_id: String,
    pub test_plan_digest: String,
    pub observation_inventory_digest: String,
    pub report_expires_at: String,
    pub passed_capability_count: u64,
    pub policy_violation_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceSandboxVerifierKeyRoots {
    pub key_record_id: String,
    pub key_record_digest: String,
    pub key_id: String,
    pub verifier_operator: String,
    pub verifier_product: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceRuntimeCompatibilityRoots {
    pub verification_receipt_id: String,
    pub verification_receipt_digest: String,
    pub verification_material_digest: String,
    pub run_observation_id: String,
    pub run_observation_digest: String,
    pub run_observation_material_digest: String,
    pub runner_execution_id: String,
    pub profile_id: String,
    pub profile_revision: u64,
    pub profile_digest: String,
    pub runner_policy_digest: String,
    pub fixture_catalog_digest: String,
    pub supervisor_session_policy_digest: String,
    pub source_capsule_sha256: String,
    pub source_capsule_size_bytes: u64,
    pub launch_image_sha256: String,
    pub launch_image_size_bytes: u64,
    pub public_fixture_delivery_root: String,
    pub expires_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceRunRoots {
    pub registry_release: ExternalPoolAdapterTaskProtocolConformanceRegistryReleaseRoots,
    pub vulnerability_reattestation: ExternalPoolAdapterTaskProtocolConformanceVulnerabilityRoots,
    pub sandbox_reattestation: ExternalPoolAdapterTaskProtocolConformanceSandboxRoots,
    pub sandbox_verifier_key: ExternalPoolAdapterTaskProtocolConformanceSandboxVerifierKeyRoots,
    pub runtime_compatibility: ExternalPoolAdapterTaskProtocolConformanceRuntimeCompatibilityRoots,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceSyntheticSubject {
    pub subject_kind: String,
    pub subject_id: String,
    pub subject_digest: String,
    pub authority_status: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceSyntheticSubjects {
    pub fixture_lane: ExternalPoolAdapterTaskProtocolConformanceSyntheticSubject,
    pub fixture_executor: ExternalPoolAdapterTaskProtocolConformanceSyntheticSubject,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformancePredecessor {
    pub run_receipt_id: String,
    pub run_receipt_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceRunMaterial {
    pub registry_release: ExternalPoolAdapterTaskProtocolConformanceRegistryReleaseRoots,
    pub vulnerability_reattestation: ExternalPoolAdapterTaskProtocolConformanceVulnerabilityRoots,
    pub sandbox_reattestation: ExternalPoolAdapterTaskProtocolConformanceSandboxRoots,
    pub sandbox_verifier_key: ExternalPoolAdapterTaskProtocolConformanceSandboxVerifierKeyRoots,
    pub runtime_compatibility: ExternalPoolAdapterTaskProtocolConformanceRuntimeCompatibilityRoots,
    pub task_protocol_profile_id: String,
    pub task_protocol_profile_revision: u64,
    pub task_protocol_profile_digest: String,
    pub fixture_catalog_id: String,
    pub fixture_catalog_revision: u64,
    pub fixture_catalog_digest: String,
    pub synthetic_subjects: ExternalPoolAdapterTaskProtocolConformanceSyntheticSubjects,
    pub session_root_digests: Vec<String>,
    pub run_nonce_digest: String,
    /// Fresh V272 delivery root. The randomized V268 delivery root remains nested in the V268
    /// lineage above and must not be substituted here.
    pub public_fixture_delivery_root: String,
    pub session_roots_digest: String,
    pub session_transcript_digest: String,
    pub delivery_inventory_digest: String,
    pub exchange_inventory_digest: String,
    pub task_observation_root: String,
    pub exchanges: Vec<TaskProtocolConformanceExchangeObservation>,
    pub capabilities: Vec<TaskProtocolConformanceCapabilityObservation>,
    pub cleanup: TaskProtocolConformanceCleanupEvidence,
    pub duration_ms: u64,
    pub sequence: u64,
    pub predecessor_run_receipt_id: Option<String>,
    pub predecessor_run_receipt_digest: Option<String>,
    pub run_started_at: String,
    pub run_completed_at: String,
    pub post_cleanup_checked_at: String,
    pub expires_at: String,
    pub recorded_at: String,
    pub evidence_scope: String,
    pub receipt_status: String,
    pub non_production_authority_status: String,
    pub effects: ExternalPoolAdapterTaskProtocolConformanceEffects,
    pub readiness: ExternalPoolAdapterTaskProtocolConformanceReadiness,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceRunReceipt {
    pub schema: String,
    pub run_receipt_id: String,
    pub run_receipt_digest: String,
    pub run_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub run: ExternalPoolAdapterTaskProtocolConformanceRunMaterial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceRevocationMaterial {
    pub run_receipt_id: String,
    pub run_receipt_digest: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub reason: String,
    pub revoked_at: String,
    pub recorded_at: String,
    pub revocation_status: String,
    pub effects: ExternalPoolAdapterTaskProtocolConformanceEffects,
    pub readiness: ExternalPoolAdapterTaskProtocolConformanceReadiness,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceRevocationReceipt {
    pub schema: String,
    pub revocation_receipt_id: String,
    pub revocation_receipt_digest: String,
    pub revocation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub revocation: ExternalPoolAdapterTaskProtocolConformanceRevocationMaterial,
}
