use anyhow::{bail, ensure, Result};
use rusqlite::{params, Connection, Transaction};

use crate::{
    compute_federation::{
        external_pool_adapter_atomic_activation::{
            ExternalPoolAdapterAtomicActivationReceipt, ExternalPoolStableExecutorBindingMaterial,
            ExternalPoolStableExecutorIdMaterial,
        },
        external_pool_adapter_provider_active_successor::{
            provider_active_successor_runtime_observation_digest,
            ExternalPoolAdapterProviderActiveSuccessorActivationWitness,
            ExternalPoolAdapterProviderActiveSuccessorCredentialEvidence,
            ExternalPoolAdapterProviderActiveSuccessorEffects,
            ExternalPoolAdapterProviderActiveSuccessorLineage,
            ExternalPoolAdapterProviderActiveSuccessorMaterial,
            ExternalPoolAdapterProviderActiveSuccessorProviderEvidence,
            ExternalPoolAdapterProviderActiveSuccessorReadiness,
            ExternalPoolAdapterProviderActiveSuccessorRuntimeObservation,
            ExternalPoolAdapterProviderActiveSuccessorTaskProtocolEvidence,
            PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT,
        },
        provider::{ComputeProvider, PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_REGISTERING},
        route_authority::AuthorizedComputeRouteAuthorization,
    },
    store::{
        compute_attempt_start_outbox::{
            audit_persisted_compute_route_authority_on, persist_compute_route_authority_on,
        },
        compute_external_pool_adapter_credential_reattestation::PreparedExternalPoolAdapterCredentialProjectedActiveTransition,
        compute_external_pool_adapter_runtime_bundle::{
            install_external_pool_adapter_atomic_activation_pending_plan_on,
            ExternalPoolAdapterAtomicActivationPendingPlanGuard,
            ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
            ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject,
        },
        compute_external_pool_adapter_task_protocol_conformance::PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier,
        compute_provider_registry::{
            current_registered_provider_on, validate_compute_provider_contract,
            ComputeProviderRegistrationReceipt,
        },
    },
};

use super::super::append::{
    insert_prepared_external_pool_adapter_provider_active_successor_genesis_on,
    postcommit_external_pool_adapter_provider_active_successor_readback_on,
    prepare_external_pool_adapter_provider_active_successor_genesis_append_on,
    CommittedExternalPoolAdapterProviderActiveSuccessorAppend,
    PendingExternalPoolAdapterProviderActiveSuccessorAppend,
};
use super::{
    pending::build_pending_plan,
    read::historical_external_pool_adapter_atomic_activation_authority_on,
    receipt::persist_external_pool_adapter_atomic_activation_receipt_on,
    types::HistoricalExternalPoolAdapterAtomicActivationAuthority,
};

/// Uncommitted V277/V274 pair plus the still-live, fully consumed connection-local plan.
/// The caller must commit its outer transaction and immediately use the postcommit seam below.
pub(super) struct PendingExternalPoolAdapterAtomicActivationCommit<'runtime> {
    activation_receipt_id: String,
    activation_receipt_digest: String,
    activation_root_digest: String,
    v274_append: PendingExternalPoolAdapterProviderActiveSuccessorAppend<'runtime>,
    plan_guard: ExternalPoolAdapterAtomicActivationPendingPlanGuard,
}

/// Runs the exact 16 INSERT + 1 CAS UPDATE closure inside the caller's IMMEDIATE transaction.
/// The V274 purpose seal is minted before the pending V277 plan and every business write. The
/// sealed append never leaves this private kernel, so it cannot be inserted on another connection.
#[allow(clippy::too_many_arguments)]
pub(super) fn persist_external_pool_adapter_atomic_activation_closure_on<'tx, 'conn, 'runtime>(
    transaction: &'tx Transaction<'conn>,
    runtime: &'runtime ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
    active_successor_receipt_id: String,
    no_work: &ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject<'_, 'tx, 'conn>,
    transition: &PreparedExternalPoolAdapterCredentialProjectedActiveTransition<'_, 'tx, 'conn>,
    task_protocol: &PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier<'tx, 'conn>,
    source: &ComputeProviderRegistrationReceipt,
    target: &ComputeProvider,
    target_digest: &str,
    route: &AuthorizedComputeRouteAuthorization,
    receipt: &ExternalPoolAdapterAtomicActivationReceipt,
) -> Result<PendingExternalPoolAdapterAtomicActivationCommit<'runtime>> {
    validate_typed_sources(
        no_work,
        transition,
        task_protocol,
        source,
        target,
        target_digest,
        receipt,
    )?;
    validate_transition(transaction, source, target, target_digest, receipt)?;
    let successor = build_genesis_successor(no_work, transition, task_protocol, receipt)?;
    let v274_append = prepare_external_pool_adapter_provider_active_successor_genesis_append_on(
        transaction,
        runtime,
        active_successor_receipt_id,
        receipt,
        no_work,
        task_protocol,
        successor,
    )?;
    let plan = build_pending_plan(source, target, target_digest, route, receipt)?;
    let plan_guard =
        install_external_pool_adapter_atomic_activation_pending_plan_on(transaction, plan)?;

    persist_compute_route_authority_on(transaction, route)?;
    persist_provider_transition_on(transaction, source, target, target_digest, receipt)?;
    persist_external_pool_adapter_atomic_activation_receipt_on(transaction, receipt)?;
    plan_guard.ensure_fully_consumed()?;
    insert_prepared_external_pool_adapter_provider_active_successor_genesis_on(
        transaction,
        &v274_append,
    )?;

    audit_persisted_compute_route_authority_on(transaction, route)?;
    audit_provider_transition_on(transaction, target, target_digest)?;
    historical_external_pool_adapter_atomic_activation_authority_on(
        transaction,
        &receipt.activation_receipt_id,
        &receipt.activation_receipt_digest,
        &receipt.activation.identity.activation_root_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("V277/V274 pair is not visible before commit"))?;
    plan_guard.ensure_fully_consumed()?;

    Ok(PendingExternalPoolAdapterAtomicActivationCommit {
        activation_receipt_id: receipt.activation_receipt_id.clone(),
        activation_receipt_digest: receipt.activation_receipt_digest.clone(),
        activation_root_digest: receipt.activation.identity.activation_root_digest.clone(),
        v274_append,
        plan_guard,
    })
}

/// Same-connection postcommit readback, followed by V274 promotion and only then plan discard.
/// A failed readback never promotes V274; every failure still clears the plan via RAII.
pub(super) fn finalize_external_pool_adapter_atomic_activation_after_commit_on(
    connection: &Connection,
    pending: PendingExternalPoolAdapterAtomicActivationCommit<'_>,
) -> Result<(
    HistoricalExternalPoolAdapterAtomicActivationAuthority,
    CommittedExternalPoolAdapterProviderActiveSuccessorAppend,
)> {
    ensure!(
        connection.is_autocommit(),
        "V277 final readback requires a committed autocommit connection"
    );
    let PendingExternalPoolAdapterAtomicActivationCommit {
        activation_receipt_id,
        activation_receipt_digest,
        activation_root_digest,
        v274_append,
        plan_guard,
    } = pending;
    plan_guard.ensure_same_connection(connection)?;
    plan_guard.ensure_fully_consumed()?;
    let authority = historical_external_pool_adapter_atomic_activation_authority_on(
        connection,
        &activation_receipt_id,
        &activation_receipt_digest,
        &activation_root_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("committed V277/V274 pair is not visible on its connection"))?;
    let promoted = postcommit_external_pool_adapter_provider_active_successor_readback_on(
        connection,
        &plan_guard,
        v274_append,
    )?;
    plan_guard.discard()?;
    Ok((authority, promoted))
}

fn validate_typed_sources(
    no_work: &ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject<'_, '_, '_>,
    transition: &PreparedExternalPoolAdapterCredentialProjectedActiveTransition<'_, '_, '_>,
    task_protocol: &PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier<'_, '_>,
    source: &ComputeProviderRegistrationReceipt,
    target: &ComputeProvider,
    target_digest: &str,
    receipt: &ExternalPoolAdapterAtomicActivationReceipt,
) -> Result<()> {
    let activation = &receipt.activation;
    let v253 = &activation.v253_genesis_input;
    let renewable = &activation.renewable_evidence;
    let observation = no_work.observation();
    let credential = transition.credential().receipt();
    let observed_credential = observation.credential();
    let planned = no_work.preflight();
    let root = &planned.activation_root().activation_root;
    let executor = &activation.stable_executor;
    let id_material: ExternalPoolStableExecutorIdMaterial =
        serde_json::from_str(&executor.executor_id_material_json)?;
    let binding_material: ExternalPoolStableExecutorBindingMaterial =
        serde_json::from_str(&executor.executor_binding_material_json)?;
    let target_json = serde_json::to_string(planned.target())?;
    let planned_target_digest = {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(target_json.as_bytes()))
    };
    if transition.planned().activation_root() != planned.activation_root()
        || source != planned.source()
        || target != planned.target()
        || target_digest != planned_target_digest
        || id_material.service_actor_id != root.service_actor_id
        || id_material.task_production_carrier_policy_digest
            != root.task_production_carrier_policy_digest
        || binding_material.service_actor_id != root.service_actor_id
        || binding_material.task_production_carrier_policy_digest
            != root.task_production_carrier_policy_digest
        || binding_material.logical_projection_compatibility_digest
            != root.logical_projection_compatibility_digest
        || binding_material.lane_subject_digest != root.lane_subject_digest
        || credential != observed_credential
        || v253.registering_reattestation_receipt_id != credential.reattestation_receipt_id
        || v253.registering_reattestation_receipt_digest != credential.reattestation_receipt_digest
        || v253.projected_transition_proof_material_json != transition.proof_material_json()
        || v253.projected_transition_proof_digest != transition.proof_digest()
        || renewable.active_runtime_observation_id
            != observation.post_cleanup_observation_commitment()
        || renewable.observation_started_at != observation.probe_checked_at()
        || renewable.observation_completed_at != observation.checked_at()
        || renewable.observation_expires_at != observation.expires_at()
        || renewable.task_protocol_conformance_run_receipt_id
            != task_protocol.receipt().run_receipt_id
        || renewable.task_protocol_conformance_run_receipt_digest
            != task_protocol.receipt().run_receipt_digest
        || renewable.task_protocol_conformance_expires_at != task_protocol.receipt().run.expires_at
        || renewable.task_protocol_active_carrier_material_json != task_protocol.material_json()
        || renewable.task_protocol_active_carrier_digest != task_protocol.digest()
        || activation.evidence_checked_at != no_work.evidence_checked_at()
    {
        bail!("V277 receipt does not retain the exact typed no-work, V253, and V272 evidence");
    }
    Ok(())
}

fn build_genesis_successor(
    no_work: &ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject<'_, '_, '_>,
    transition: &PreparedExternalPoolAdapterCredentialProjectedActiveTransition<'_, '_, '_>,
    task_protocol: &PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier<'_, '_>,
    receipt: &ExternalPoolAdapterAtomicActivationReceipt,
) -> Result<ExternalPoolAdapterProviderActiveSuccessorMaterial> {
    let activation = &receipt.activation;
    let source = &activation.provider_transition.source_registering_provider;
    let target = &activation.provider_transition.target_active_provider;
    let observation = no_work.observation();
    let credential = transition.credential().receipt();
    let provider_evidence = |provider: &crate::compute_federation::external_pool_adapter_atomic_activation::ExternalPoolAdapterAtomicActivationProviderEvidence| {
        ExternalPoolAdapterProviderActiveSuccessorProviderEvidence {
            provider_id: provider.provider_id.clone(),
            provider_policy_revision: provider.provider_policy_revision,
            provider_json: provider.provider_json.clone(),
            provider_digest: provider.provider_digest.clone(),
        }
    };
    let mut runtime_observation = ExternalPoolAdapterProviderActiveSuccessorRuntimeObservation {
        runtime_observation_id: observation.post_cleanup_observation_commitment().into(),
        runtime_observation_digest: String::new(),
        observed_provider: provider_evidence(target),
        observation_started_at: observation.probe_checked_at().into(),
        observation_completed_at: observation.checked_at().into(),
        observation_expires_at: observation.expires_at().into(),
    };
    runtime_observation.runtime_observation_digest =
        provider_active_successor_runtime_observation_digest(&runtime_observation)?;
    let effects = ExternalPoolAdapterProviderActiveSuccessorEffects {
        credential_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
        adapter_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
        provider_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
        route_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
        activation_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
        execution_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
        usage_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
        market_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
        settlement_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
    };
    let readiness = ExternalPoolAdapterProviderActiveSuccessorReadiness {
        process_spawn_ready: false,
        ipc_session_ready: false,
        secret_delivery_ready: false,
        broker_connect_ready: false,
        upstream_probe_ready: false,
        runtime_launch_ready: false,
        route_ready: false,
        execution_ready: false,
        activation_ready: false,
    };
    Ok(ExternalPoolAdapterProviderActiveSuccessorMaterial {
        activation: no_work.preflight().activation_root().clone(),
        lineage: ExternalPoolAdapterProviderActiveSuccessorLineage {
            successor_sequence: 1,
            predecessor_active_successor_receipt_id: None,
            predecessor_active_successor_receipt_digest: None,
        },
        evidence_provider: provider_evidence(target),
        credential_evidence: ExternalPoolAdapterProviderActiveSuccessorCredentialEvidence {
            reattestation_receipt_id: credential.reattestation_receipt_id.clone(),
            reattestation_receipt_digest: credential.reattestation_receipt_digest.clone(),
            observed_provider: provider_evidence(source),
        },
        runtime_observation,
        task_protocol_evidence: ExternalPoolAdapterProviderActiveSuccessorTaskProtocolEvidence {
            task_protocol_conformance_run_receipt_id: task_protocol
                .receipt()
                .run_receipt_id
                .clone(),
            task_protocol_conformance_run_receipt_digest: task_protocol
                .receipt()
                .run_receipt_digest
                .clone(),
            task_protocol_conformance_expires_at: task_protocol.receipt().run.expires_at.clone(),
        },
        activation_witness: ExternalPoolAdapterProviderActiveSuccessorActivationWitness {
            activation_witness_id: receipt.activation_receipt_id.clone(),
            activation_witness_digest: receipt.activation_receipt_digest.clone(),
        },
        activation_target_updated_at: activation.activation_target_updated_at.clone(),
        evidence_checked_at: activation.evidence_checked_at.clone(),
        created_at: activation.created_at.clone(),
        effects,
        readiness,
    })
}

fn validate_transition(
    transaction: &Transaction<'_>,
    source: &ComputeProviderRegistrationReceipt,
    target: &ComputeProvider,
    target_digest: &str,
    receipt: &ExternalPoolAdapterAtomicActivationReceipt,
) -> Result<()> {
    validate_compute_provider_contract(target)?;
    let current = current_registered_provider_on(transaction, &source.provider.provider_id)?
        .ok_or_else(|| anyhow::anyhow!("V277 source Provider disappeared"))?;
    let target_json = serde_json::to_string(target)?;
    let transition = &receipt.activation.provider_transition;
    if current.provider != source.provider
        || current.provider_digest != source.provider_digest
        || source.provider.status != PROVIDER_STATUS_REGISTERING
        || target.status != PROVIDER_STATUS_ACTIVE
        || target.provider_id != source.provider.provider_id
        || target.provider_kind != source.provider.provider_kind
        || target.owner_account_id != source.provider.owner_account_id
        || target.created_at != source.provider.created_at
        || target.policy_revision != source.provider.policy_revision.checked_add(1).unwrap_or(0)
        || target.updated_at != receipt.activation.activation_target_updated_at
        || transition.source_registering_provider.provider_digest != source.provider_digest
        || transition.target_active_provider.provider_json != target_json
        || transition.target_active_provider.provider_digest != target_digest
    {
        bail!("V277 Provider transition is not the exact adjacent planned pair");
    }
    Ok(())
}

fn persist_provider_transition_on(
    transaction: &Transaction<'_>,
    source: &ComputeProviderRegistrationReceipt,
    target: &ComputeProvider,
    target_digest: &str,
    receipt: &ExternalPoolAdapterAtomicActivationReceipt,
) -> Result<()> {
    let target_json = serde_json::to_string(target)?;
    transaction.execute(
        "INSERT INTO compute_provider_versions (
             provider_id,policy_revision,provider_digest,provider_json,created_at
         ) VALUES (?1,?2,?3,?4,?5)",
        params![
            target.provider_id,
            target.policy_revision,
            target_digest,
            target_json,
            receipt.activation.evidence_checked_at,
        ],
    )?;
    let updated = transaction.execute(
        "UPDATE compute_providers
            SET settlement_account_id=?1,display_name=?2,status=?3,trust_tier=?4,
                home_region=?5,current_policy_revision=?6,current_provider_digest=?7,updated_at=?8
          WHERE provider_id=?9 AND provider_kind=?10 AND owner_account_id=?11
            AND status='registering' AND current_policy_revision=?12
            AND current_provider_digest=?13 AND created_at=?14 AND updated_at=?15",
        params![
            clean_optional(target.settlement_account_id.as_deref()),
            target.display_name,
            target.status,
            target.trust_tier,
            clean_optional(target.home_region.as_deref()),
            target.policy_revision,
            target_digest,
            target.updated_at,
            source.provider.provider_id,
            source.provider.provider_kind,
            source.provider.owner_account_id,
            source.provider.policy_revision,
            source.provider_digest,
            source.provider.created_at,
            source.provider.updated_at,
        ],
    )?;
    ensure!(updated == 1, "V277 Provider CAS lost its exact source pair");
    Ok(())
}

fn audit_provider_transition_on(
    transaction: &Transaction<'_>,
    target: &ComputeProvider,
    target_digest: &str,
) -> Result<()> {
    let stored = current_registered_provider_on(transaction, &target.provider_id)?
        .ok_or_else(|| anyhow::anyhow!("V277 active Provider disappeared after CAS"))?;
    ensure!(
        stored.provider == *target && stored.provider_digest == target_digest,
        "V277 active Provider readback is not exact"
    );
    Ok(())
}

fn clean_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
