use anyhow::{bail, Context, Result};
use rusqlite::{Row, Transaction};

use super::types::{
    PolicyPreparedWorkItem, PolicyPreparedWorkSet, StoredPolicyCapabilityRevocation,
    COMPUTE_PLUGIN_SHARING_POLICY_PREPARED_WORK_SET_SCHEMA,
};
use crate::node_agent_compute_plugin_host::local_authority::sharing_policy_binding::types::ProjectedSharingPolicyBinding;

pub(super) const MAX_PREPARED_FETCH_CLAIMS: usize = 4096;
pub(super) const MAX_PREPARED_VERIFICATIONS: usize = 4096;

pub(super) fn read_prepared_work_set(
    transaction: &Transaction<'_>,
    projected: &ProjectedSharingPolicyBinding,
) -> Result<(PolicyPreparedWorkSet, usize, usize)> {
    let fetch_items = read_fetch_items(
        transaction,
        "WHERE state = 'prepared' ORDER BY claim_id ASC LIMIT 4097",
        &[],
    )?;
    let verification_items = read_verification_items(
        transaction,
        "WHERE state = 'prepared' ORDER BY verification_id ASC LIMIT 4097",
        &[],
    )?;
    validate_prepared_bindings(&fetch_items, &verification_items, projected)?;
    let fetch_count = fetch_items.len();
    let verification_count = verification_items.len();
    Ok((
        PolicyPreparedWorkSet {
            schema: COMPUTE_PLUGIN_SHARING_POLICY_PREPARED_WORK_SET_SCHEMA.to_string(),
            items: fetch_items.into_iter().chain(verification_items).collect(),
        },
        fetch_count,
        verification_count,
    ))
}

pub(super) fn read_terminalized_work_set(
    transaction: &Transaction<'_>,
    stored: &StoredPolicyCapabilityRevocation,
) -> Result<PolicyPreparedWorkSet> {
    let receipt = &stored.hashed_receipt.receipt;
    let fetch_items = read_fetch_items(
        transaction,
        "WHERE state = 'aborted' AND resolved_at_ms = ?1 AND resolution_reason = ?2 ORDER BY claim_id ASC LIMIT 4097",
        &[&receipt.bound_at_ms, &receipt.fetch_resolution_reason],
    )?;
    let verification_items = read_verification_items(
        transaction,
        "WHERE state = 'aborted' AND resolved_at_ms = ?1 AND resolution_reason = ?2 AND result_json = ?3 AND result_digest = ?4 ORDER BY verification_id ASC LIMIT 4097",
        &[
            &receipt.bound_at_ms,
            &receipt.verification_resolution_reason,
            &stored.verification_result_json,
            &receipt.verification_result_digest,
        ],
    )?;
    Ok(PolicyPreparedWorkSet {
        schema: COMPUTE_PLUGIN_SHARING_POLICY_PREPARED_WORK_SET_SCHEMA.to_string(),
        items: fetch_items.into_iter().chain(verification_items).collect(),
    })
}

fn validate_prepared_bindings(
    fetch_items: &[PolicyPreparedWorkItem],
    verification_items: &[PolicyPreparedWorkItem],
    projected: &ProjectedSharingPolicyBinding,
) -> Result<()> {
    if fetch_items.len() > MAX_PREPARED_FETCH_CLAIMS
        || verification_items.len() > MAX_PREPARED_VERIFICATIONS
    {
        bail!("COMPUTE_PLUGIN_POLICY_REVOCATION_WORK_SET_TOO_LARGE");
    }
    for item in fetch_items.iter().chain(verification_items) {
        let (authority_epoch, process_owner_epoch, state_revision, prepared_at_ms) = match item {
            PolicyPreparedWorkItem::FetchClaim {
                authority_epoch,
                process_owner_epoch,
                prepared_at_ms,
                ..
            } => (
                *authority_epoch,
                *process_owner_epoch,
                None,
                *prepared_at_ms,
            ),
            PolicyPreparedWorkItem::CandidateVerification {
                authority_epoch,
                process_owner_epoch,
                authority_state_revision,
                prepared_at_ms,
                ..
            } => (
                *authority_epoch,
                *process_owner_epoch,
                Some(*authority_state_revision),
                *prepared_at_ms,
            ),
        };
        if authority_epoch != projected.before.authority_epoch
            || process_owner_epoch != projected.before.process_owner_epoch
            || state_revision.is_some_and(|revision| revision != projected.before.state_revision)
            || prepared_at_ms > projected.before.trusted_time_high_water_ms
        {
            bail!("COMPUTE_PLUGIN_POLICY_REVOCATION_WORK_BINDING_CHANGED");
        }
    }
    Ok(())
}

fn read_fetch_items(
    transaction: &Transaction<'_>,
    predicate: &str,
    values: &[&dyn rusqlite::ToSql],
) -> Result<Vec<PolicyPreparedWorkItem>> {
    let sql = format!(
        "SELECT claim_id, plan_id, plan_digest, ordinal, candidate_token, authority_epoch, \
         process_owner_epoch, cursor_generation, redirect_generation, offset_bytes, length_bytes, \
         end_offset_bytes, prepared_at_ms FROM fetch_claims {predicate}"
    );
    let mut statement = transaction
        .prepare(&sql)
        .context("COMPUTE_PLUGIN_POLICY_REVOCATION_FETCH_PREPARE")?;
    let rows = statement
        .query_map(values, map_fetch_item)
        .context("COMPUTE_PLUGIN_POLICY_REVOCATION_FETCH_READ")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("COMPUTE_PLUGIN_POLICY_REVOCATION_FETCH_ROWS")?;
    Ok(rows)
}

fn map_fetch_item(row: &Row<'_>) -> rusqlite::Result<PolicyPreparedWorkItem> {
    Ok(PolicyPreparedWorkItem::FetchClaim {
        claim_id: row.get(0)?,
        plan_id: row.get(1)?,
        plan_digest: row.get(2)?,
        ordinal: row.get(3)?,
        candidate_token: row.get(4)?,
        authority_epoch: row.get(5)?,
        process_owner_epoch: row.get(6)?,
        cursor_generation: row.get(7)?,
        redirect_generation: row.get(8)?,
        offset_bytes: row.get(9)?,
        length_bytes: row.get(10)?,
        end_offset_bytes: row.get(11)?,
        prepared_at_ms: row.get(12)?,
    })
}

fn read_verification_items(
    transaction: &Transaction<'_>,
    predicate: &str,
    values: &[&dyn rusqlite::ToSql],
) -> Result<Vec<PolicyPreparedWorkItem>> {
    let sql = format!(
        "SELECT verification_id, candidate_token, owner_plan_id, owner_plan_digest, \
         verification_generation, candidate_generation, application_inventory_revision, \
         authority_state_revision, authority_epoch, process_owner_epoch, artifact_count, \
         artifact_bytes, expected_artifact_set_digest, file_set_binding_digest, prepared_at_ms \
         FROM candidate_verification_runs {predicate}"
    );
    let mut statement = transaction
        .prepare(&sql)
        .context("COMPUTE_PLUGIN_POLICY_REVOCATION_VERIFICATION_PREPARE")?;
    let rows = statement
        .query_map(values, map_verification_item)
        .context("COMPUTE_PLUGIN_POLICY_REVOCATION_VERIFICATION_READ")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("COMPUTE_PLUGIN_POLICY_REVOCATION_VERIFICATION_ROWS")?;
    Ok(rows)
}

fn map_verification_item(row: &Row<'_>) -> rusqlite::Result<PolicyPreparedWorkItem> {
    Ok(PolicyPreparedWorkItem::CandidateVerification {
        verification_id: row.get(0)?,
        candidate_token: row.get(1)?,
        owner_plan_id: row.get(2)?,
        owner_plan_digest: row.get(3)?,
        verification_generation: row.get(4)?,
        candidate_generation: row.get(5)?,
        application_inventory_revision: row.get(6)?,
        authority_state_revision: row.get(7)?,
        authority_epoch: row.get(8)?,
        process_owner_epoch: row.get(9)?,
        artifact_count: row.get(10)?,
        artifact_bytes: row.get(11)?,
        expected_artifact_set_digest: row.get(12)?,
        file_set_binding_digest: row.get(13)?,
        prepared_at_ms: row.get(14)?,
    })
}
