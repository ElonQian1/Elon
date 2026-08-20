//! Exact 17-column V278 pending-plan material for a V274 refresh INSERT.

use anyhow::{anyhow, Result};
use rusqlite::types::Value;

use crate::store::compute_external_pool_adapter_runtime_bundle::{
    ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlan,
    ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlanInput,
};

use super::material::PendingExternalPoolAdapterProviderActiveSuccessorAppend;

pub(super) fn pending_plan_for_external_pool_adapter_provider_active_successor_refresh(
    pending: &PendingExternalPoolAdapterProviderActiveSuccessorAppend<'_>,
) -> Result<ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlan> {
    let receipt = pending.receipt();
    let successor = &receipt.successor;
    let predecessor_id = successor
        .lineage
        .predecessor_active_successor_receipt_id
        .as_ref()
        .ok_or_else(|| anyhow!("V274 refresh pending plan lacks predecessor id"))?;
    let predecessor_digest = successor
        .lineage
        .predecessor_active_successor_receipt_digest
        .as_ref()
        .ok_or_else(|| anyhow!("V274 refresh pending plan lacks predecessor digest"))?;
    let sequence = i64::try_from(successor.lineage.successor_sequence)
        .map_err(|_| anyhow!("V274 refresh sequence exceeds SQLite INTEGER"))?;
    let input = ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlanInput::new(
        Value::Text("provider_active_successor_refresh".into()),
        Value::Text(receipt.active_successor_receipt_id.clone()),
        Value::Text(receipt.receipt_digest.clone()),
        Value::Text(pending.receipt_json.clone()),
        Value::Text(
            successor
                .activation
                .activation_root
                .provider_binding_id
                .clone(),
        ),
        Value::Text(successor.activation.activation_root_digest.clone()),
        Value::Integer(sequence),
        Value::Text(predecessor_id.clone()),
        Value::Text(predecessor_digest.clone()),
        Value::Text(successor.activation_target_updated_at.clone()),
        Value::Text(successor.evidence_checked_at.clone()),
        Value::Text(successor.created_at.clone()),
        Value::Text(successor.runtime_observation.observation_expires_at.clone()),
        Value::Text(pending.process_custody.process_custody_epoch_digest.clone()),
        Value::Text(pending.process_custody.process_custody_nonce_digest.clone()),
        Value::Text(pending.process_custody.process_custody_seal_digest.clone()),
        Value::Text(pending.receipt_integrity_digest().into()),
    )?;
    ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlan::new(input)
}
