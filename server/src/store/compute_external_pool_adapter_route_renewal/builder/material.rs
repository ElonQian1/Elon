use anyhow::Result;

use crate::compute_federation::{
    external_pool_adapter_route_renewal::*, route_authority::AuthorizedComputeRouteAuthorization,
};

pub(super) fn build_receipt(
    activation: &crate::compute_federation::external_pool_adapter_atomic_activation::ExternalPoolAdapterAtomicActivationReceipt,
    genesis: &crate::compute_federation::external_pool_adapter_provider_active_successor::ExternalPoolAdapterProviderActiveSuccessorReceipt,
    active: &crate::compute_federation::provider::ComputeProvider,
    active_digest: &str,
    root: &crate::compute_federation::external_pool_adapter_provider_active_successor::ExternalPoolAdapterProviderActiveSuccessorActivationRootEnvelope,
    evidence: &crate::compute_federation::external_pool_adapter_credential_reattestation::ExternalPoolAdapterCredentialReattestationReceipt,
    sequence: i64,
    predecessor_id: Option<String>,
    predecessor_digest: Option<String>,
    predecessor: &AuthorizedComputeRouteAuthorization,
    route: &AuthorizedComputeRouteAuthorization,
    checked_at: &str,
    expires_at: String,
    cleanup_expires_at: String,
    idempotency_json: String,
    idempotency_digest: String,
    receipt_id: String,
) -> Result<ExternalPoolAdapterRouteRenewalReceipt> {
    let old = predecessor.envelope();
    let old_seal = predecessor.seal();
    let renewed = route.envelope();
    let new_seal = route.seal();
    let material = ExternalPoolAdapterRouteRenewalMaterial {
        identity: ExternalPoolAdapterRouteRenewalIdentity {
            provider_binding_id: root.provider_binding_id.clone(),
            provider_binding_digest: root.provider_binding_digest.clone(),
            activation_root_digest: activation
                .activation
                .identity
                .activation_root_digest
                .clone(),
            renewal_sequence: sequence,
            predecessor_route_renewal_receipt_id: predecessor_id,
            predecessor_route_renewal_receipt_digest: predecessor_digest,
        },
        activation_witness: ExternalPoolAdapterRouteRenewalActivationWitness {
            activation_receipt_id: activation.activation_receipt_id.clone(),
            activation_receipt_digest: activation.activation_receipt_digest.clone(),
            activation_genesis_successor_receipt_id: genesis.active_successor_receipt_id.clone(),
            activation_genesis_successor_receipt_digest: genesis.receipt_digest.clone(),
        },
        active_subject: ExternalPoolAdapterRouteRenewalActiveSubject {
            active_provider_id: active.provider_id.clone(),
            active_provider_policy_revision: active.policy_revision,
            active_provider_digest: active_digest.to_owned(),
        },
        stable_binding: ExternalPoolAdapterRouteRenewalStableBinding {
            executor_id: activation.activation.stable_executor.executor_id.clone(),
            stable_executor_binding_digest: activation
                .activation
                .stable_executor
                .stable_executor_binding_digest
                .clone(),
            projected_v211_adapter_binding_digest: activation
                .activation
                .projected_v211_binding
                .projected_v211_adapter_binding_digest
                .clone(),
            route_adapter_projection_id: activation
                .activation
                .route_closure
                .route_adapter_projection_id
                .clone(),
            route_adapter_revision: activation.activation.route_closure.route_adapter_revision,
            route_adapter_digest: activation
                .activation
                .route_closure
                .route_adapter_digest
                .clone(),
        },
        predecessor_route: ExternalPoolAdapterRouteRenewalPredecessorClosure {
            service_actor_authorization_id: old.authorization.actor_authorization_id.clone(),
            service_actor_authorization_digest: old
                .authorization
                .actor_authorization_digest
                .clone(),
            route_credential_id: old.authorization.credential.credential_id.clone(),
            route_credential_revision: old.authorization.credential.credential_revision,
            route_credential_digest: old.authorization.credential.credential_digest.clone(),
            route_authorization_id: old.route_authorization_id.clone(),
            route_authorization_revision: old.route_authorization_revision,
            route_authorization_digest: old.route_authorization_digest.clone(),
            route_seal_id: old_seal.seal_id.clone(),
            route_seal_digest: old_seal.seal_digest.clone(),
        },
        credential_evidence: ExternalPoolAdapterRouteRenewalCredentialEvidence {
            credential_reattestation_receipt_id: evidence.reattestation_receipt_id.clone(),
            credential_reattestation_receipt_digest: evidence.reattestation_receipt_digest.clone(),
        },
        renewed_route: ExternalPoolAdapterRenewedRouteClosure {
            service_actor_id: renewed.authorization.verified_by_service_actor_id.clone(),
            service_actor_authorization_id: renewed.authorization.actor_authorization_id.clone(),
            service_actor_authorization_revision: route
                .inputs()
                .actor()
                .envelope()
                .actor_authorization_revision,
            service_actor_authorization_digest: renewed
                .authorization
                .actor_authorization_digest
                .clone(),
            route_credential_id: renewed.authorization.credential.credential_id.clone(),
            route_credential_revision: renewed.authorization.credential.credential_revision,
            route_credential_digest: renewed.authorization.credential.credential_digest.clone(),
            route_authorization_id: renewed.route_authorization_id.clone(),
            route_authorization_revision: renewed.route_authorization_revision,
            route_authorization_digest: renewed.route_authorization_digest.clone(),
            route_capabilities: renewed.authorization.capabilities.clone(),
            route_capability_set_digest: new_seal.capability_set_digest.clone(),
            route_seal_id: new_seal.seal_id.clone(),
            route_seal_digest: new_seal.seal_digest.clone(),
        },
        timing: ExternalPoolAdapterRouteRenewalTiming {
            authenticated_at: checked_at.to_owned(),
            authorized_at: checked_at.to_owned(),
            expires_at,
            cleanup_expires_at,
            evidence_checked_at: checked_at.to_owned(),
            created_at: checked_at.to_owned(),
        },
        audit: ExternalPoolAdapterRouteRenewalAudit {
            delegation_id: root.delegation_id.clone(),
            delegation_digest: root.delegation_digest.clone(),
            renewal_policy_digest: canonical_external_pool_adapter_route_renewal_policy_digest()?,
            renewed_by_actor_kind: ROUTE_RENEWAL_ACTOR_KIND.into(),
            renewed_by_service_actor_id: root.service_actor_id.clone(),
            idempotency_material_json: idempotency_json,
            idempotency_digest,
        },
    };
    build_external_pool_adapter_route_renewal_receipt_from_material(receipt_id, material)
}
