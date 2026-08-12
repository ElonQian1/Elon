use serde::{Deserialize, Serialize};

use crate::compute_federation::external_pool_adapter_release::ComputeExternalPoolAdapterReleaseVerifierIntent;

pub(crate) const CREDENTIAL_VERIFICATION_BINDING_SCHEMA: &str =
    "compute_federation.external_pool_adapter_credential_verification_binding.v1";
pub(crate) const CREDENTIAL_VERIFICATION_CHALLENGE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_credential_verification_challenge.v1";
pub(crate) const CREDENTIAL_VERIFICATION_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_credential_verification_receipt.v1";
pub(crate) const CREDENTIAL_VERIFICATION_CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_credential_verification_currentness.v1";
pub(crate) const CREDENTIAL_VERIFICATION_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const CREDENTIAL_VERIFICATION_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const CREDENTIAL_VERIFICATION_SIGNATURE_ALGORITHM: &str = "rsa-pkcs1v15-sha256";
pub(crate) const CREDENTIAL_VERIFICATION_POLICY_ID: &str =
    "external_pool_non_bearer_credential_signed_challenge_v1";
pub(crate) const CREDENTIAL_VERIFICATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_credential_verification";
pub(crate) const CREDENTIAL_VERIFICATION_EVIDENCE_SCOPE: &str =
    "verifier_signature_over_exact_v221_non_bearer_locator_commitment_v222_admission_and_asserted_authentication";
pub(crate) const CREDENTIAL_VERIFICATION_EFFECT: &str = "signed_credential_verification_current";
pub(crate) const CREDENTIAL_VERIFICATION_NO_EFFECT: &str = "none";
pub(crate) const MAX_CREDENTIAL_VERIFICATION_VALIDITY_MINUTES: i64 = 60;
pub(crate) const MAX_CREDENTIAL_VERIFICATION_RUN_MINUTES: i64 = 10;
pub(crate) const MAX_CREDENTIAL_VERIFICATION_REPORT_DELAY_MINUTES: i64 = 5;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCredentialVerificationDraft {
    pub verifier_report_id: String,
    pub verification_started_at: String,
    pub verification_completed_at: String,
    pub report_generated_at: String,
    pub report_expires_at: String,
    pub credential_resolution_outcome: String,
    pub provider_authentication_outcome: String,
    pub provider_response_evidence_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCredentialVerificationBinding {
    pub schema: String,
    pub application_id: String,
    pub application_digest: String,
    pub onboarding_applied_at: String,
    pub provider_id: String,
    pub provider_kind: String,
    pub provider_owner_account_id: String,
    pub settlement_account_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub provider_status: String,
    pub adapter_id: String,
    pub adapter_release_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
    pub credential_ref_scheme: String,
    pub credential_locator_commitment: String,
    pub admission_id: String,
    pub admission_digest: String,
    pub admission_applied_at: String,
    pub declared_implementation_sha256: String,
    pub capability_set_digest: String,
    pub expected_credential_verifier: ComputeExternalPoolAdapterReleaseVerifierIntent,
    pub credential_verifier_key_record_id: String,
    pub credential_verifier_key_record_digest: String,
    pub credential_verifier_key_id: String,
    pub credential_verifier_record_id: String,
    pub credential_verifier_record_digest: String,
    pub signature_algorithm: String,
    pub verification_policy_id: String,
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
pub(crate) struct ExternalPoolAdapterCredentialVerificationChallenge {
    pub schema: &'static str,
    pub canonicalization: &'static str,
    pub digest_algorithm: &'static str,
    pub signature_algorithm: &'static str,
    pub signature_message_base64: String,
    pub signature_message_digest: String,
    pub binding: ExternalPoolAdapterCredentialVerificationBinding,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCredentialVerificationMaterial {
    pub binding: ExternalPoolAdapterCredentialVerificationBinding,
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
    pub credential_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCredentialVerificationReceipt {
    pub schema: String,
    pub credential_verification_receipt_id: String,
    pub credential_verification_receipt_digest: String,
    pub verification_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub verification: ExternalPoolAdapterCredentialVerificationMaterial,
}
