use serde::{Deserialize, Serialize};

use super::super::route_authority::ComputeRouteCapabilityBinding;

pub(crate) const ROUTE_RENEWAL_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_route_renewal.v1";
pub(crate) const ROUTE_RENEWAL_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const ROUTE_RENEWAL_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const ROUTE_RENEWAL_ACTOR_KIND: &str = "platform_dispatch_service";
pub(crate) const ROUTE_RENEWAL_MAX_JSON_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const ROUTE_RENEWAL_RENEW_BEFORE_SECONDS: i64 = 60;
pub(crate) const ROUTE_RENEWAL_FRESH_MAX_SECONDS: i64 = 300;
pub(crate) const ROUTE_RENEWAL_CLEANUP_MAX_SECONDS: i64 = 1_800;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRouteRenewalIdentity {
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub activation_root_digest: String,
    pub renewal_sequence: i64,
    pub predecessor_route_renewal_receipt_id: Option<String>,
    pub predecessor_route_renewal_receipt_digest: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRouteRenewalActivationWitness {
    pub activation_receipt_id: String,
    pub activation_receipt_digest: String,
    pub activation_genesis_successor_receipt_id: String,
    pub activation_genesis_successor_receipt_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRouteRenewalActiveSubject {
    pub active_provider_id: String,
    pub active_provider_policy_revision: i64,
    pub active_provider_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRouteRenewalStableBinding {
    pub executor_id: String,
    pub stable_executor_binding_digest: String,
    pub projected_v211_adapter_binding_digest: String,
    pub route_adapter_projection_id: String,
    pub route_adapter_revision: i64,
    pub route_adapter_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRouteRenewalPredecessorClosure {
    pub service_actor_authorization_id: String,
    pub service_actor_authorization_digest: String,
    pub route_credential_id: String,
    pub route_credential_revision: i64,
    pub route_credential_digest: String,
    pub route_authorization_id: String,
    pub route_authorization_revision: i64,
    pub route_authorization_digest: String,
    pub route_seal_id: String,
    pub route_seal_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRouteRenewalCredentialEvidence {
    pub credential_reattestation_receipt_id: String,
    pub credential_reattestation_receipt_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRenewedRouteClosure {
    pub service_actor_id: String,
    pub service_actor_authorization_id: String,
    pub service_actor_authorization_revision: i64,
    pub service_actor_authorization_digest: String,
    pub route_credential_id: String,
    pub route_credential_revision: i64,
    pub route_credential_digest: String,
    pub route_authorization_id: String,
    pub route_authorization_revision: i64,
    pub route_authorization_digest: String,
    pub route_capabilities: Vec<ComputeRouteCapabilityBinding>,
    pub route_capability_set_digest: String,
    pub route_seal_id: String,
    pub route_seal_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRouteRenewalTiming {
    pub authenticated_at: String,
    pub authorized_at: String,
    pub expires_at: String,
    pub cleanup_expires_at: String,
    pub evidence_checked_at: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRouteRenewalAudit {
    pub delegation_id: String,
    pub delegation_digest: String,
    pub renewal_policy_digest: String,
    pub renewed_by_actor_kind: String,
    pub renewed_by_service_actor_id: String,
    pub idempotency_material_json: String,
    pub idempotency_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRouteRenewalMaterial {
    pub identity: ExternalPoolAdapterRouteRenewalIdentity,
    pub activation_witness: ExternalPoolAdapterRouteRenewalActivationWitness,
    pub active_subject: ExternalPoolAdapterRouteRenewalActiveSubject,
    pub stable_binding: ExternalPoolAdapterRouteRenewalStableBinding,
    pub predecessor_route: ExternalPoolAdapterRouteRenewalPredecessorClosure,
    pub credential_evidence: ExternalPoolAdapterRouteRenewalCredentialEvidence,
    pub renewed_route: ExternalPoolAdapterRenewedRouteClosure,
    pub timing: ExternalPoolAdapterRouteRenewalTiming,
    pub audit: ExternalPoolAdapterRouteRenewalAudit,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRouteRenewalReceipt {
    pub schema: String,
    pub route_renewal_receipt_id: String,
    pub route_renewal_receipt_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub renewal: ExternalPoolAdapterRouteRenewalMaterial,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRouteRenewalPolicy {
    pub renew_before_seconds: i64,
    pub fresh_max_seconds: i64,
    pub cleanup_max_seconds: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterRouteRenewalIdempotencyMaterial {
    pub provider_binding_id: String,
    pub activation_receipt_id: String,
    pub activation_root_digest: String,
    pub renewal_sequence: i64,
    pub predecessor_route_renewal_receipt_id: Option<String>,
    pub predecessor_route_renewal_receipt_digest: Option<String>,
    pub credential_reattestation_receipt_id: String,
    pub credential_reattestation_receipt_digest: String,
    pub evidence_checked_at: String,
}
