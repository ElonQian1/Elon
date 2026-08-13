use serde::{Deserialize, Serialize};

use crate::compute_federation::external_pool_adapter_release::ComputeExternalPoolAdapterReleaseVerifierIntent;

pub(crate) const CREDENTIAL_REATTESTATION_BINDING_SCHEMA: &str =
    "compute_federation.external_pool_adapter_credential_reattestation_binding.v1";
pub(crate) const CREDENTIAL_REATTESTATION_CHALLENGE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_credential_reattestation_challenge.v1";
pub(crate) const CREDENTIAL_REATTESTATION_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_credential_reattestation_receipt.v1";
pub(crate) const CREDENTIAL_REATTESTATION_REVOCATION_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_credential_reattestation_revocation_receipt.v1";
pub(crate) const CREDENTIAL_REATTESTATION_CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_credential_reattestation_currentness.v1";
pub(crate) const CREDENTIAL_REATTESTATION_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const CREDENTIAL_REATTESTATION_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const CREDENTIAL_REATTESTATION_SIGNATURE_ALGORITHM: &str = "rsa-pkcs1v15-sha256";
pub(crate) const CREDENTIAL_REATTESTATION_POLICY_ID: &str =
    "external_pool_non_bearer_credential_renewable_signed_challenge_v2";
pub(crate) const CREDENTIAL_REATTESTATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_credential_reattestation";
pub(crate) const CREDENTIAL_REATTESTATION_REVOCATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_credential_reattestation_revocation";
pub(crate) const CREDENTIAL_REATTESTATION_EVIDENCE_SCOPE: &str =
    "verifier_signature_over_exact_v249_provider_binding_stable_credential_subject_and_single_use_nonce";
pub(crate) const CREDENTIAL_REATTESTATION_EFFECT: &str =
    "signed_provider_credential_reattestation_verified_current";
pub(crate) const CREDENTIAL_REATTESTATION_REVOCATION_EFFECT: &str =
    "credential_reattestation_revoked";
pub(crate) const CREDENTIAL_REATTESTATION_NO_EFFECT: &str = "none";
pub(crate) const CREDENTIAL_REATTESTATION_CHALLENGE_VALIDITY_MINUTES: i64 = 5;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCredentialReattestationBinding {
    pub schema: String,
    pub challenge_id: String,
    pub challenge_nonce_base64: String,
    pub challenge_nonce_digest: String,
    pub challenge_issued_at: String,
    pub challenge_expires_at: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub provider_binding_material_digest: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub registry_release_material_digest: String,
    pub route_adapter_projection_id: String,
    pub installation_receipt_id: String,
    pub installation_receipt_digest: String,
    pub installation_content_digest: String,
    pub application_id: String,
    pub application_digest: String,
    pub adoption_receipt_id: String,
    pub adoption_receipt_digest: String,
    pub provider_id: String,
    pub provider_kind: String,
    pub provider_owner_account_id: String,
    pub observed_settlement_account_id: String,
    pub observed_provider_policy_revision: i64,
    pub observed_provider_digest: String,
    pub observed_provider_status: String,
    pub adapter_id: String,
    pub release_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
    pub admission_id: String,
    pub admission_digest: String,
    pub legacy_credential_verification_receipt_id: String,
    pub legacy_credential_verification_receipt_digest: String,
    pub credential_ref_scheme: String,
    pub credential_locator_commitment: String,
    pub expected_credential_verifier: ComputeExternalPoolAdapterReleaseVerifierIntent,
    pub credential_verifier_digest: String,
    pub credential_verifier_key_record_id: String,
    pub credential_verifier_key_record_digest: String,
    pub credential_verifier_key_id: String,
    pub credential_verifier_record_id: String,
    pub credential_verifier_record_digest: String,
    pub signature_algorithm: String,
    pub verification_policy_id: String,
    pub sequence: u64,
    pub predecessor_receipt_id: Option<String>,
    pub predecessor_receipt_digest: Option<String>,
    pub verifier_report_id: String,
    pub verification_started_at: String,
    pub verification_completed_at: String,
    pub report_generated_at: String,
    pub report_expires_at: String,
    pub credential_resolution_outcome: String,
    pub provider_authentication_outcome: String,
    pub provider_response_evidence_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterCredentialReattestationChallenge {
    pub schema: &'static str,
    pub canonicalization: &'static str,
    pub digest_algorithm: &'static str,
    pub signature_algorithm: &'static str,
    pub signature_message_base64: String,
    pub signature_message_digest: String,
    pub binding: ExternalPoolAdapterCredentialReattestationBinding,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCredentialReattestationMaterial {
    pub binding: ExternalPoolAdapterCredentialReattestationBinding,
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
    pub credential_reattestation_effect: String,
    pub adapter_effect: String,
    pub provider_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCredentialReattestationReceipt {
    pub schema: String,
    pub reattestation_receipt_id: String,
    pub reattestation_receipt_digest: String,
    pub reattestation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub reattestation: ExternalPoolAdapterCredentialReattestationMaterial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCredentialReattestationRevocationMaterial {
    pub reattestation_receipt_id: String,
    pub reattestation_receipt_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
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
    pub route_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCredentialReattestationRevocationReceipt {
    pub schema: String,
    pub revocation_receipt_id: String,
    pub revocation_receipt_digest: String,
    pub revocation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub revocation: ExternalPoolAdapterCredentialReattestationRevocationMaterial,
}
