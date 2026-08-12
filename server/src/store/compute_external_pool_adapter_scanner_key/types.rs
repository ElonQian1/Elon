use serde::Serialize;

use crate::compute_federation::external_pool_adapter_scanner_key::{
    ExternalPoolAdapterScannerKeyActivationReceipt, ExternalPoolAdapterScannerKeyRecord,
    ExternalPoolAdapterScannerKeyRevocationReceipt,
};

pub(super) const CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_scanner_key_currentness.v1";

pub(crate) struct RegisterExternalPoolAdapterScannerKey {
    pub scanner_operator: String,
    pub scanner_product: String,
    pub key_id: String,
    pub public_key_pem: String,
    pub created_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(crate) struct ActivateExternalPoolAdapterScannerKey {
    pub key_record_id: String,
    pub expected_key_record_digest: String,
    pub activated_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(crate) struct RevokeExternalPoolAdapterScannerKey {
    pub key_record_id: String,
    pub expected_key_record_digest: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterScannerKeyRecordSummary {
    pub key_record_id: String,
    pub key_record_digest: String,
    pub registration_material_digest: String,
    pub scanner_operator: String,
    pub scanner_product: String,
    pub key_id: String,
    pub algorithm: String,
    pub created_by_admin_user_id: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterScannerKeyRegistrationWriteReceipt {
    pub key_record: ExternalPoolAdapterScannerKeyRecordSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterScannerKeyActivationSummary {
    pub activation_receipt_id: String,
    pub activation_receipt_digest: String,
    pub activation_material_digest: String,
    pub activated_by_admin_user_id: String,
    pub activated_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterScannerKeyActivationWriteReceipt {
    pub key_record: ExternalPoolAdapterScannerKeyRecordSummary,
    pub activation: ExternalPoolAdapterScannerKeyActivationSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterScannerKeyRevocationSummary {
    pub revocation_receipt_id: String,
    pub revocation_receipt_digest: String,
    pub revocation_material_digest: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub revoked_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterScannerKeyRevocationWriteReceipt {
    pub key_record: ExternalPoolAdapterScannerKeyRecordSummary,
    pub revocation: ExternalPoolAdapterScannerKeyRevocationSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterScannerKeyCurrentnessReceipt {
    pub schema: &'static str,
    pub key_record: ExternalPoolAdapterScannerKeyRecordSummary,
    pub current_status: String,
    pub activation: Option<ExternalPoolAdapterScannerKeyActivationSummary>,
    pub revocation: Option<ExternalPoolAdapterScannerKeyRevocationSummary>,
}

pub(super) struct StoredScannerKeyRecord {
    pub record: ExternalPoolAdapterScannerKeyRecord,
    pub json: String,
}

pub(super) struct StoredScannerKeyActivation {
    pub receipt: ExternalPoolAdapterScannerKeyActivationReceipt,
    pub json: String,
}

pub(super) struct StoredScannerKeyRevocation {
    pub receipt: ExternalPoolAdapterScannerKeyRevocationReceipt,
    pub json: String,
}

/// Non-serializable current trust root for the future signed report transaction.
pub(in crate::store) struct CurrentExternalPoolAdapterScannerKeyAuthority {
    key_record_id: String,
    key_record_digest: String,
    key_id: String,
    scanner_operator: String,
    scanner_product: String,
    public_key_pem: String,
}

/// Exact historical root retained for report readback after key revocation.
pub(in crate::store) struct ExternalPoolAdapterScannerKeyRecordAuthority {
    key_record_id: String,
    key_record_digest: String,
    key_id: String,
    scanner_operator: String,
    scanner_product: String,
    public_key_pem: String,
}

macro_rules! authority_impl {
    ($name:ident) => {
        impl $name {
            pub(super) fn new(root: &StoredScannerKeyRecord) -> Self {
                let item = &root.record.registration;
                Self {
                    key_record_id: root.record.key_record_id.clone(),
                    key_record_digest: root.record.key_record_digest.clone(),
                    key_id: item.key_id.clone(),
                    scanner_operator: item.scanner_operator.clone(),
                    scanner_product: item.scanner_product.clone(),
                    public_key_pem: item.public_key_pem.clone(),
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
            pub(in crate::store) fn scanner_operator(&self) -> &str {
                &self.scanner_operator
            }
            pub(in crate::store) fn scanner_product(&self) -> &str {
                &self.scanner_product
            }
            pub(in crate::store) fn public_key_pem(&self) -> &str {
                &self.public_key_pem
            }
        }
    };
}

authority_impl!(CurrentExternalPoolAdapterScannerKeyAuthority);
authority_impl!(ExternalPoolAdapterScannerKeyRecordAuthority);

impl StoredScannerKeyRecord {
    pub(super) fn summary(&self) -> ExternalPoolAdapterScannerKeyRecordSummary {
        let item = &self.record.registration;
        ExternalPoolAdapterScannerKeyRecordSummary {
            key_record_id: self.record.key_record_id.clone(),
            key_record_digest: self.record.key_record_digest.clone(),
            registration_material_digest: self.record.registration_material_digest.clone(),
            scanner_operator: item.scanner_operator.clone(),
            scanner_product: item.scanner_product.clone(),
            key_id: item.key_id.clone(),
            algorithm: item.algorithm.clone(),
            created_by_admin_user_id: item.created_by_admin_user_id.clone(),
            created_at: item.created_at.clone(),
        }
    }
}

impl StoredScannerKeyActivation {
    pub(super) fn summary(&self) -> ExternalPoolAdapterScannerKeyActivationSummary {
        ExternalPoolAdapterScannerKeyActivationSummary {
            activation_receipt_id: self.receipt.activation_receipt_id.clone(),
            activation_receipt_digest: self.receipt.activation_receipt_digest.clone(),
            activation_material_digest: self.receipt.activation_material_digest.clone(),
            activated_by_admin_user_id: self.receipt.activation.activated_by_admin_user_id.clone(),
            activated_at: self.receipt.activation.occurred_at.clone(),
        }
    }
}

impl StoredScannerKeyRevocation {
    pub(super) fn summary(&self) -> ExternalPoolAdapterScannerKeyRevocationSummary {
        ExternalPoolAdapterScannerKeyRevocationSummary {
            revocation_receipt_id: self.receipt.revocation_receipt_id.clone(),
            revocation_receipt_digest: self.receipt.revocation_receipt_digest.clone(),
            revocation_material_digest: self.receipt.revocation_material_digest.clone(),
            revoked_by_admin_user_id: self.receipt.revocation.revoked_by_admin_user_id.clone(),
            reason: self.receipt.revocation.reason.clone(),
            revoked_at: self.receipt.revocation.occurred_at.clone(),
        }
    }
}
