use serde::{Deserialize, Serialize};

pub(crate) const SANDBOX_VERIFIER_KEY_RECORD_SCHEMA: &str =
    "compute_federation.external_pool_adapter_sandbox_verifier_key.v1";
pub(crate) const SANDBOX_VERIFIER_KEY_ACTIVATION_SCHEMA: &str =
    "compute_federation.external_pool_adapter_sandbox_verifier_key_activation_receipt.v1";
pub(crate) const SANDBOX_VERIFIER_KEY_REVOCATION_SCHEMA: &str =
    "compute_federation.external_pool_adapter_sandbox_verifier_key_revocation_receipt.v1";
pub(crate) const SANDBOX_VERIFIER_KEY_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const SANDBOX_VERIFIER_KEY_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const SANDBOX_VERIFIER_KEY_ALGORITHM: &str = "rsa-pkcs1v15-sha256";
pub(crate) const SANDBOX_VERIFIER_KEY_ACTOR_KIND: &str = "platform_admin";
pub(crate) const SANDBOX_VERIFIER_KEY_STATUS_PENDING: &str = "pending_activation";
pub(crate) const SANDBOX_VERIFIER_KEY_STATUS_ACTIVE: &str = "active";
pub(crate) const SANDBOX_VERIFIER_KEY_STATUS_REVOKED: &str = "revoked";
pub(crate) const SANDBOX_VERIFIER_KEY_REGISTER_CONFIRMATION: &str =
    "confirm_external_pool_adapter_sandbox_verifier_key_registration";
pub(crate) const SANDBOX_VERIFIER_KEY_ACTIVATE_CONFIRMATION: &str =
    "confirm_external_pool_adapter_sandbox_verifier_key_activation";
pub(crate) const SANDBOX_VERIFIER_KEY_REVOKE_CONFIRMATION: &str =
    "confirm_external_pool_adapter_sandbox_verifier_key_revocation";
pub(crate) const SANDBOX_VERIFIER_KEY_NO_EFFECT: &str = "none";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSandboxVerifierKeyRegistration {
    pub verifier_operator: String,
    pub verifier_product: String,
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
    pub conformance_report_effect: String,
    pub vulnerability_report_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSandboxVerifierKeyRecord {
    pub schema: String,
    pub key_record_id: String,
    pub key_record_digest: String,
    pub registration_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub registration: ExternalPoolAdapterSandboxVerifierKeyRegistration,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSandboxVerifierKeyTransition {
    pub key_record_id: String,
    pub key_record_digest: String,
    pub key_id: String,
    pub verifier_operator: String,
    pub verifier_product: String,
    pub actor_kind: String,
    pub actor_user_id: String,
    pub reason: Option<String>,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub occurred_at: String,
    pub recorded_at: String,
    pub currentness_effect: String,
    pub conformance_report_effect: String,
    pub vulnerability_report_effect: String,
    pub adapter_effect: String,
    pub route_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterSandboxVerifierKeyTransitionReceipt {
    pub schema: String,
    pub transition_receipt_id: String,
    pub transition_receipt_digest: String,
    pub transition_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub transition: ExternalPoolAdapterSandboxVerifierKeyTransition,
}
