//! Fresh V274 successor append; it never requires the predecessor to retain process currentness.

use anyhow::{bail, Result};
use rusqlite::Transaction;

use crate::{
    compute_federation::{
        external_pool_adapter_provider_active_successor::{
            ExternalPoolAdapterProviderActiveSuccessorMaterial,
            ExternalPoolAdapterProviderActiveSuccessorProviderEvidence,
        },
        provider::ComputeProvider,
    },
    store::{
        compute_external_pool_adapter_runtime_bundle::{
            install_external_pool_adapter_provider_active_successor_refresh_pending_plan_on,
            ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlanGuard,
            ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
        },
        compute_external_pool_adapter_task_protocol_conformance::CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority,
    },
};

use super::super::read::head_by_binding_and_root_on;
use super::{
    material::{prepare_pending_append, PendingExternalPoolAdapterProviderActiveSuccessorAppend},
    readback::insert_and_readback_pending_append_on,
    refresh_pending_plan::pending_plan_for_external_pool_adapter_provider_active_successor_refresh,
};

/// Uncommitted refresh append plus its dedicated, fully consumed connection-local plan.
pub(in crate::store) struct PendingExternalPoolAdapterProviderActiveSuccessorRefresh<'runtime> {
    pub(super) append: PendingExternalPoolAdapterProviderActiveSuccessorAppend<'runtime>,
    pub(super) plan_guard: ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlanGuard,
}

pub(in crate::store) fn append_external_pool_adapter_provider_active_successor_refresh_on<
    'tx,
    'conn,
    'runtime,
>(
    transaction: &'tx Transaction<'conn>,
    runtime: &'runtime ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
    active_successor_receipt_id: String,
    task_protocol: &CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority<'tx, 'conn>,
    successor: ExternalPoolAdapterProviderActiveSuccessorMaterial,
) -> Result<PendingExternalPoolAdapterProviderActiveSuccessorRefresh<'runtime>> {
    if successor.lineage.successor_sequence <= 1 {
        bail!("V274 refresh must have a successor sequence greater than one");
    }
    let activation = &successor.activation;
    let head = head_by_binding_and_root_on(
        transaction,
        &activation.activation_root.provider_binding_id,
        &activation.activation_root_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("V274 refresh lacks an exact historical predecessor"))?;
    let expected_sequence = head
        .receipt
        .successor
        .lineage
        .successor_sequence
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("V274 successor sequence overflow"))?;
    if successor.lineage.successor_sequence != expected_sequence
        || successor
            .lineage
            .predecessor_active_successor_receipt_id
            .as_deref()
            != Some(head.receipt.active_successor_receipt_id.as_str())
        || successor
            .lineage
            .predecessor_active_successor_receipt_digest
            .as_deref()
            != Some(head.receipt.receipt_digest.as_str())
        || successor.activation != head.receipt.successor.activation
        || successor.activation_witness != head.receipt.successor.activation_witness
    {
        bail!("V274 refresh does not extend the exact single historical head");
    }
    require_exact_refresh_sources(&successor, task_protocol)?;
    let pending = prepare_pending_append(runtime, active_successor_receipt_id, successor)?;
    let plan = pending_plan_for_external_pool_adapter_provider_active_successor_refresh(&pending)?;
    let plan_guard =
        install_external_pool_adapter_provider_active_successor_refresh_pending_plan_on(
            transaction,
            plan,
        )?;
    insert_and_readback_pending_append_on(transaction, &pending)?;
    plan_guard.ensure_fully_consumed()?;
    Ok(PendingExternalPoolAdapterProviderActiveSuccessorRefresh {
        append: pending,
        plan_guard,
    })
}

fn require_exact_refresh_sources(
    successor: &ExternalPoolAdapterProviderActiveSuccessorMaterial,
    task_protocol: &CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority<'_, '_>,
) -> Result<()> {
    let carrier = task_protocol.carrier();
    let historical = carrier.historical_activation();
    let activation_receipt = historical.receipt();
    let credential_receipt = carrier.credential().receipt();
    let task_receipt = task_protocol.receipt();
    if &successor.activation != historical.activation_root()
        || successor.activation_witness.activation_witness_id
            != activation_receipt.activation_receipt_id
        || successor.activation_witness.activation_witness_digest
            != activation_receipt.activation_receipt_digest
        || !same_live_provider(&successor.evidence_provider, historical.active_provider())?
        || successor.credential_evidence.reattestation_receipt_id
            != credential_receipt.reattestation_receipt_id
        || successor.credential_evidence.reattestation_receipt_digest
            != credential_receipt.reattestation_receipt_digest
        || !same_live_provider(
            &successor.credential_evidence.observed_provider,
            historical.active_provider(),
        )?
        || !same_live_provider(
            &successor.runtime_observation.observed_provider,
            historical.active_provider(),
        )?
        || successor
            .task_protocol_evidence
            .task_protocol_conformance_run_receipt_id
            != task_receipt.run_receipt_id
        || successor
            .task_protocol_evidence
            .task_protocol_conformance_run_receipt_digest
            != task_receipt.run_receipt_digest
        || successor
            .task_protocol_evidence
            .task_protocol_conformance_expires_at
            != task_receipt.run.expires_at
        || successor.activation_target_updated_at
            != activation_receipt.activation.activation_target_updated_at
        || successor.evidence_checked_at != task_protocol.checked_at()
        || successor.created_at != task_protocol.checked_at()
    {
        bail!("V274 refresh is not exact for its typed projected-active V272 source");
    }
    Ok(())
}

fn same_live_provider(
    evidence: &ExternalPoolAdapterProviderActiveSuccessorProviderEvidence,
    provider: &ComputeProvider,
) -> Result<bool> {
    let provider_json = serde_json::to_string(provider)?;
    let provider_digest = {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(provider_json.as_bytes()))
    };
    Ok(evidence.provider_id == provider.provider_id
        && evidence.provider_policy_revision == provider.policy_revision
        && evidence.provider_json == provider_json
        && evidence.provider_digest == provider_digest)
}
