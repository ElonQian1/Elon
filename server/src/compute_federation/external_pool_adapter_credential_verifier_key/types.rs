use serde::{Deserialize, Serialize};

pub(crate) const RECORD_SCHEMA: &str =
    "compute_federation.external_pool_adapter_credential_verifier_key.v1";
pub(crate) const REVOCATION_SCHEMA: &str =
    "compute_federation.external_pool_adapter_credential_verifier_key_revocation_receipt.v1";
pub(crate) const CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const KEY_ALGORITHM: &str = "rsa-pkcs1v15-sha256";
pub(crate) const ACTOR_KIND: &str = "platform_admin";
pub(crate) const STATUS_ACTIVE: &str = "active";
pub(crate) const STATUS_REVOKED: &str = "revoked";
pub(crate) const REGISTER_CONFIRMATION: &str =
    "confirm_external_pool_adapter_credential_verifier_key_registration";
pub(crate) const REVOKE_CONFIRMATION: &str =
    "confirm_external_pool_adapter_credential_verifier_key_revocation";
pub(crate) const NO_EFFECT: &str = "none";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CredentialVerifierKeyRegistration {
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
    pub public_key_pem: String,
    pub actor_kind: String,
    pub created_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub created_at: String,
    pub recorded_at: String,
    pub currentness_effect: String,
    pub credential_receipt_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CredentialVerifierKeyRecord {
    pub schema: String,
    pub key_record_id: String,
    pub key_record_digest: String,
    pub registration_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub registration: CredentialVerifierKeyRegistration,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CredentialVerifierKeyRevocation {
    pub key_record_id: String,
    pub key_record_digest: String,
    pub verifier_record_id: String,
    pub verifier_record_digest: String,
    pub key_id: String,
    pub actor_kind: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub revoked_at: String,
    pub recorded_at: String,
    pub currentness_effect: String,
    pub credential_receipt_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CredentialVerifierKeyRevocationReceipt {
    pub schema: String,
    pub revocation_receipt_id: String,
    pub revocation_receipt_digest: String,
    pub revocation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub revocation: CredentialVerifierKeyRevocation,
}
