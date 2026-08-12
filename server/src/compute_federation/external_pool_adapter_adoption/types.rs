use serde::{Deserialize, Serialize};

pub(crate) const ADOPTION_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_adoption_receipt.v1";
pub(crate) const ADOPTION_TERMINAL_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_adoption_terminal_receipt.v1";
pub(crate) const ADOPTION_CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_adapter_adoption_currentness.v1";
pub(crate) const ADOPTION_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const ADOPTION_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const ADOPTION_CONFIRMATION: &str = "confirm_external_pool_adapter_adoption";
pub(crate) const ADOPTION_REVOCATION_CONFIRMATION: &str =
    "confirm_external_pool_adapter_adoption_revocation";
pub(crate) const ADOPTION_AUTHORITY_EFFECT: &str = "adoption_authority_current";
pub(crate) const ADOPTION_REVOKED_EFFECT: &str = "adoption_authority_revoked";
pub(crate) const ADOPTION_INSTALL_EFFECT: &str = "authorization_only";
pub(crate) const ADOPTION_NO_EFFECT: &str = "none";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterAdoptionBinding {
    pub application_id: String,
    pub application_digest: String,
    pub provider_id: String,
    pub provider_owner_account_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub admission_id: String,
    pub admission_digest: String,
    pub adapter_id: String,
    pub adapter_release_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
    pub declared_implementation_sha256: String,
    pub capability_set_digest: String,
    pub sandbox_conformance_receipt_id: String,
    pub sandbox_conformance_receipt_digest: String,
    pub sandbox_report_expires_at: String,
    pub credential_verification_receipt_id: String,
    pub credential_verification_receipt_digest: String,
    pub credential_locator_commitment: String,
    pub credential_report_expires_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterAdoptionMaterial {
    pub binding: ExternalPoolAdapterAdoptionBinding,
    pub adopted_by_admin_user_id: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub adopted_at: String,
    pub recorded_at: String,
    pub adoption_effect: String,
    pub install_effect: String,
    pub provider_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterAdoptionReceipt {
    pub schema: String,
    pub adoption_receipt_id: String,
    pub adoption_receipt_digest: String,
    pub adoption_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub adoption: ExternalPoolAdapterAdoptionMaterial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterAdoptionTerminalMaterial {
    pub adoption_receipt_id: String,
    pub adoption_receipt_digest: String,
    pub revoked_by_admin_user_id: String,
    pub reason: String,
    pub confirmation: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub revoked_at: String,
    pub recorded_at: String,
    pub adoption_effect: String,
    pub provider_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterAdoptionTerminalReceipt {
    pub schema: String,
    pub terminal_receipt_id: String,
    pub terminal_receipt_digest: String,
    pub terminal_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub terminal: ExternalPoolAdapterAdoptionTerminalMaterial,
}
