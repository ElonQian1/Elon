use serde::{Deserialize, Serialize};

use crate::compute_federation::external_pool_adapter_release::{
    ComputeExternalPoolAdapterReleaseCapability, ComputeExternalPoolAdapterReleaseVerifierIntent,
};

pub(crate) const SANDBOX_CONFORMANCE_BINDING_SCHEMA: &str =
    "compute_federation.external_pool_adapter_sandbox_conformance_binding.v1";
pub(crate) const SANDBOX_CONFORMANCE_CHALLENGE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_sandbox_conformance_challenge.v1";
pub(crate) const SANDBOX_CONFORMANCE_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_sandbox_conformance_receipt.v1";
pub(crate) const SANDBOX_CONFORMANCE_CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_sandbox_conformance_currentness.v1";
pub(crate) const SANDBOX_CONFORMANCE_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const SANDBOX_CONFORMANCE_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const SANDBOX_CONFORMANCE_SIGNATURE_ALGORITHM: &str = "rsa-pkcs1v15-sha256";
pub(crate) const SANDBOX_CONFORMANCE_POLICY_ID: &str =
    "external_pool_adapter_six_capability_offline_sandbox_v1";
pub(crate) const SANDBOX_CONFORMANCE_ISOLATION_PROFILE_ID: &str =
    "offline_readonly_ephemeral_no_child_process_v1";
pub(crate) const SANDBOX_CONFORMANCE_CONFIRMATION: &str =
    "confirm_external_pool_adapter_sandbox_conformance";
pub(crate) const SANDBOX_CONFORMANCE_EVIDENCE_SCOPE: &str =
    "verifier_signature_over_exact_v236_artifact_server_derived_test_plan_and_asserted_observations";
pub(crate) const SANDBOX_CONFORMANCE_EFFECT: &str = "signed_sandbox_report_verified_current";
pub(crate) const SANDBOX_CONFORMANCE_NO_EFFECT: &str = "none";
pub(crate) const MAX_SANDBOX_REPORT_VALIDITY_HOURS: i64 = 24;
pub(crate) const MAX_SANDBOX_RUN_MINUTES: i64 = 30;
pub(crate) const MAX_SANDBOX_PEAK_MEMORY_BYTES: u64 = 536_870_912;
pub(crate) const MAX_SANDBOX_CPU_TIME_MS: u64 = 900_000;
pub(crate) const REQUIRED_SANDBOX_CAPABILITY_COUNT: usize = 6;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSandboxCapabilityObservation {
    pub capability_id: String,
    pub capability_revision: i64,
    pub test_case_id: String,
    pub outcome: String,
    pub output_transcript_digest: String,
    pub duration_ms: u64,
    pub policy_violation_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSandboxConformanceDraft {
    pub verifier_report_id: String,
    pub sandbox_runtime_id: String,
    pub runtime_image_digest: String,
    pub isolation_profile_id: String,
    pub run_started_at: String,
    pub run_completed_at: String,
    pub report_generated_at: String,
    pub report_expires_at: String,
    pub external_network_attempt_count: u64,
    pub write_outside_ephemeral_count: u64,
    pub child_process_attempt_count: u64,
    pub peak_memory_bytes: u64,
    pub cpu_time_ms: u64,
    pub observations: Vec<ExternalPoolAdapterSandboxCapabilityObservation>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSandboxCapabilityTest {
    pub capability_id: String,
    pub capability_revision: i64,
    pub test_case_id: String,
    pub input_fixture_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSandboxConformanceBinding {
    pub schema: String,
    pub admission_id: String,
    pub admission_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub declared_implementation_sha256: String,
    pub supported_capabilities: Vec<ComputeExternalPoolAdapterReleaseCapability>,
    pub capability_set_digest: String,
    pub expected_credential_verifier: ComputeExternalPoolAdapterReleaseVerifierIntent,
    pub vulnerability_report_receipt_id: String,
    pub vulnerability_report_receipt_digest: String,
    pub security_receipt_digest: String,
    pub package_receipt_digest: String,
    pub archive_sha256: String,
    pub sbom_digest: String,
    pub vulnerability_intelligence_expires_at: String,
    pub vulnerability_report_verified_at: String,
    pub sandbox_verifier_key_record_id: String,
    pub sandbox_verifier_key_record_digest: String,
    pub sandbox_verifier_key_id: String,
    pub sandbox_verifier_operator: String,
    pub sandbox_verifier_product: String,
    pub signature_algorithm: String,
    pub sandbox_policy_id: String,
    pub verifier_report_id: String,
    pub sandbox_runtime_id: String,
    pub runtime_image_digest: String,
    pub isolation_profile_id: String,
    pub run_started_at: String,
    pub run_completed_at: String,
    pub report_generated_at: String,
    pub report_expires_at: String,
    pub external_network_attempt_count: u64,
    pub write_outside_ephemeral_count: u64,
    pub child_process_attempt_count: u64,
    pub peak_memory_bytes: u64,
    pub cpu_time_ms: u64,
    pub test_plan_digest: String,
    pub test_plan: Vec<ExternalPoolAdapterSandboxCapabilityTest>,
    pub observation_inventory_digest: String,
    pub observations: Vec<ExternalPoolAdapterSandboxCapabilityObservation>,
    pub passed_capability_count: u64,
    pub policy_violation_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSandboxConformanceChallenge {
    pub schema: &'static str,
    pub canonicalization: &'static str,
    pub digest_algorithm: &'static str,
    pub signature_algorithm: &'static str,
    pub signature_message_base64: String,
    pub signature_message_digest: String,
    pub binding: ExternalPoolAdapterSandboxConformanceBinding,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSandboxConformanceMaterial {
    pub binding: ExternalPoolAdapterSandboxConformanceBinding,
    pub signature_message_digest: String,
    pub signature_base64: String,
    pub signature_digest: String,
    pub verified_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub verified_at: String,
    pub recorded_at: String,
    pub evidence_scope: String,
    pub conformance_effect: String,
    pub credential_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSandboxConformanceReceipt {
    pub schema: String,
    pub sandbox_conformance_receipt_id: String,
    pub sandbox_conformance_receipt_digest: String,
    pub conformance_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub conformance: ExternalPoolAdapterSandboxConformanceMaterial,
}
