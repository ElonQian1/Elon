use serde::Serialize;

use crate::compute_federation::{
    external_pool_adapter_credential_reattestation::{
        ExternalPoolAdapterCredentialReattestationReceipt,
        ExternalPoolAdapterCredentialReattestationRevocationReceipt,
    },
    external_pool_adapter_credential_verification::ExternalPoolAdapterCredentialVerificationDraft,
};

#[derive(Clone)]
pub(crate) struct GetExternalPoolAdapterCredentialReattestationChallenge {
    pub provider_binding_id: String,
    pub expected_provider_binding_digest: String,
    pub expected_registry_release_digest: String,
    pub credential_verifier_key_record_id: String,
    pub expected_credential_verifier_key_record_digest: String,
    pub expected_credential_verifier_key_id: String,
    pub draft: ExternalPoolAdapterCredentialVerificationDraft,
}

pub(crate) struct CreateExternalPoolAdapterCredentialReattestation {
    pub challenge_id: String,
    pub expected_signature_message_digest: String,
    pub signature_base64: String,
    pub recorded_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(crate) struct RevokeExternalPoolAdapterCredentialReattestation {
    pub reattestation_receipt_id: String,
    pub expected_reattestation_receipt_digest: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterCredentialReattestationSummary {
    pub reattestation_receipt_id: String,
    pub reattestation_receipt_digest: String,
    pub reattestation_material_digest: String,
    pub challenge_id: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub route_adapter_projection_id: String,
    pub installation_receipt_id: String,
    pub installation_receipt_digest: String,
    pub provider_id: String,
    pub provider_kind: String,
    pub provider_owner_account_id: String,
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
    pub credential_verifier_key_record_id: String,
    pub credential_verifier_key_record_digest: String,
    pub credential_verifier_key_id: String,
    pub sequence: u64,
    pub predecessor_receipt_id: Option<String>,
    pub predecessor_receipt_digest: Option<String>,
    pub verifier_report_id: String,
    pub verification_started_at: String,
    pub verification_completed_at: String,
    pub report_generated_at: String,
    pub report_expires_at: String,
    pub verified_at: String,
    pub evidence_scope: String,
    pub credential_reattestation_effect: String,
    pub adapter_effect: String,
    pub provider_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterCredentialReattestationRevocationSummary {
    pub revocation_receipt_id: String,
    pub revocation_receipt_digest: String,
    pub revocation_material_digest: String,
    pub reattestation_receipt_id: String,
    pub reattestation_receipt_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub reason: String,
    pub revoked_at: String,
    pub revocation_effect: String,
    pub adapter_effect: String,
    pub provider_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterCredentialReattestationWriteReceipt {
    pub reattestation: ExternalPoolAdapterCredentialReattestationSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterCredentialReattestationRevocationWriteReceipt {
    pub reattestation: ExternalPoolAdapterCredentialReattestationSummary,
    pub revocation: ExternalPoolAdapterCredentialReattestationRevocationSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterCredentialReattestationCurrentness {
    pub schema: &'static str,
    pub reattestation: ExternalPoolAdapterCredentialReattestationSummary,
    pub revocation: Option<ExternalPoolAdapterCredentialReattestationRevocationSummary>,
    pub current_status: String,
    pub head_status: String,
    pub provider_binding_status: String,
    pub registry_release_status: String,
    pub provider_subject_status: String,
    pub provider_revision_status: String,
    pub credential_verifier_key_status: String,
    pub report_validity_status: String,
    pub revocation_status: String,
}

pub(super) struct StoredCredentialReattestation {
    pub receipt: ExternalPoolAdapterCredentialReattestationReceipt,
    pub receipt_json: String,
}

pub(super) struct StoredCredentialReattestationRevocation {
    pub receipt: ExternalPoolAdapterCredentialReattestationRevocationReceipt,
    pub receipt_json: String,
}

pub(in crate::store) struct HistoricalExternalPoolAdapterCredentialReattestationAuthority {
    receipt: ExternalPoolAdapterCredentialReattestationReceipt,
}

pub(in crate::store) struct CurrentExternalPoolAdapterCredentialReattestationAuthority {
    receipt: ExternalPoolAdapterCredentialReattestationReceipt,
    checked_at: String,
}

impl HistoricalExternalPoolAdapterCredentialReattestationAuthority {
    pub(super) fn new(receipt: ExternalPoolAdapterCredentialReattestationReceipt) -> Self {
        Self { receipt }
    }
    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterCredentialReattestationReceipt {
        &self.receipt
    }
}

impl CurrentExternalPoolAdapterCredentialReattestationAuthority {
    pub(super) fn new(
        receipt: ExternalPoolAdapterCredentialReattestationReceipt,
        checked_at: String,
    ) -> Self {
        Self {
            receipt,
            checked_at,
        }
    }
    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterCredentialReattestationReceipt {
        &self.receipt
    }
    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}

impl StoredCredentialReattestation {
    pub(super) fn summary(&self) -> ExternalPoolAdapterCredentialReattestationSummary {
        let receipt = &self.receipt;
        let item = &receipt.reattestation;
        let binding = &item.binding;
        ExternalPoolAdapterCredentialReattestationSummary {
            reattestation_receipt_id: receipt.reattestation_receipt_id.clone(),
            reattestation_receipt_digest: receipt.reattestation_receipt_digest.clone(),
            reattestation_material_digest: receipt.reattestation_material_digest.clone(),
            challenge_id: binding.challenge_id.clone(),
            provider_binding_id: binding.provider_binding_id.clone(),
            provider_binding_digest: binding.provider_binding_digest.clone(),
            registry_release_id: binding.registry_release_id.clone(),
            registry_release_digest: binding.registry_release_digest.clone(),
            route_adapter_projection_id: binding.route_adapter_projection_id.clone(),
            installation_receipt_id: binding.installation_receipt_id.clone(),
            installation_receipt_digest: binding.installation_receipt_digest.clone(),
            provider_id: binding.provider_id.clone(),
            provider_kind: binding.provider_kind.clone(),
            provider_owner_account_id: binding.provider_owner_account_id.clone(),
            observed_provider_policy_revision: binding.observed_provider_policy_revision,
            observed_provider_digest: binding.observed_provider_digest.clone(),
            observed_provider_status: binding.observed_provider_status.clone(),
            adapter_id: binding.adapter_id.clone(),
            release_version: binding.release_version.clone(),
            adapter_config_revision: binding.adapter_config_revision,
            adapter_config_digest: binding.adapter_config_digest.clone(),
            admission_id: binding.admission_id.clone(),
            admission_digest: binding.admission_digest.clone(),
            legacy_credential_verification_receipt_id: binding
                .legacy_credential_verification_receipt_id
                .clone(),
            legacy_credential_verification_receipt_digest: binding
                .legacy_credential_verification_receipt_digest
                .clone(),
            credential_verifier_key_record_id: binding.credential_verifier_key_record_id.clone(),
            credential_verifier_key_record_digest: binding
                .credential_verifier_key_record_digest
                .clone(),
            credential_verifier_key_id: binding.credential_verifier_key_id.clone(),
            sequence: binding.sequence,
            predecessor_receipt_id: binding.predecessor_receipt_id.clone(),
            predecessor_receipt_digest: binding.predecessor_receipt_digest.clone(),
            verifier_report_id: binding.verifier_report_id.clone(),
            verification_started_at: binding.verification_started_at.clone(),
            verification_completed_at: binding.verification_completed_at.clone(),
            report_generated_at: binding.report_generated_at.clone(),
            report_expires_at: binding.report_expires_at.clone(),
            verified_at: item.verified_at.clone(),
            evidence_scope: item.evidence_scope.clone(),
            credential_reattestation_effect: item.credential_reattestation_effect.clone(),
            adapter_effect: item.adapter_effect.clone(),
            provider_effect: item.provider_effect.clone(),
            route_effect: item.route_effect.clone(),
            execution_effect: item.execution_effect.clone(),
            usage_effect: item.usage_effect.clone(),
            settlement_effect: item.settlement_effect.clone(),
        }
    }
}

impl StoredCredentialReattestationRevocation {
    pub(super) fn summary(&self) -> ExternalPoolAdapterCredentialReattestationRevocationSummary {
        let receipt = &self.receipt;
        let item = &receipt.revocation;
        ExternalPoolAdapterCredentialReattestationRevocationSummary {
            revocation_receipt_id: receipt.revocation_receipt_id.clone(),
            revocation_receipt_digest: receipt.revocation_receipt_digest.clone(),
            revocation_material_digest: receipt.revocation_material_digest.clone(),
            reattestation_receipt_id: item.reattestation_receipt_id.clone(),
            reattestation_receipt_digest: item.reattestation_receipt_digest.clone(),
            provider_binding_id: item.provider_binding_id.clone(),
            provider_binding_digest: item.provider_binding_digest.clone(),
            reason: item.reason.clone(),
            revoked_at: item.revoked_at.clone(),
            revocation_effect: item.revocation_effect.clone(),
            adapter_effect: item.adapter_effect.clone(),
            provider_effect: item.provider_effect.clone(),
            route_effect: item.route_effect.clone(),
            execution_effect: item.execution_effect.clone(),
            usage_effect: item.usage_effect.clone(),
            settlement_effect: item.settlement_effect.clone(),
        }
    }
}
