use serde::{Deserialize, Serialize};

use crate::compute_federation::{
    external_pool_adapter_artifact_sandbox_conformance::{
        ExternalPoolAdapterSandboxCapabilityObservation, ExternalPoolAdapterSandboxCapabilityTest,
    },
    external_pool_adapter_release::{
        ComputeExternalPoolAdapterReleaseCapability,
        ComputeExternalPoolAdapterReleaseVerifierIntent,
    },
};

pub(crate) const SANDBOX_REATTESTATION_BINDING_SCHEMA: &str =
    "compute_federation.external_pool_adapter_sandbox_reattestation_binding.v1";
pub(crate) const SANDBOX_REATTESTATION_CHALLENGE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_sandbox_reattestation_challenge.v1";
pub(crate) const SANDBOX_REATTESTATION_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_sandbox_reattestation_receipt.v1";
pub(crate) const SANDBOX_REATTESTATION_REVOCATION_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_sandbox_reattestation_revocation_receipt.v1";
pub(crate) const SANDBOX_REATTESTATION_CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_sandbox_reattestation_currentness.v1";
pub(crate) const SANDBOX_REATTESTATION_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const SANDBOX_REATTESTATION_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const SANDBOX_REATTESTATION_SIGNATURE_ALGORITHM: &str = "rsa-pkcs1v15-sha256";
pub(crate) const SANDBOX_REATTESTATION_POLICY_ID: &str =
    "external_pool_adapter_six_capability_offline_sandbox_v1";
pub(crate) const SANDBOX_REATTESTATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_sandbox_reattestation";
pub(crate) const SANDBOX_REATTESTATION_REVOCATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_sandbox_reattestation_revocation";
pub(crate) const SANDBOX_REATTESTATION_EVIDENCE_SCOPE: &str =
    "verifier_signature_over_exact_v249_registry_release_current_v250_vulnerability_reattestation_server_derived_test_plan_and_single_use_nonce";
pub(crate) const SANDBOX_REATTESTATION_EFFECT: &str =
    "signed_sandbox_reattestation_verified_current";
pub(crate) const SANDBOX_REATTESTATION_REVOCATION_EFFECT: &str = "sandbox_reattestation_revoked";
pub(crate) const SANDBOX_REATTESTATION_NO_EFFECT: &str = "none";
pub(crate) const SANDBOX_REATTESTATION_CHALLENGE_VALIDITY_MINUTES: i64 = 5;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSandboxReattestationBinding {
    pub schema: String,
    pub challenge_id: String,
    pub challenge_nonce_base64: String,
    pub challenge_nonce_digest: String,
    pub challenge_issued_at: String,
    pub challenge_expires_at: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub registry_release_material_digest: String,
    pub admission_id: String,
    pub admission_digest: String,
    pub package_receipt_id: String,
    pub package_receipt_digest: String,
    pub source_receipt_id: String,
    pub source_receipt_digest: String,
    pub adapter_id: String,
    pub release_version: String,
    pub route_kind: String,
    pub supported_provider_kinds: Vec<String>,
    pub implementation_digest: String,
    pub declared_implementation_sha256: String,
    pub supported_capabilities: Vec<ComputeExternalPoolAdapterReleaseCapability>,
    pub capability_set_digest: String,
    pub expected_credential_verifier: ComputeExternalPoolAdapterReleaseVerifierIntent,
    pub credential_verifier_digest: String,
    pub archive_sha256: String,
    pub archive_size_bytes: u64,
    pub manifest_digest: String,
    pub entry_inventory_digest: String,
    pub entry_count: u64,
    pub total_uncompressed_bytes: u64,
    pub installation_content_digest: String,
    pub vulnerability_reattestation_receipt_id: String,
    pub vulnerability_reattestation_receipt_digest: String,
    pub vulnerability_reattestation_material_digest: String,
    pub vulnerability_reattestation_sequence: u64,
    pub vulnerability_reattestation_verified_at: String,
    pub vulnerability_intelligence_snapshot_digest: String,
    pub vulnerability_intelligence_expires_at: String,
    pub security_receipt_id: String,
    pub security_receipt_digest: String,
    pub security_material_digest: String,
    pub sbom_digest: String,
    pub component_inventory_digest: String,
    pub component_count: u64,
    pub dependency_inventory_digest: String,
    pub sandbox_verifier_key_record_id: String,
    pub sandbox_verifier_key_record_digest: String,
    pub sandbox_verifier_key_id: String,
    pub sandbox_verifier_operator: String,
    pub sandbox_verifier_product: String,
    pub signature_algorithm: String,
    pub sandbox_policy_id: String,
    pub sequence: u64,
    pub predecessor_receipt_id: Option<String>,
    pub predecessor_receipt_digest: Option<String>,
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
pub(crate) struct ExternalPoolAdapterSandboxReattestationChallenge {
    pub schema: &'static str,
    pub canonicalization: &'static str,
    pub digest_algorithm: &'static str,
    pub signature_algorithm: &'static str,
    pub signature_message_base64: String,
    pub signature_message_digest: String,
    pub binding: ExternalPoolAdapterSandboxReattestationBinding,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSandboxReattestationMaterial {
    pub binding: ExternalPoolAdapterSandboxReattestationBinding,
    pub signature_message_digest: String,
    pub signature_base64: String,
    pub signature_digest: String,
    pub recorded_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub verified_at: String,
    pub recorded_at: String,
    pub evidence_scope: String,
    pub sandbox_reattestation_effect: String,
    pub adapter_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSandboxReattestationReceipt {
    pub schema: String,
    pub reattestation_receipt_id: String,
    pub reattestation_receipt_digest: String,
    pub reattestation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub reattestation: ExternalPoolAdapterSandboxReattestationMaterial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSandboxReattestationRevocationMaterial {
    pub reattestation_receipt_id: String,
    pub reattestation_receipt_digest: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub revoked_at: String,
    pub recorded_at: String,
    pub revocation_effect: String,
    pub adapter_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSandboxReattestationRevocationReceipt {
    pub schema: String,
    pub revocation_receipt_id: String,
    pub revocation_receipt_digest: String,
    pub revocation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub revocation: ExternalPoolAdapterSandboxReattestationRevocationMaterial,
}
