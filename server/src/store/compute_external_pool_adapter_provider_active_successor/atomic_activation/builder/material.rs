//! Canonical V277 receipt projection from the final typed evidence set.

use anyhow::Result;

use crate::{
    compute_federation::{
        external_pool_adapter_atomic_activation::*,
        external_pool_adapter_provider_active_successor::{
            provider_active_successor_runtime_observation_digest,
            ExternalPoolAdapterProviderActiveSuccessorProviderEvidence,
            ExternalPoolAdapterProviderActiveSuccessorRuntimeObservation,
        },
        route_authority::{
            canonical_route_capability_set_digest, AuthorizedComputeRouteAuthorization,
        },
    },
    store::{
        compute_external_pool_adapter_credential_reattestation::PreparedExternalPoolAdapterCredentialProjectedActiveTransition,
        compute_external_pool_adapter_runtime_bundle::ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject,
        compute_external_pool_adapter_task_protocol_conformance::PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier,
        new_id,
    },
};

#[allow(clippy::too_many_arguments)]
pub(super) fn build_genesis_receipt(
    no_work: &ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject<'_, '_, '_>,
    transition: &PreparedExternalPoolAdapterCredentialProjectedActiveTransition<'_, '_, '_>,
    task_protocol: &PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier<'_, '_>,
    stable: &ExternalPoolStableExecutorBinding,
    projected: &ExternalPoolProjectedV211AdapterBinding,
    route: &AuthorizedComputeRouteAuthorization,
    target_json: String,
    target_digest: String,
) -> Result<ExternalPoolAdapterAtomicActivationReceipt> {
    let planned = no_work.preflight();
    let root_envelope = planned.activation_root();
    let root = &root_envelope.activation_root;
    let observation = no_work.observation();
    let credential = transition.credential().receipt();
    let source = provider_evidence(
        root.source_registering_provider_id.clone(),
        root.source_registering_provider_policy_revision,
        root.source_registering_provider_json.clone(),
        root.source_registering_provider_digest.clone(),
    );
    let target = provider_evidence(
        root.initial_active_provider_id.clone(),
        root.initial_active_provider_policy_revision,
        target_json,
        target_digest,
    );
    let fresh_expires_at = task_protocol.fresh_expires_at_for(no_work)?;
    let mut runtime_observation = ExternalPoolAdapterProviderActiveSuccessorRuntimeObservation {
        runtime_observation_id: observation.post_cleanup_observation_commitment().into(),
        runtime_observation_digest: String::new(),
        observed_provider: ExternalPoolAdapterProviderActiveSuccessorProviderEvidence {
            provider_id: target.provider_id.clone(),
            provider_policy_revision: target.provider_policy_revision,
            provider_json: target.provider_json.clone(),
            provider_digest: target.provider_digest.clone(),
        },
        observation_started_at: observation.probe_checked_at().into(),
        observation_completed_at: observation.checked_at().into(),
        observation_expires_at: fresh_expires_at,
    };
    runtime_observation.runtime_observation_digest =
        provider_active_successor_runtime_observation_digest(&runtime_observation)?;
    let idempotency = ExternalPoolAdapterAtomicActivationIdempotencyMaterial {
        actor_kind: ATOMIC_ACTIVATION_ACTOR_KIND.into(),
        actor_user_id: root.provider_owner_account_id.clone(),
        provider_binding_id: root.provider_binding_id.clone(),
        provider_binding_digest: root.provider_binding_digest.clone(),
        activation_root_digest: root_envelope.activation_root_digest.clone(),
        scope: ATOMIC_ACTIVATION_IDEMPOTENCY_SCOPE.into(),
        key: root_envelope.activation_root_digest.clone(),
    };
    let (idempotency_material_json, idempotency_digest) =
        canonical_atomic_activation_idempotency_json_and_digest(&idempotency)?;
    let confirmation = ExternalPoolAdapterAtomicActivationConfirmationMaterial {
        confirmation: ATOMIC_ACTIVATION_CONFIRMATION.into(),
        actor_kind: ATOMIC_ACTIVATION_ACTOR_KIND.into(),
        actor_user_id: root.provider_owner_account_id.clone(),
        idempotency_digest: idempotency_digest.clone(),
        provider_binding_id: root.provider_binding_id.clone(),
        provider_binding_digest: root.provider_binding_digest.clone(),
        activation_root_digest: root_envelope.activation_root_digest.clone(),
    };
    let (confirmation_material_json, confirmation_digest) =
        canonical_atomic_activation_confirmation_json_and_digest(&confirmation)?;
    let authorization = route.envelope();
    let route_credential = route.inputs().credential().envelope();
    let actor = route.inputs().actor().envelope();
    let adapter = route.inputs().adapter().envelope();
    let seal = route.seal();
    let capabilities = authorization.authorization.capabilities.clone();
    let activation = ExternalPoolAdapterAtomicActivationMaterial {
        identity: ExternalPoolAdapterAtomicActivationIdentity {
            provider_binding_id: root.provider_binding_id.clone(),
            provider_binding_digest: root.provider_binding_digest.clone(),
            activation_root_digest: root_envelope.activation_root_digest.clone(),
        },
        provider_transition: ExternalPoolAdapterAtomicActivationProviderTransition {
            source_registering_provider: source,
            target_active_provider: target,
        },
        v253_genesis_input: ExternalPoolAdapterAtomicActivationV253GenesisInput {
            registering_reattestation_receipt_id: credential.reattestation_receipt_id.clone(),
            registering_reattestation_receipt_digest: credential
                .reattestation_receipt_digest
                .clone(),
            projected_transition_proof_material_json: transition.proof_material_json().into(),
            projected_transition_proof_digest: transition.proof_digest().into(),
        },
        stable_executor: stable.clone(),
        projected_v211_binding: projected.clone(),
        route_closure: ExternalPoolAdapterAtomicActivationRouteClosure {
            route_adapter_projection_id: adapter.adapter_id.clone(),
            route_adapter_revision: adapter.adapter_revision,
            route_adapter_digest: adapter.adapter_digest.clone(),
            service_actor_id: actor.authorization.service_actor_id.clone(),
            service_actor_authorization_id: actor.actor_authorization_id.clone(),
            service_actor_authorization_digest: actor.actor_authorization_digest.clone(),
            route_credential_id: route_credential.credential_id.clone(),
            route_credential_revision: route_credential.credential_revision,
            route_credential_digest: route_credential.credential_digest.clone(),
            route_authorization_id: authorization.route_authorization_id.clone(),
            route_authorization_revision: authorization.route_authorization_revision,
            route_authorization_digest: authorization.route_authorization_digest.clone(),
            route_capability_count: i64::try_from(capabilities.len())?,
            route_capability_set_digest: canonical_route_capability_set_digest(&capabilities)?,
            capabilities,
            route_seal_id: seal.seal_id.clone(),
            route_seal_digest: seal.seal_digest.clone(),
        },
        renewable_evidence: ExternalPoolAdapterAtomicActivationRenewableEvidence {
            active_runtime_observation_id: runtime_observation.runtime_observation_id,
            active_runtime_observation_digest: runtime_observation.runtime_observation_digest,
            observation_started_at: runtime_observation.observation_started_at,
            observation_completed_at: runtime_observation.observation_completed_at,
            observation_expires_at: runtime_observation.observation_expires_at,
            task_protocol_conformance_run_receipt_id: task_protocol
                .receipt()
                .run_receipt_id
                .clone(),
            task_protocol_conformance_run_receipt_digest: task_protocol
                .receipt()
                .run_receipt_digest
                .clone(),
            task_protocol_conformance_expires_at: task_protocol.receipt().run.expires_at.clone(),
            task_protocol_active_carrier_material_json: task_protocol.material_json().into(),
            task_protocol_active_carrier_digest: task_protocol.digest().into(),
        },
        audit: ExternalPoolAdapterAtomicActivationAudit {
            activated_by_actor_kind: ATOMIC_ACTIVATION_ACTOR_KIND.into(),
            activated_by_actor_user_id: root.provider_owner_account_id.clone(),
            idempotency_scope: ATOMIC_ACTIVATION_IDEMPOTENCY_SCOPE.into(),
            idempotency_key: root_envelope.activation_root_digest.clone(),
            idempotency_material_json,
            idempotency_digest,
            confirmation: ATOMIC_ACTIVATION_CONFIRMATION.into(),
            confirmation_material_json,
            confirmation_digest,
        },
        activation_target_updated_at: planned.activation_target_updated_at().into(),
        evidence_checked_at: no_work.evidence_checked_at().into(),
        created_at: no_work.evidence_checked_at().into(),
    };
    build_external_pool_adapter_atomic_activation_receipt(
        new_id("external_pool_adapter_atomic_activation"),
        activation,
    )
}

fn provider_evidence(
    provider_id: String,
    provider_policy_revision: i64,
    provider_json: String,
    provider_digest: String,
) -> ExternalPoolAdapterAtomicActivationProviderEvidence {
    ExternalPoolAdapterAtomicActivationProviderEvidence {
        provider_id,
        provider_policy_revision,
        provider_json,
        provider_digest,
    }
}
