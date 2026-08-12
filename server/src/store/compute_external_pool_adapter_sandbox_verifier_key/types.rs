use serde::Serialize;

use crate::compute_federation::external_pool_adapter_sandbox_verifier_key::{
    ExternalPoolAdapterSandboxVerifierKeyRecord,
    ExternalPoolAdapterSandboxVerifierKeyTransitionReceipt,
};

pub(super) const CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_sandbox_verifier_key_currentness.v1";

pub(crate) struct RegisterExternalPoolAdapterSandboxVerifierKey {
    pub verifier_operator: String,
    pub verifier_product: String,
    pub key_id: String,
    pub public_key_pem: String,
    pub created_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(crate) struct ActivateExternalPoolAdapterSandboxVerifierKey {
    pub key_record_id: String,
    pub expected_key_record_digest: String,
    pub activated_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

pub(crate) struct RevokeExternalPoolAdapterSandboxVerifierKey {
    pub key_record_id: String,
    pub expected_key_record_digest: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSandboxVerifierKeyRecordSummary {
    pub key_record_id: String,
    pub key_record_digest: String,
    pub registration_material_digest: String,
    pub verifier_operator: String,
    pub verifier_product: String,
    pub key_id: String,
    pub algorithm: String,
    pub created_by_admin_user_id: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSandboxVerifierKeyTransitionSummary {
    pub transition_receipt_id: String,
    pub transition_receipt_digest: String,
    pub transition_kind: String,
    pub actor_user_id: String,
    pub reason: Option<String>,
    pub occurred_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSandboxVerifierKeyRegistrationWriteReceipt {
    pub key_record: ExternalPoolAdapterSandboxVerifierKeyRecordSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSandboxVerifierKeyTransitionWriteReceipt {
    pub key_record: ExternalPoolAdapterSandboxVerifierKeyRecordSummary,
    pub transition: ExternalPoolAdapterSandboxVerifierKeyTransitionSummary,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ExternalPoolAdapterSandboxVerifierKeyCurrentnessReceipt {
    pub schema: &'static str,
    pub key_record: ExternalPoolAdapterSandboxVerifierKeyRecordSummary,
    pub current_status: String,
    pub activation: Option<ExternalPoolAdapterSandboxVerifierKeyTransitionSummary>,
    pub revocation: Option<ExternalPoolAdapterSandboxVerifierKeyTransitionSummary>,
}

pub(super) struct StoredKeyRecord {
    pub record: ExternalPoolAdapterSandboxVerifierKeyRecord,
    pub json: String,
}

pub(super) struct StoredTransition {
    pub receipt: ExternalPoolAdapterSandboxVerifierKeyTransitionReceipt,
    pub json: String,
}

pub(in crate::store) struct CurrentExternalPoolAdapterSandboxVerifierKeyAuthority {
    key_record_id: String,
    key_record_digest: String,
    key_id: String,
    verifier_operator: String,
    verifier_product: String,
    public_key_pem: String,
}

pub(in crate::store) struct ExternalPoolAdapterSandboxVerifierKeyRecordAuthority {
    key_record_id: String,
    key_record_digest: String,
    key_id: String,
    verifier_operator: String,
    verifier_product: String,
    public_key_pem: String,
}

macro_rules! authority_impl {
    ($name:ident) => {
        impl $name {
            pub(super) fn new(root: &StoredKeyRecord) -> Self {
                let item = &root.record.registration;
                Self {
                    key_record_id: root.record.key_record_id.clone(),
                    key_record_digest: root.record.key_record_digest.clone(),
                    key_id: item.key_id.clone(),
                    verifier_operator: item.verifier_operator.clone(),
                    verifier_product: item.verifier_product.clone(),
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
            pub(in crate::store) fn verifier_operator(&self) -> &str {
                &self.verifier_operator
            }
            pub(in crate::store) fn verifier_product(&self) -> &str {
                &self.verifier_product
            }
            pub(in crate::store) fn public_key_pem(&self) -> &str {
                &self.public_key_pem
            }
        }
    };
}

authority_impl!(CurrentExternalPoolAdapterSandboxVerifierKeyAuthority);
authority_impl!(ExternalPoolAdapterSandboxVerifierKeyRecordAuthority);

impl StoredKeyRecord {
    pub(super) fn summary(&self) -> ExternalPoolAdapterSandboxVerifierKeyRecordSummary {
        let item = &self.record.registration;
        ExternalPoolAdapterSandboxVerifierKeyRecordSummary {
            key_record_id: self.record.key_record_id.clone(),
            key_record_digest: self.record.key_record_digest.clone(),
            registration_material_digest: self.record.registration_material_digest.clone(),
            verifier_operator: item.verifier_operator.clone(),
            verifier_product: item.verifier_product.clone(),
            key_id: item.key_id.clone(),
            algorithm: item.algorithm.clone(),
            created_by_admin_user_id: item.created_by_admin_user_id.clone(),
            created_at: item.created_at.clone(),
        }
    }
}

impl StoredTransition {
    pub(super) fn summary(&self) -> ExternalPoolAdapterSandboxVerifierKeyTransitionSummary {
        let item = &self.receipt.transition;
        ExternalPoolAdapterSandboxVerifierKeyTransitionSummary {
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
