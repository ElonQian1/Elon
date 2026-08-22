//! Durable candidate selection; workers provide no receipt or authority identity.

use anyhow::{bail, ensure, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::store::{
    compute_external_pool_adapter_installation::external_pool_adapter_installation_receipt_authority_on,
    compute_external_pool_adapter_provider_active_successor::historical_external_pool_adapter_atomic_activation_history_for_binding_on,
    compute_external_pool_adapter_upstream_transport_target::historical_external_pool_adapter_upstream_transport_target_authority_on,
};

use super::types::{
    ExternalPoolAdapterActivePreparationCandidate, ExternalPoolAdapterActivePreparationIdentity,
};

pub(super) fn select_external_pool_adapter_active_preparation_candidate_on(
    transaction: &Transaction<'_>,
    provider_id: Option<&str>,
    selection_slot: u64,
) -> Result<Option<ExternalPoolAdapterActivePreparationCandidate>> {
    let selected = if let Some(provider_id) = provider_id {
        transaction
            .query_row(
                &format!("{CANDIDATE_SQL} AND a.target_active_provider_id=?1 ORDER BY a.created_at,a.activation_receipt_id LIMIT 1"),
                params![provider_id],
                candidate_row,
            )
            .optional()?
    } else {
        let candidate_count: i64 = transaction.query_row(
            &format!("SELECT COUNT(*) FROM ({CANDIDATE_SQL})"),
            [],
            |row| row.get(0),
        )?;
        if candidate_count == 0 {
            return Ok(None);
        }
        ensure!(
            candidate_count > 0,
            "active preparation candidate count is invalid"
        );
        let offset = i64::try_from(selection_slot % u64::try_from(candidate_count)?)?;
        transaction
            .query_row(
                &format!("{CANDIDATE_SQL} ORDER BY a.created_at,a.activation_receipt_id LIMIT 1 OFFSET ?1"),
                params![offset],
                candidate_row,
            )
            .optional()?
    };
    let Some((
        activation_id,
        activation_digest,
        root_digest,
        binding_id,
        genesis_id,
        genesis_digest,
    )) = selected
    else {
        return Ok(None);
    };
    let activation = historical_external_pool_adapter_atomic_activation_history_for_binding_on(
        transaction,
        &binding_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("active preparation candidate lost V277 history"))?;
    let root = &activation.activation_root().activation_root;
    ensure!(
        activation.receipt().activation_receipt_id == activation_id
            && activation.receipt().activation_receipt_digest == activation_digest
            && activation.genesis().active_successor_receipt_id == genesis_id
            && activation.genesis().receipt_digest == genesis_digest
            && root.provider_binding_id == binding_id
            && activation.activation_root().activation_root_digest == root_digest,
        "active preparation candidate roots changed during historical audit"
    );
    let installation = external_pool_adapter_installation_receipt_authority_on(
        transaction,
        &root.installation_receipt_id,
        &root.installation_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("active preparation candidate lost installation history"))?;
    let target = historical_external_pool_adapter_upstream_transport_target_authority_on(
        transaction,
        &root.target_id,
        &root.target_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("active preparation candidate lost V258 target"))?;
    let installation_binding = &installation.receipt().installation.binding;
    if target.target.provider_binding_id != root.provider_binding_id
        || target.target.provider_binding_digest != root.provider_binding_digest
        || installation_binding.provider_id != root.provider_id
        || installation_binding.provider_owner_account_id != root.provider_owner_account_id
        || installation_binding.provider_policy_revision
            != root.source_registering_provider_policy_revision
        || installation_binding.provider_digest != root.source_registering_provider_digest
        || installation_binding.installation_content_digest != root.installation_content_digest
        || installation_binding.adapter_id != root.logical_adapter_id
    {
        bail!("active preparation candidate target or installation roots drifted");
    }
    Ok(Some(ExternalPoolAdapterActivePreparationCandidate {
        identity: ExternalPoolAdapterActivePreparationIdentity {
            provider_id: activation.active_provider().provider_id.clone(),
            provider_binding_id: root.provider_binding_id.clone(),
            activation_root_digest: activation.activation_root().activation_root_digest.clone(),
        },
        activation_receipt_id: activation_id,
        activation_receipt_digest: activation_digest,
        activation_genesis_successor_receipt_id: genesis_id,
        activation_genesis_successor_receipt_digest: genesis_digest,
        installation_binding: installation.receipt().installation.binding.clone(),
        target,
    }))
}

fn candidate_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<(String, String, String, String, String, String)> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

const CANDIDATE_SQL: &str = "
SELECT a.activation_receipt_id,a.activation_receipt_digest,a.activation_root_digest,
       a.provider_binding_id,g.active_successor_receipt_id,g.receipt_digest
  FROM compute_external_pool_adapter_atomic_activation_receipts a
  JOIN compute_external_pool_adapter_provider_active_successor_receipts g
    ON g.activation_witness_id=a.activation_receipt_id
   AND g.activation_witness_digest=a.activation_receipt_digest
   AND g.activation_root_digest=a.activation_root_digest
   AND g.successor_sequence=1
 WHERE NOT EXISTS(
       SELECT 1 FROM compute_external_pool_adapter_provider_active_successor_revocations r
        WHERE r.active_successor_receipt_id=g.active_successor_receipt_id)";
