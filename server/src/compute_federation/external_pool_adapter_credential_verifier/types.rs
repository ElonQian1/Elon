use serde::{Deserialize, Serialize};

pub(crate) const CREDENTIAL_VERIFIER_RECORD_SCHEMA: &str =
    "compute_federation.external_pool_adapter_credential_verifier.v1";
pub(crate) const CREDENTIAL_VERIFIER_ACTIVATION_SCHEMA: &str =
    "compute_federation.external_pool_adapter_credential_verifier_activation_receipt.v1";
pub(crate) const CREDENTIAL_VERIFIER_REVOCATION_SCHEMA: &str =
    "compute_federation.external_pool_adapter_credential_verifier_revocation_receipt.v1";
pub(crate) const CREDENTIAL_VERIFIER_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const CREDENTIAL_VERIFIER_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const CREDENTIAL_VERIFIER_ACTOR_KIND: &str = "platform_admin";
pub(crate) const CREDENTIAL_VERIFIER_STATUS_PENDING: &str = "pending_activation";
pub(crate) const CREDENTIAL_VERIFIER_STATUS_ACTIVE: &str = "active";
pub(crate) const CREDENTIAL_VERIFIER_STATUS_REVOKED: &str = "revoked";
pub(crate) const CREDENTIAL_VERIFIER_REGISTER_CONFIRMATION: &str =
    "confirm_external_pool_adapter_credential_verifier_registration";
pub(crate) const CREDENTIAL_VERIFIER_ACTIVATE_CONFIRMATION: &str =
    "confirm_external_pool_adapter_credential_verifier_activation";
pub(crate) const CREDENTIAL_VERIFIER_REVOKE_CONFIRMATION: &str =
    "confirm_external_pool_adapter_credential_verifier_revocation";
pub(crate) const CREDENTIAL_VERIFIER_NO_EFFECT: &str = "none";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCredentialVerifierRegistration {
    pub verifier_operator: String,
    pub verifier_product: String,
    pub verification_kind: String,
    pub verifier_id: String,
    pub verifier_revision: i64,
    pub verifier_digest: String,
    pub actor_kind: String,
    pub created_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub created_at: String,
    pub recorded_at: String,
    pub currentness_effect: String,
    pub credential_receipt_effect: String,
    pub adapter_adoption_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCredentialVerifierRecord {
    pub schema: String,
    pub verifier_record_id: String,
    pub verifier_record_digest: String,
    pub registration_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub registration: ExternalPoolAdapterCredentialVerifierRegistration,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCredentialVerifierTransition {
    pub verifier_record_id: String,
    pub verifier_record_digest: String,
    pub verification_kind: String,
    pub verifier_id: String,
    pub verifier_revision: i64,
    pub verifier_digest: String,
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
    pub credential_receipt_effect: String,
    pub adapter_adoption_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCredentialVerifierTransitionReceipt {
    pub schema: String,
    pub transition_receipt_id: String,
    pub transition_receipt_digest: String,
    pub transition_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub transition: ExternalPoolAdapterCredentialVerifierTransition,
}
