use serde::Serialize;

use crate::compute_federation::external_pool_adapter_credential_verifier_key::{
    CredentialVerifierKeyRecord, CredentialVerifierKeyRevocationReceipt,
};

pub(super) const CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_credential_verifier_key_currentness.v1";

pub(crate) struct RegisterCredentialVerifierKey {
    pub verifier_record_id: String,
    pub expected_verifier_record_digest: String,
    pub verification_kind: String,
    pub verifier_id: String,
    pub verifier_revision: i64,
    pub expected_verifier_digest: String,
    pub key_id: String,
    pub public_key_pem: String,
    pub created_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(crate) struct RevokeCredentialVerifierKey {
    pub key_record_id: String,
    pub expected_key_record_digest: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct CredentialVerifierKeyRecordSummary {
    pub key_record_id: String,
    pub key_record_digest: String,
    pub registration_material_digest: String,
    pub verifier_record_id: String,
    pub verifier_record_digest: String,
    pub verifier_operator: String,
    pub verifier_product: String,
    pub verification_kind: String,
    pub verifier_id: String,
    pub verifier_revision: i64,
    pub verifier_digest: String,
    pub key_id: String,
    pub algorithm: String,
    pub created_by_admin_user_id: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct CredentialVerifierKeyRevocationSummary {
    pub revocation_receipt_id: String,
    pub revocation_receipt_digest: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub revoked_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct CredentialVerifierKeyRegistrationWriteReceipt {
    pub key_record: CredentialVerifierKeyRecordSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct CredentialVerifierKeyRevocationWriteReceipt {
    pub key_record: CredentialVerifierKeyRecordSummary,
    pub revocation: CredentialVerifierKeyRevocationSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct CredentialVerifierKeyCurrentnessReceipt {
    pub schema: &'static str,
    pub key_record: CredentialVerifierKeyRecordSummary,
    pub current_status: String,
    pub revocation: Option<CredentialVerifierKeyRevocationSummary>,
}

pub(super) struct StoredKeyRecord {
    pub record: CredentialVerifierKeyRecord,
    pub json: String,
}

pub(super) struct StoredRevocation {
    pub receipt: CredentialVerifierKeyRevocationReceipt,
    pub json: String,
}

pub(in crate::store) struct CurrentCredentialVerifierKeyAuthority {
    key_record_id: String,
    key_record_digest: String,
    verifier_record_id: String,
    verifier_record_digest: String,
    verification_kind: String,
    verifier_id: String,
    verifier_revision: i64,
    verifier_digest: String,
    key_id: String,
    public_key_pem: String,
}

impl CurrentCredentialVerifierKeyAuthority {
    pub(super) fn new(root: &StoredKeyRecord) -> Self {
        let item = &root.record.registration;
        Self {
            key_record_id: root.record.key_record_id.clone(),
            key_record_digest: root.record.key_record_digest.clone(),
            verifier_record_id: item.verifier_record_id.clone(),
            verifier_record_digest: item.verifier_record_digest.clone(),
            verification_kind: item.verification_kind.clone(),
            verifier_id: item.verifier_id.clone(),
            verifier_revision: item.verifier_revision,
            verifier_digest: item.verifier_digest.clone(),
            key_id: item.key_id.clone(),
            public_key_pem: item.public_key_pem.clone(),
        }
    }

    pub(in crate::store) fn key_record_id(&self) -> &str {
        &self.key_record_id
    }
    pub(in crate::store) fn key_record_digest(&self) -> &str {
        &self.key_record_digest
    }
    pub(in crate::store) fn verifier_record_id(&self) -> &str {
        &self.verifier_record_id
    }
    pub(in crate::store) fn verifier_record_digest(&self) -> &str {
        &self.verifier_record_digest
    }
    pub(in crate::store) fn verification_kind(&self) -> &str {
        &self.verification_kind
    }
    pub(in crate::store) fn verifier_id(&self) -> &str {
        &self.verifier_id
    }
    pub(in crate::store) fn verifier_revision(&self) -> i64 {
        self.verifier_revision
    }
    pub(in crate::store) fn verifier_digest(&self) -> &str {
        &self.verifier_digest
    }
    pub(in crate::store) fn key_id(&self) -> &str {
        &self.key_id
    }
    pub(in crate::store) fn public_key_pem(&self) -> &str {
        &self.public_key_pem
    }
}

impl StoredKeyRecord {
    pub(super) fn summary(&self) -> CredentialVerifierKeyRecordSummary {
        let item = &self.record.registration;
        CredentialVerifierKeyRecordSummary {
            key_record_id: self.record.key_record_id.clone(),
            key_record_digest: self.record.key_record_digest.clone(),
            registration_material_digest: self.record.registration_material_digest.clone(),
            verifier_record_id: item.verifier_record_id.clone(),
            verifier_record_digest: item.verifier_record_digest.clone(),
            verifier_operator: item.verifier_operator.clone(),
            verifier_product: item.verifier_product.clone(),
            verification_kind: item.verification_kind.clone(),
            verifier_id: item.verifier_id.clone(),
            verifier_revision: item.verifier_revision,
            verifier_digest: item.verifier_digest.clone(),
            key_id: item.key_id.clone(),
            algorithm: item.algorithm.clone(),
            created_by_admin_user_id: item.created_by_admin_user_id.clone(),
            created_at: item.created_at.clone(),
        }
    }
}

impl StoredRevocation {
    pub(super) fn summary(&self) -> CredentialVerifierKeyRevocationSummary {
        let item = &self.receipt.revocation;
        CredentialVerifierKeyRevocationSummary {
            revocation_receipt_id: self.receipt.revocation_receipt_id.clone(),
            revocation_receipt_digest: self.receipt.revocation_receipt_digest.clone(),
            revoked_by_admin_user_id: item.revoked_by_admin_user_id.clone(),
            reason: item.reason.clone(),
            revoked_at: item.revoked_at.clone(),
        }
    }
}
