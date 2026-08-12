use serde::Serialize;

use crate::compute_federation::external_pool_adapter_credential_verification::{
    ExternalPoolAdapterCredentialVerificationDraft,
    ExternalPoolAdapterCredentialVerificationReceipt,
};

#[derive(Clone)]
pub(crate) struct GetExternalPoolAdapterCredentialVerificationChallenge {
    pub application_id: String,
    pub expected_application_digest: String,
    pub admission_id: String,
    pub expected_admission_digest: String,
    pub credential_verifier_key_record_id: String,
    pub expected_credential_verifier_key_record_digest: String,
    pub expected_credential_verifier_key_id: String,
    pub draft: ExternalPoolAdapterCredentialVerificationDraft,
}

pub(crate) struct CreateExternalPoolAdapterCredentialVerification {
    pub challenge: GetExternalPoolAdapterCredentialVerificationChallenge,
    pub expected_signature_message_digest: String,
    pub signature_base64: String,
    pub recorded_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterCredentialVerificationSummary {
    pub credential_verification_receipt_id: String,
    pub credential_verification_receipt_digest: String,
    pub verification_material_digest: String,
    pub application_id: String,
    pub application_digest: String,
    pub provider_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub adapter_id: String,
    pub adapter_release_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
    pub credential_ref_scheme: String,
    pub credential_locator_commitment: String,
    pub admission_id: String,
    pub admission_digest: String,
    pub credential_verifier_key_record_id: String,
    pub credential_verifier_key_record_digest: String,
    pub credential_verifier_key_id: String,
    pub credential_verifier_record_id: String,
    pub credential_verifier_record_digest: String,
    pub verifier_report_id: String,
    pub report_expires_at: String,
    pub provider_response_evidence_digest: String,
    pub signature_message_digest: String,
    pub signature_digest: String,
    pub recorded_by_admin_user_id: String,
    pub verified_at: String,
    pub evidence_scope: String,
    pub credential_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterCredentialVerificationWriteReceipt {
    pub credential_verification: ExternalPoolAdapterCredentialVerificationSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterCredentialVerificationCurrentness {
    pub schema: &'static str,
    pub credential_verification: ExternalPoolAdapterCredentialVerificationSummary,
    pub current_status: String,
    pub onboarding_status: String,
    pub provider_status: String,
    pub admission_status: String,
    pub verifier_key_status: String,
    pub report_validity_status: String,
}

pub(super) struct StoredExternalPoolAdapterCredentialVerification {
    pub receipt: ExternalPoolAdapterCredentialVerificationReceipt,
    pub receipt_json: String,
}

pub(in crate::store) struct CurrentExternalPoolAdapterCredentialVerificationAuthority {
    receipt: ExternalPoolAdapterCredentialVerificationReceipt,
}

impl CurrentExternalPoolAdapterCredentialVerificationAuthority {
    pub(super) fn new(receipt: ExternalPoolAdapterCredentialVerificationReceipt) -> Self {
        Self { receipt }
    }
    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterCredentialVerificationReceipt {
        &self.receipt
    }
}

impl StoredExternalPoolAdapterCredentialVerification {
    pub(super) fn summary(&self) -> ExternalPoolAdapterCredentialVerificationSummary {
        let receipt = &self.receipt;
        let item = &receipt.verification;
        let binding = &item.binding;
        ExternalPoolAdapterCredentialVerificationSummary {
            credential_verification_receipt_id: receipt.credential_verification_receipt_id.clone(),
            credential_verification_receipt_digest: receipt
                .credential_verification_receipt_digest
                .clone(),
            verification_material_digest: receipt.verification_material_digest.clone(),
            application_id: binding.application_id.clone(),
            application_digest: binding.application_digest.clone(),
            provider_id: binding.provider_id.clone(),
            provider_policy_revision: binding.provider_policy_revision,
            provider_digest: binding.provider_digest.clone(),
            adapter_id: binding.adapter_id.clone(),
            adapter_release_version: binding.adapter_release_version.clone(),
            adapter_config_revision: binding.adapter_config_revision,
            adapter_config_digest: binding.adapter_config_digest.clone(),
            credential_ref_scheme: binding.credential_ref_scheme.clone(),
            credential_locator_commitment: binding.credential_locator_commitment.clone(),
            admission_id: binding.admission_id.clone(),
            admission_digest: binding.admission_digest.clone(),
            credential_verifier_key_record_id: binding.credential_verifier_key_record_id.clone(),
            credential_verifier_key_record_digest: binding
                .credential_verifier_key_record_digest
                .clone(),
            credential_verifier_key_id: binding.credential_verifier_key_id.clone(),
            credential_verifier_record_id: binding.credential_verifier_record_id.clone(),
            credential_verifier_record_digest: binding.credential_verifier_record_digest.clone(),
            verifier_report_id: binding.verifier_report_id.clone(),
            report_expires_at: binding.report_expires_at.clone(),
            provider_response_evidence_digest: binding.provider_response_evidence_digest.clone(),
            signature_message_digest: item.signature_message_digest.clone(),
            signature_digest: item.signature_digest.clone(),
            recorded_by_admin_user_id: item.recorded_by_admin_user_id.clone(),
            verified_at: item.verified_at.clone(),
            evidence_scope: item.evidence_scope.clone(),
            credential_effect: item.credential_effect.clone(),
            adapter_effect: item.adapter_effect.clone(),
            route_effect: item.route_effect.clone(),
            execution_effect: item.execution_effect.clone(),
            settlement_effect: item.settlement_effect.clone(),
        }
    }
}

pub(super) fn write_receipt(
    stored: &StoredExternalPoolAdapterCredentialVerification,
    replayed: bool,
) -> ExternalPoolAdapterCredentialVerificationWriteReceipt {
    ExternalPoolAdapterCredentialVerificationWriteReceipt {
        credential_verification: stored.summary(),
        replayed,
    }
}
