use serde::Serialize;

use crate::compute_federation::external_pool_adapter_artifact_signing_key::{
    ExternalPoolAdapterArtifactSigningKeyActivationReceipt,
    ExternalPoolAdapterArtifactSigningKeyRecord,
    ExternalPoolAdapterArtifactSigningKeyRevocationReceipt,
};

pub(super) const CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_artifact_signing_key_currentness.v1";

pub(crate) struct RegisterExternalPoolAdapterArtifactSigningKey {
    pub source_operator: String,
    pub key_id: String,
    pub public_key_pem: String,
    pub created_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(crate) struct ActivateExternalPoolAdapterArtifactSigningKey {
    pub key_record_id: String,
    pub expected_key_record_digest: String,
    pub activated_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(crate) struct RevokeExternalPoolAdapterArtifactSigningKey {
    pub key_record_id: String,
    pub expected_key_record_digest: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactSigningKeyRecordSummary {
    pub key_record_id: String,
    pub key_record_digest: String,
    pub registration_material_digest: String,
    pub source_operator: String,
    pub key_id: String,
    pub algorithm: String,
    pub created_by_admin_user_id: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactSigningKeyRegistrationWriteReceipt {
    pub key_record: ExternalPoolAdapterArtifactSigningKeyRecordSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactSigningKeyActivationSummary {
    pub activation_receipt_id: String,
    pub activation_receipt_digest: String,
    pub activation_material_digest: String,
    pub activated_by_admin_user_id: String,
    pub activated_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactSigningKeyActivationWriteReceipt {
    pub key_record: ExternalPoolAdapterArtifactSigningKeyRecordSummary,
    pub activation: ExternalPoolAdapterArtifactSigningKeyActivationSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactSigningKeyRevocationSummary {
    pub revocation_receipt_id: String,
    pub revocation_receipt_digest: String,
    pub revocation_material_digest: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub revoked_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactSigningKeyRevocationWriteReceipt {
    pub key_record: ExternalPoolAdapterArtifactSigningKeyRecordSummary,
    pub revocation: ExternalPoolAdapterArtifactSigningKeyRevocationSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterArtifactSigningKeyCurrentnessReceipt {
    pub schema: &'static str,
    pub key_record: ExternalPoolAdapterArtifactSigningKeyRecordSummary,
    pub current_status: String,
    pub activation: Option<ExternalPoolAdapterArtifactSigningKeyActivationSummary>,
    pub revocation: Option<ExternalPoolAdapterArtifactSigningKeyRevocationSummary>,
}

pub(super) struct StoredSigningKeyRecord {
    pub record: ExternalPoolAdapterArtifactSigningKeyRecord,
    pub record_json: String,
}

pub(super) struct StoredSigningKeyActivation {
    pub receipt: ExternalPoolAdapterArtifactSigningKeyActivationReceipt,
    pub receipt_json: String,
}

pub(super) struct StoredSigningKeyRevocation {
    pub receipt: ExternalPoolAdapterArtifactSigningKeyRevocationReceipt,
    pub receipt_json: String,
}

impl StoredSigningKeyRecord {
    pub(super) fn summary(&self) -> ExternalPoolAdapterArtifactSigningKeyRecordSummary {
        let registration = &self.record.registration;
        ExternalPoolAdapterArtifactSigningKeyRecordSummary {
            key_record_id: self.record.key_record_id.clone(),
            key_record_digest: self.record.key_record_digest.clone(),
            registration_material_digest: self.record.registration_material_digest.clone(),
            source_operator: registration.source_operator.clone(),
            key_id: registration.key_id.clone(),
            algorithm: registration.algorithm.clone(),
            created_by_admin_user_id: registration.created_by_admin_user_id.clone(),
            created_at: registration.created_at.clone(),
        }
    }
}

impl StoredSigningKeyActivation {
    pub(super) fn summary(&self) -> ExternalPoolAdapterArtifactSigningKeyActivationSummary {
        ExternalPoolAdapterArtifactSigningKeyActivationSummary {
            activation_receipt_id: self.receipt.activation_receipt_id.clone(),
            activation_receipt_digest: self.receipt.activation_receipt_digest.clone(),
            activation_material_digest: self.receipt.activation_material_digest.clone(),
            activated_by_admin_user_id: self.receipt.activation.activated_by_admin_user_id.clone(),
            activated_at: self.receipt.activation.occurred_at.clone(),
        }
    }
}

impl StoredSigningKeyRevocation {
    pub(super) fn summary(&self) -> ExternalPoolAdapterArtifactSigningKeyRevocationSummary {
        ExternalPoolAdapterArtifactSigningKeyRevocationSummary {
            revocation_receipt_id: self.receipt.revocation_receipt_id.clone(),
            revocation_receipt_digest: self.receipt.revocation_receipt_digest.clone(),
            revocation_material_digest: self.receipt.revocation_material_digest.clone(),
            revoked_by_admin_user_id: self.receipt.revocation.revoked_by_admin_user_id.clone(),
            reason: self.receipt.revocation.reason.clone(),
            revoked_at: self.receipt.revocation.occurred_at.clone(),
        }
    }
}

/// Non-serializable point-in-time proof for a future signed-provenance Store transaction.
pub(in crate::store) struct CurrentExternalPoolAdapterArtifactSigningKeyAuthority {
    key_record_id: String,
    key_record_digest: String,
    key_id: String,
    source_operator: String,
    public_key_pem: String,
}

impl CurrentExternalPoolAdapterArtifactSigningKeyAuthority {
    pub(super) fn new(record: &StoredSigningKeyRecord) -> Self {
        Self {
            key_record_id: record.record.key_record_id.clone(),
            key_record_digest: record.record.key_record_digest.clone(),
            key_id: record.record.registration.key_id.clone(),
            source_operator: record.record.registration.source_operator.clone(),
            public_key_pem: record.record.registration.public_key_pem.clone(),
        }
    }

    pub(in crate::store) fn key_record_id(&self) -> &str {
        &self.key_record_id
    }

    pub(in crate::store) fn key_record_digest(&self) -> &str {
        &self.key_record_digest
    }

    pub(in crate::store) fn key_id(&self) -> &str {
        &self.key_id
    }

    pub(in crate::store) fn source_operator(&self) -> &str {
        &self.source_operator
    }

    pub(in crate::store) fn public_key_pem(&self) -> &str {
        &self.public_key_pem
    }
}
