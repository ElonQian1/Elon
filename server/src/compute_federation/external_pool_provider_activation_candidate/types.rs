use serde::{Deserialize, Serialize};

pub(crate) const ACTIVATION_DELEGATION_SCHEMA: &str =
    "compute_federation.external_pool_provider_activation_delegation.v1";
pub(crate) const ACTIVATION_CANDIDATE_SCHEMA: &str =
    "compute_federation.external_pool_provider_activation_candidate.v1";
pub(crate) const ACTIVATION_DELEGATION_REVOCATION_SCHEMA: &str =
    "compute_federation.external_pool_provider_activation_delegation_revocation.v1";
pub(crate) const ACTIVATION_CANDIDATE_CURRENTNESS_SCHEMA: &str =
    "compute_federation.external_pool_provider_activation_candidate_currentness.v1";
pub(crate) const ACTIVATION_PREFLIGHT_SCHEMA: &str =
    "compute_federation.external_pool_provider_activation_preflight.v1";
pub(crate) const ACTIVATION_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const ACTIVATION_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const ACTIVATION_CANDIDATE_CONFIRMATION: &str =
    "confirm_external_pool_provider_activation_candidate";
pub(crate) const ACTIVATION_DELEGATION_REVOCATION_CONFIRMATION: &str =
    "confirm_external_pool_provider_activation_delegation_revocation";
pub(crate) const ACTIVATION_SERVICE_ACTOR_KIND: &str = "platform_dispatch_service";
pub(crate) const ACTIVATION_CANDIDATE_STATUS: &str = "candidate_current_not_activation_ready";
pub(crate) const ACTIVATION_INPUTS_CURRENT: &str = "inputs_current";
pub(crate) const ACTIVATION_CLOSURE_NOT_IMPLEMENTED: &str = "activation_closure_not_implemented";
pub(crate) const ACTIVATION_DELEGATION_EFFECT: &str = "owner_delegation_recorded";
pub(crate) const ACTIVATION_CANDIDATE_EFFECT: &str = "activation_candidate_recorded";
pub(crate) const ACTIVATION_DELEGATION_REVOCATION_EFFECT: &str = "owner_delegation_revoked";
pub(crate) const ACTIVATION_ROUTE_CANDIDATE_ONLY: &str = "candidate_only";
pub(crate) const ACTIVATION_NO_EFFECT: &str = "none";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolProviderActivationDelegationMaterial {
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub route_adapter_projection_id: String,
    pub provider_id: String,
    pub provider_owner_account_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub provider_status: String,
    pub logical_adapter_id: String,
    pub release_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
    pub service_actor_id: String,
    pub service_actor_kind: String,
    pub allowed_route_kinds: Vec<String>,
    pub allowed_actor_phases: Vec<String>,
    pub issued_by_owner_user_id: String,
    pub issued_at: String,
    pub recorded_at: String,
    pub sequence: u64,
    pub predecessor_delegation_id: Option<String>,
    pub predecessor_delegation_digest: Option<String>,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
    pub delegation_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolProviderActivationDelegationReceipt {
    pub schema: String,
    pub delegation_id: String,
    pub delegation_digest: String,
    pub delegation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub delegation: ExternalPoolProviderActivationDelegationMaterial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolProviderActivationCandidateMaterial {
    pub delegation_id: String,
    pub delegation_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub installation_receipt_id: String,
    pub installation_receipt_digest: String,
    pub installation_content_digest: String,
    pub route_adapter_projection_id: String,
    pub provider_id: String,
    pub provider_owner_account_id: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub provider_status: String,
    pub logical_adapter_id: String,
    pub release_version: String,
    pub adapter_config_revision: i64,
    pub adapter_config_digest: String,
    pub implementation_digest: String,
    pub capability_set_digest: String,
    pub credential_verifier_digest: String,
    pub logical_adapter_binding_digest: String,
    pub logical_projection_compatibility_digest: String,
    pub service_actor_id: String,
    pub sequence: u64,
    pub predecessor_candidate_id: Option<String>,
    pub predecessor_candidate_digest: Option<String>,
    pub checked_at: String,
    pub recorded_at: String,
    pub candidate_status: String,
    pub activation_closure_status: String,
    pub candidate_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolProviderActivationCandidateReceipt {
    pub schema: String,
    pub candidate_id: String,
    pub candidate_digest: String,
    pub candidate_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub candidate: ExternalPoolProviderActivationCandidateMaterial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolProviderActivationDelegationRevocationMaterial {
    pub delegation_id: String,
    pub delegation_digest: String,
    pub candidate_id: String,
    pub candidate_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub provider_id: String,
    pub revoked_by_owner_user_id: String,
    pub reason: String,
    pub revoked_at: String,
    pub recorded_at: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub confirmation: String,
    pub revocation_effect: String,
    pub provider_effect: String,
    pub credential_effect: String,
    pub route_effect: String,
    pub execution_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolProviderActivationDelegationRevocationReceipt {
    pub schema: String,
    pub revocation_id: String,
    pub revocation_digest: String,
    pub revocation_material_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub revocation: ExternalPoolProviderActivationDelegationRevocationMaterial,
}
