use serde::{Deserialize, Serialize};

pub(crate) const SIGNING_KEY_RECORD_SCHEMA: &str =
    "compute_federation.external_pool_adapter_artifact_signing_key.v1";
pub(crate) const SIGNING_KEY_ACTIVATION_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_artifact_signing_key_activation_receipt.v1";
pub(crate) const SIGNING_KEY_REVOCATION_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_artifact_signing_key_revocation_receipt.v1";
pub(crate) const SIGNING_KEY_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const SIGNING_KEY_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const SIGNING_KEY_ALGORITHM: &str = "rsa-pkcs1v15-sha256";
pub(crate) const SIGNING_KEY_ACTOR_KIND: &str = "platform_admin";
pub(crate) const SIGNING_KEY_STATUS_PENDING_ACTIVATION: &str = "pending_activation";
pub(crate) const SIGNING_KEY_STATUS_ACTIVE: &str = "active";
pub(crate) const SIGNING_KEY_STATUS_REVOKED: &str = "revoked";
pub(crate) const SIGNING_KEY_REGISTRATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_artifact_signing_key_registration";
pub(crate) const SIGNING_KEY_ACTIVATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_artifact_signing_key_activation";
pub(crate) const SIGNING_KEY_REVOCATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_artifact_signing_key_revocation";
pub(crate) const SIGNING_KEY_ARTIFACT_EFFECT_NONE: &str = "none";
pub(crate) const SIGNING_KEY_ADAPTER_EFFECT_NONE: &str = "none";
pub(crate) const SIGNING_KEY_ROUTE_EFFECT_NONE: &str = "none";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactSigningKeyRegistration {
    pub source_operator: String,
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
    pub artifact_signature_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactSigningKeyRecord {
    pub schema: String,
    pub key_record_id: String,
    pub key_record_digest: String,
    pub registration_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub registration: ExternalPoolAdapterArtifactSigningKeyRegistration,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactSigningKeyActivation {
    pub key_record_id: String,
    pub key_record_digest: String,
    pub key_id: String,
    pub source_operator: String,
    pub actor_kind: String,
    pub activated_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub occurred_at: String,
    pub recorded_at: String,
    pub currentness_effect: String,
    pub artifact_signature_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactSigningKeyActivationReceipt {
    pub schema: String,
    pub activation_receipt_id: String,
    pub activation_receipt_digest: String,
    pub activation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub activation: ExternalPoolAdapterArtifactSigningKeyActivation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactSigningKeyRevocation {
    pub key_record_id: String,
    pub key_record_digest: String,
    pub key_id: String,
    pub source_operator: String,
    pub actor_kind: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub occurred_at: String,
    pub recorded_at: String,
    pub currentness_effect: String,
    pub artifact_signature_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterArtifactSigningKeyRevocationReceipt {
    pub schema: String,
    pub revocation_receipt_id: String,
    pub revocation_receipt_digest: String,
    pub revocation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub revocation: ExternalPoolAdapterArtifactSigningKeyRevocation,
}
