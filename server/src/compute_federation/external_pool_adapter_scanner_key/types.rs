use serde::{Deserialize, Serialize};

pub(crate) const SCANNER_KEY_RECORD_SCHEMA: &str =
    "compute_federation.external_pool_adapter_scanner_key.v1";
pub(crate) const SCANNER_KEY_ACTIVATION_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_scanner_key_activation_receipt.v1";
pub(crate) const SCANNER_KEY_REVOCATION_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_scanner_key_revocation_receipt.v1";
pub(crate) const SCANNER_KEY_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const SCANNER_KEY_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const SCANNER_KEY_ALGORITHM: &str = "rsa-pkcs1v15-sha256";
pub(crate) const SCANNER_KEY_ACTOR_KIND: &str = "platform_admin";
pub(crate) const SCANNER_KEY_STATUS_PENDING: &str = "pending_activation";
pub(crate) const SCANNER_KEY_STATUS_ACTIVE: &str = "active";
pub(crate) const SCANNER_KEY_STATUS_REVOKED: &str = "revoked";
pub(crate) const SCANNER_KEY_REGISTER_CONFIRMATION: &str =
    "confirm_external_pool_adapter_scanner_key_registration";
pub(crate) const SCANNER_KEY_ACTIVATE_CONFIRMATION: &str =
    "confirm_external_pool_adapter_scanner_key_activation";
pub(crate) const SCANNER_KEY_REVOKE_CONFIRMATION: &str =
    "confirm_external_pool_adapter_scanner_key_revocation";
pub(crate) const SCANNER_KEY_NO_EFFECT: &str = "none";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterScannerKeyRegistration {
    pub scanner_operator: String,
    pub scanner_product: String,
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
    pub vulnerability_report_effect: String,
    pub artifact_security_effect: String,
    pub conformance_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterScannerKeyRecord {
    pub schema: String,
    pub key_record_id: String,
    pub key_record_digest: String,
    pub registration_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub registration: ExternalPoolAdapterScannerKeyRegistration,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterScannerKeyActivation {
    pub key_record_id: String,
    pub key_record_digest: String,
    pub key_id: String,
    pub scanner_operator: String,
    pub scanner_product: String,
    pub actor_kind: String,
    pub activated_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub occurred_at: String,
    pub recorded_at: String,
    pub currentness_effect: String,
    pub vulnerability_report_effect: String,
    pub artifact_security_effect: String,
    pub conformance_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterScannerKeyActivationReceipt {
    pub schema: String,
    pub activation_receipt_id: String,
    pub activation_receipt_digest: String,
    pub activation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub activation: ExternalPoolAdapterScannerKeyActivation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterScannerKeyRevocation {
    pub key_record_id: String,
    pub key_record_digest: String,
    pub key_id: String,
    pub scanner_operator: String,
    pub scanner_product: String,
    pub actor_kind: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub occurred_at: String,
    pub recorded_at: String,
    pub currentness_effect: String,
    pub vulnerability_report_effect: String,
    pub artifact_security_effect: String,
    pub conformance_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterScannerKeyRevocationReceipt {
    pub schema: String,
    pub revocation_receipt_id: String,
    pub revocation_receipt_digest: String,
    pub revocation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub revocation: ExternalPoolAdapterScannerKeyRevocation,
}
