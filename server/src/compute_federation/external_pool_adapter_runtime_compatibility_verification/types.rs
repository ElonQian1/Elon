use serde::{Deserialize, Serialize};

use crate::compute_federation::external_pool_adapter_release::ComputeExternalPoolAdapterReleaseCapability;

use super::release_types::ExternalPoolAdapterRuntimeCompatibilityRegistryReleaseRoots;

pub(crate) const RUNTIME_COMPATIBILITY_V2_PROFILE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_compatibility_profile.v2";
pub(crate) const RUNTIME_COMPATIBILITY_V2_PROFILE_ENVELOPE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_compatibility_profile_envelope.v2";
pub(crate) const RUNTIME_COMPATIBILITY_V2_PROFILE_ID: &str =
    "external_pool_adapter_linux_runtime_compatibility_v2";
pub(crate) const RUNTIME_COMPATIBILITY_V2_PROFILE_REVISION: u64 = 2;
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_CHALLENGE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_compatibility_verification_challenge.v1";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_OBSERVATION_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_compatibility_server_run_observation.v1";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_SIGNATURE_CHALLENGE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_compatibility_signature_challenge.v1";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_compatibility_verification_receipt.v1";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_REVOCATION_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_compatibility_revocation_receipt.v1";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_compatibility_currentness.v1";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_SIGNATURE_ALGORITHM: &str =
    "rsa-pkcs1v15-sha256";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_CHALLENGE_VALIDITY_MINUTES: i64 = 5;
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_RECEIPT_VALIDITY_HOURS: i64 = 24;
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_MAX_RECEIPT_JSON_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_CHALLENGE_CONFIRMATION: &str =
    "confirm_external_pool_adapter_runtime_compatibility_challenge";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_runtime_compatibility_signature";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_REVOCATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_runtime_compatibility_revocation";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_OBSERVATION_STATUS: &str =
    "server_run_observed_no_authority";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_SIGNED_RECEIPT_STATUS: &str =
    "signed_verifier_assertion_over_server_run_observation_no_activation_authority";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_REVOCATION_STATUS: &str =
    "signed_verifier_assertion_revoked_historical_only";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_CURRENT_STATUS: &str =
    "current_signed_verifier_assertion";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_HISTORICAL_STATUS: &str =
    "historical_signed_verifier_assertion";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_EVIDENCE_SCOPE: &str =
    "v237_signature_over_server_owned_controlled_public_fixture_runtime_observation_v1";
pub(crate) const RUNTIME_COMPATIBILITY_VERIFICATION_NO_EFFECT: &str = "none";

pub(crate) const REQUIRED_RUNTIME_COMPATIBILITY_OBSERVATIONS: [&str; 10] = [
    "source_capsule_materialized",
    "derived_launch_image",
    "authenticated_bootstrap",
    "public_config_delivery",
    "public_credential_delivery",
    "adapter_request_exact_match",
    "public_no_work_response_delivery",
    "authenticated_shutdown",
    "bounded_reap",
    "cgroup_cleanup",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityPolicyRef {
    pub policy_id: String,
    pub policy_revision: u64,
    pub policy_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityFixtureResourceRequirement {
    pub purpose: String,
    pub path: String,
    pub role: String,
    pub min_size_bytes: u64,
    pub max_size_bytes: u64,
    pub public_fixture_only: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityFixtureResourceIdentity {
    pub purpose: String,
    pub path: String,
    pub role: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityEffects {
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
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityReadiness {
    pub process_ready: bool,
    pub session_ready: bool,
    pub secret_delivery_ready: bool,
    pub broker_connect_ready: bool,
    pub upstream_probe_ready: bool,
    pub runtime_launch_ready: bool,
    pub activation_ready: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityRunnerPolicy {
    pub policy_id: String,
    pub policy_revision: u64,
    pub host_os: String,
    pub host_arch: String,
    pub runner_owner: String,
    pub retained_file_policy: String,
    pub launch_image_derivation_policy: String,
    pub post_exec_dumpable_policy: String,
    pub exec_transition_ptrace_guard: String,
    pub seqpacket_ancillary_policy: String,
    pub no_work_protocol_policy: String,
    pub fixture_delivery_policy: String,
    pub request_match_policy: String,
    pub response_policy: String,
    pub network_policy: String,
    pub upstream_policy: String,
    pub cleanup_policy: String,
    pub observation_commit_policy: String,
    pub challenge_validity_seconds: u64,
    pub verification_receipt_validity_seconds: u64,
    pub max_run_seconds: u64,
    pub max_probe_timeout_ms: u64,
    pub caller_supplied_material_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityPublicFixtureCatalog {
    pub catalog_id: String,
    pub catalog_revision: u64,
    pub inventory_policy: String,
    pub resources: Vec<ExternalPoolAdapterRuntimeCompatibilityFixtureResourceRequirement>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityProfileV2 {
    pub schema: String,
    pub profile_id: String,
    pub profile_revision: u64,
    pub host_os: String,
    pub host_arch: String,
    pub release_capabilities: Vec<ComputeExternalPoolAdapterReleaseCapability>,
    pub runtime_launch_policy: ExternalPoolAdapterRuntimeCompatibilityPolicyRef,
    pub upstream_transport_policy: ExternalPoolAdapterRuntimeCompatibilityPolicyRef,
    pub supervisor_session_policy: ExternalPoolAdapterRuntimeCompatibilityPolicyRef,
    pub source_capsule_policy: ExternalPoolAdapterRuntimeCompatibilityPolicyRef,
    pub runner_policy: ExternalPoolAdapterRuntimeCompatibilityPolicyRef,
    pub fixture_catalog: ExternalPoolAdapterRuntimeCompatibilityPolicyRef,
    pub evidence_scope: String,
    pub effects: ExternalPoolAdapterRuntimeCompatibilityEffects,
    pub readiness: ExternalPoolAdapterRuntimeCompatibilityReadiness,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityProfileV2Envelope {
    pub schema: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub profile_digest: String,
    pub profile: ExternalPoolAdapterRuntimeCompatibilityProfileV2,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityChallengeMaterial {
    pub schema: String,
    pub challenge_id: String,
    pub challenge_nonce_base64: String,
    pub challenge_nonce_digest: String,
    pub issued_at: String,
    pub expires_at: String,
    pub registry_release: ExternalPoolAdapterRuntimeCompatibilityRegistryReleaseRoots,
    pub runtime_kind: String,
    pub entrypoint_path: String,
    pub entrypoint_sha256: String,
    pub entrypoint_size_bytes: u64,
    pub profile_id: String,
    pub profile_revision: u64,
    pub profile_digest: String,
    pub runtime_launch_policy: ExternalPoolAdapterRuntimeCompatibilityPolicyRef,
    pub upstream_transport_policy: ExternalPoolAdapterRuntimeCompatibilityPolicyRef,
    pub supervisor_session_policy: ExternalPoolAdapterRuntimeCompatibilityPolicyRef,
    pub source_capsule_policy: ExternalPoolAdapterRuntimeCompatibilityPolicyRef,
    pub runner_policy: ExternalPoolAdapterRuntimeCompatibilityPolicyRef,
    pub fixture_catalog: ExternalPoolAdapterRuntimeCompatibilityPolicyRef,
    pub fixture_resources: Vec<ExternalPoolAdapterRuntimeCompatibilityFixtureResourceIdentity>,
    pub sandbox_verifier_key_record_id: String,
    pub sandbox_verifier_key_record_digest: String,
    pub sandbox_verifier_key_id: String,
    pub sandbox_verifier_operator: String,
    pub sandbox_verifier_product: String,
    pub signature_algorithm: String,
    pub sequence: u64,
    pub predecessor_verification_receipt_id: Option<String>,
    pub predecessor_verification_receipt_digest: Option<String>,
    pub created_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub recorded_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt {
    pub schema: String,
    pub challenge_digest: String,
    pub challenge_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub challenge: ExternalPoolAdapterRuntimeCompatibilityChallengeMaterial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityObservation {
    pub observation_id: String,
    pub observation_revision: u64,
    pub outcome: String,
    pub duration_ms: u64,
    pub policy_violation_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityNoWorkEvidence {
    pub probe_nonce_digest: String,
    pub request_bytes: u64,
    pub response_bytes: u64,
    pub request_sha256: String,
    pub response_sha256: String,
    pub probe_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityServerRunObservationMaterial {
    pub runner_execution_id: String,
    pub challenge_id: String,
    pub challenge_digest: String,
    pub challenge_nonce_digest: String,
    pub registry_release: ExternalPoolAdapterRuntimeCompatibilityRegistryReleaseRoots,
    pub profile_id: String,
    pub profile_revision: u64,
    pub profile_digest: String,
    pub runner_policy_digest: String,
    pub fixture_catalog_digest: String,
    pub source_capsule_sha256: String,
    pub source_capsule_size_bytes: u64,
    pub source_capsule_policy_digest: String,
    pub launch_image_sha256: String,
    pub launch_image_size_bytes: u64,
    pub public_fixture_delivery_root: String,
    pub run_started_at: String,
    pub run_completed_at: String,
    pub recorded_at: String,
    pub fixture_resources: Vec<ExternalPoolAdapterRuntimeCompatibilityFixtureResourceIdentity>,
    pub observations: Vec<ExternalPoolAdapterRuntimeCompatibilityObservation>,
    pub no_work: ExternalPoolAdapterRuntimeCompatibilityNoWorkEvidence,
    pub child_network_attempt_count: u64,
    pub upstream_connect_attempt_count: u64,
    pub write_outside_ephemeral_count: u64,
    pub additional_process_attempt_count: u64,
    pub policy_violation_count: u64,
    pub observation_status: String,
    pub effects: ExternalPoolAdapterRuntimeCompatibilityEffects,
    pub readiness: ExternalPoolAdapterRuntimeCompatibilityReadiness,
}

/// Store-runner handoff. It is deliberately non-Clone/non-Debug/non-Serde.
pub(crate) struct PreparedExternalPoolAdapterRuntimeCompatibilityServerRunObservation {
    pub(super) material: ExternalPoolAdapterRuntimeCompatibilityServerRunObservationMaterial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt {
    pub schema: String,
    pub run_observation_id: String,
    pub run_observation_digest: String,
    pub run_observation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub observation: ExternalPoolAdapterRuntimeCompatibilityServerRunObservationMaterial,
}

#[derive(Clone, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilitySignatureChallenge {
    pub schema: &'static str,
    pub canonicalization: &'static str,
    pub digest_algorithm: &'static str,
    pub signature_algorithm: &'static str,
    pub signature_message_base64: String,
    pub signature_message_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityVerificationMaterial {
    pub runner_execution_id: String,
    pub challenge_id: String,
    pub challenge_digest: String,
    pub run_observation_id: String,
    pub run_observation_digest: String,
    pub run_observation_material_digest: String,
    pub registry_release: ExternalPoolAdapterRuntimeCompatibilityRegistryReleaseRoots,
    pub profile_id: String,
    pub profile_revision: u64,
    pub profile_digest: String,
    pub runner_policy_digest: String,
    pub fixture_catalog_digest: String,
    pub public_fixture_delivery_root: String,
    pub sandbox_verifier_key_record_id: String,
    pub sandbox_verifier_key_record_digest: String,
    pub sandbox_verifier_key_id: String,
    pub sandbox_verifier_operator: String,
    pub sandbox_verifier_product: String,
    pub signature_algorithm: String,
    pub sequence: u64,
    pub predecessor_verification_receipt_id: Option<String>,
    pub predecessor_verification_receipt_digest: Option<String>,
    pub signature_message_digest: String,
    pub signature_base64: String,
    pub signature_digest: String,
    pub verified_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub verified_at: String,
    pub recorded_at: String,
    pub expires_at: String,
    pub evidence_scope: String,
    pub receipt_status: String,
    pub effects: ExternalPoolAdapterRuntimeCompatibilityEffects,
    pub readiness: ExternalPoolAdapterRuntimeCompatibilityReadiness,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilityVerificationReceipt {
    pub schema: String,
    pub verification_receipt_id: String,
    pub verification_receipt_digest: String,
    pub verification_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub verification: ExternalPoolAdapterRuntimeCompatibilityVerificationMaterial,
}
