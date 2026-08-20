//! V274 sequence-one append used only inside the V277 atomic activation transaction.

use anyhow::{bail, Result};
use rusqlite::Transaction;

use crate::{
    compute_federation::{
        external_pool_adapter_atomic_activation::{
            ExternalPoolAdapterAtomicActivationProviderEvidence,
            ExternalPoolAdapterAtomicActivationReceipt,
        },
        external_pool_adapter_provider_active_successor::{
            ExternalPoolAdapterProviderActiveSuccessorMaterial,
            ExternalPoolAdapterProviderActiveSuccessorProviderEvidence,
        },
    },
    store::{
        compute_external_pool_adapter_runtime_bundle::{
            ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
            ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject,
        },
        compute_external_pool_adapter_task_protocol_conformance::PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier,
    },
};

use super::super::read::head_by_binding_and_root_on;
use super::{
    material::{prepare_pending_append, PendingExternalPoolAdapterProviderActiveSuccessorAppend},
    readback::insert_and_readback_pending_append_on,
};

pub(in crate::store::compute_external_pool_adapter_provider_active_successor) fn prepare_external_pool_adapter_provider_active_successor_genesis_append_on<
    'tx,
    'conn,
    'runtime,
>(
    transaction: &'tx Transaction<'conn>,
    runtime: &'runtime ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
    active_successor_receipt_id: String,
    activation_receipt: &ExternalPoolAdapterAtomicActivationReceipt,
    no_work: &ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject<'_, 'tx, 'conn>,
    task_protocol: &PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier<'tx, 'conn>,
    successor: ExternalPoolAdapterProviderActiveSuccessorMaterial,
) -> Result<PendingExternalPoolAdapterProviderActiveSuccessorAppend<'runtime>> {
    let lineage = &successor.lineage;
    let activation = &successor.activation;
    if lineage.successor_sequence != 1
        || lineage.predecessor_active_successor_receipt_id.is_some()
        || lineage
            .predecessor_active_successor_receipt_digest
            .is_some()
    {
        bail!("V274 genesis must be exact sequence one without a predecessor");
    }
    if head_by_binding_and_root_on(
        transaction,
        &activation.activation_root.provider_binding_id,
        &activation.activation_root_digest,
    )?
    .is_some()
    {
        bail!("V274 genesis root already has durable history");
    }
    require_exact_genesis_sources(&successor, activation_receipt, no_work, task_protocol)?;
    prepare_pending_append(runtime, active_successor_receipt_id, successor)
}

pub(in crate::store::compute_external_pool_adapter_provider_active_successor) fn insert_prepared_external_pool_adapter_provider_active_successor_genesis_on(
    transaction: &Transaction<'_>,
    pending: &PendingExternalPoolAdapterProviderActiveSuccessorAppend<'_>,
) -> Result<()> {
    insert_and_readback_pending_append_on(transaction, pending)
}

fn require_exact_genesis_sources(
    successor: &ExternalPoolAdapterProviderActiveSuccessorMaterial,
    activation_receipt: &ExternalPoolAdapterAtomicActivationReceipt,
    no_work: &ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject<'_, '_, '_>,
    task_protocol: &PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier<'_, '_>,
) -> Result<()> {
    let activation = &activation_receipt.activation;
    let identity = &activation.identity;
    let transition = &activation.provider_transition;
    let renewable = &activation.renewable_evidence;
    let credential = &successor.credential_evidence;
    let runtime = &successor.runtime_observation;
    let task = &successor.task_protocol_evidence;
    let observation = no_work.observation();
    let fresh_expires_at = task_protocol.fresh_expires_at_for(no_work)?;
    let planned = no_work.preflight();
    let final_target = planned.target();
    let final_target_json = serde_json::to_string(final_target)?;
    let final_target_digest = {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(final_target_json.as_bytes()))
    };
    if !observation.no_work_observed()
        || !observation.authenticated_shutdown_completed()
        || !observation.pidfd_reaped()
        || !observation.cgroup_cleaned()
        || !observation.scratch_cleaned()
        || observation.checked_at() != no_work.evidence_checked_at()
        || planned.activation_root() != &successor.activation
        || final_target.provider_id != transition.target_active_provider.provider_id
        || final_target.policy_revision
            != transition.target_active_provider.provider_policy_revision
        || final_target_json != transition.target_active_provider.provider_json
        || final_target_digest != transition.target_active_provider.provider_digest
        || successor.activation.activation_root.provider_binding_id != identity.provider_binding_id
        || successor.activation.activation_root.provider_binding_digest
            != identity.provider_binding_digest
        || successor.activation.activation_root_digest != identity.activation_root_digest
        || successor.activation_witness.activation_witness_id
            != activation_receipt.activation_receipt_id
        || successor.activation_witness.activation_witness_digest
            != activation_receipt.activation_receipt_digest
        || !same_provider_evidence(
            &successor.evidence_provider,
            &transition.target_active_provider,
        )
        || credential.reattestation_receipt_id
            != activation
                .v253_genesis_input
                .registering_reattestation_receipt_id
        || credential.reattestation_receipt_digest
            != activation
                .v253_genesis_input
                .registering_reattestation_receipt_digest
        || !same_provider_evidence(
            &credential.observed_provider,
            &transition.source_registering_provider,
        )
        || runtime.runtime_observation_id != renewable.active_runtime_observation_id
        || runtime.runtime_observation_digest != renewable.active_runtime_observation_digest
        || runtime.runtime_observation_id != observation.post_cleanup_observation_commitment()
        || !same_provider_evidence(
            &runtime.observed_provider,
            &transition.target_active_provider,
        )
        || runtime.observation_started_at != renewable.observation_started_at
        || runtime.observation_completed_at != renewable.observation_completed_at
        || runtime.observation_expires_at != renewable.observation_expires_at
        || runtime.observation_started_at != observation.probe_checked_at()
        || runtime.observation_completed_at != observation.checked_at()
        || runtime.observation_expires_at != fresh_expires_at
        || task.task_protocol_conformance_run_receipt_id != task_protocol.receipt().run_receipt_id
        || task.task_protocol_conformance_run_receipt_digest
            != task_protocol.receipt().run_receipt_digest
        || task.task_protocol_conformance_expires_at != task_protocol.receipt().run.expires_at
        || task.task_protocol_conformance_run_receipt_id
            != renewable.task_protocol_conformance_run_receipt_id
        || task.task_protocol_conformance_run_receipt_digest
            != renewable.task_protocol_conformance_run_receipt_digest
        || task.task_protocol_conformance_expires_at
            != renewable.task_protocol_conformance_expires_at
        || task_protocol.material_json() != renewable.task_protocol_active_carrier_material_json
        || task_protocol.digest() != renewable.task_protocol_active_carrier_digest
        || successor.activation_target_updated_at != activation.activation_target_updated_at
        || successor.evidence_checked_at != activation.evidence_checked_at
        || successor.created_at != activation.created_at
    {
        bail!("V274 genesis is not exact for its typed V277/V272 sources");
    }
    Ok(())
}

fn same_provider_evidence(
    successor: &ExternalPoolAdapterProviderActiveSuccessorProviderEvidence,
    activation: &ExternalPoolAdapterAtomicActivationProviderEvidence,
) -> bool {
    successor.provider_id == activation.provider_id
        && successor.provider_policy_revision == activation.provider_policy_revision
        && successor.provider_json == activation.provider_json
        && successor.provider_digest == activation.provider_digest
}
