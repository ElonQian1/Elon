use serde::Serialize;

use crate::compute_federation::external_pool_adapter_credential_verifier::{
    ExternalPoolAdapterCredentialVerifierRecord,
    ExternalPoolAdapterCredentialVerifierTransitionReceipt,
};

pub(super) const CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_credential_verifier_currentness.v1";

pub(crate) struct RegisterExternalPoolAdapterCredentialVerifier {
    pub verifier_operator: String,
    pub verifier_product: String,
    pub verification_kind: String,
    pub verifier_id: String,
    pub verifier_revision: i64,
    pub verifier_digest: String,
    pub created_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(crate) struct ActivateExternalPoolAdapterCredentialVerifier {
    pub verifier_record_id: String,
    pub expected_verifier_record_digest: String,
    pub activated_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(crate) struct RevokeExternalPoolAdapterCredentialVerifier {
    pub verifier_record_id: String,
    pub expected_verifier_record_digest: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterCredentialVerifierRecordSummary {
    pub verifier_record_id: String,
    pub verifier_record_digest: String,
    pub registration_material_digest: String,
    pub verifier_operator: String,
    pub verifier_product: String,
    pub verification_kind: String,
    pub verifier_id: String,
    pub verifier_revision: i64,
    pub verifier_digest: String,
    pub created_by_admin_user_id: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterCredentialVerifierTransitionSummary {
    pub transition_receipt_id: String,
    pub transition_receipt_digest: String,
    pub transition_kind: String,
    pub actor_user_id: String,
    pub reason: Option<String>,
    pub occurred_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterCredentialVerifierRegistrationWriteReceipt {
    pub verifier_record: ExternalPoolAdapterCredentialVerifierRecordSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterCredentialVerifierTransitionWriteReceipt {
    pub verifier_record: ExternalPoolAdapterCredentialVerifierRecordSummary,
    pub transition: ExternalPoolAdapterCredentialVerifierTransitionSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterCredentialVerifierCurrentnessReceipt {
    pub schema: &'static str,
    pub verifier_record: ExternalPoolAdapterCredentialVerifierRecordSummary,
    pub current_status: String,
    pub activation: Option<ExternalPoolAdapterCredentialVerifierTransitionSummary>,
    pub revocation: Option<ExternalPoolAdapterCredentialVerifierTransitionSummary>,
}

pub(super) struct StoredVerifierRecord {
    pub record: ExternalPoolAdapterCredentialVerifierRecord,
    pub json: String,
}

pub(super) struct StoredTransition {
    pub receipt: ExternalPoolAdapterCredentialVerifierTransitionReceipt,
    pub json: String,
}

impl StoredVerifierRecord {
    pub(super) fn summary(&self) -> ExternalPoolAdapterCredentialVerifierRecordSummary {
        let item = &self.record.registration;
        ExternalPoolAdapterCredentialVerifierRecordSummary {
            verifier_record_id: self.record.verifier_record_id.clone(),
            verifier_record_digest: self.record.verifier_record_digest.clone(),
            registration_material_digest: self.record.registration_material_digest.clone(),
            verifier_operator: item.verifier_operator.clone(),
            verifier_product: item.verifier_product.clone(),
            verification_kind: item.verification_kind.clone(),
            verifier_id: item.verifier_id.clone(),
            verifier_revision: item.verifier_revision,
            verifier_digest: item.verifier_digest.clone(),
            created_by_admin_user_id: item.created_by_admin_user_id.clone(),
            created_at: item.created_at.clone(),
        }
    }
}

impl StoredTransition {
    pub(super) fn summary(&self) -> ExternalPoolAdapterCredentialVerifierTransitionSummary {
        let item = &self.receipt.transition;
        ExternalPoolAdapterCredentialVerifierTransitionSummary {
            transition_receipt_id: self.receipt.transition_receipt_id.clone(),
            transition_receipt_digest: self.receipt.transition_receipt_digest.clone(),
            transition_kind: if item.reason.is_some() {
                "revocation"
            } else {
                "activation"
            }
            .into(),
            actor_user_id: item.actor_user_id.clone(),
            reason: item.reason.clone(),
            occurred_at: item.occurred_at.clone(),
        }
    }
}
