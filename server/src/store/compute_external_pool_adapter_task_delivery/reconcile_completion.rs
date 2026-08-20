//! Exact claimed-to-delivery-observed reconcile poll transition.

use anyhow::{ensure, Result};
use rusqlite::{params, types::Value, Connection};

use crate::store::hash_token;

use super::{
    read::read_reconcile_poll_on,
    types::{
        ExternalPoolAdapterTaskPollClaim, CLAIM_STATUS_CLAIMED, CLAIM_STATUS_DELIVERY_OBSERVED,
    },
};

pub(super) fn reconcile_poll_cas_values(
    claim: &ExternalPoolAdapterTaskPollClaim,
    _completed_at: &str,
) -> Result<Vec<Value>> {
    let next_revision = claim
        .claim_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("V278 reconcile completion revision overflow"))?;
    Ok(vec![
        Value::Text(claim.poll_id.clone()),
        Value::Text(claim.poll_digest.clone()),
        Value::Text(CLAIM_STATUS_CLAIMED.to_string()),
        Value::Text(CLAIM_STATUS_DELIVERY_OBSERVED.to_string()),
        Value::Integer(i64::try_from(claim.claim_revision)?),
        Value::Integer(i64::try_from(next_revision)?),
        Value::Integer(i64::try_from(claim.claim_generation)?),
        Value::Integer(i64::try_from(claim.claim_generation)?),
        Value::Text(claim.claim_owner_id.clone()),
        Value::Null,
        Value::Text(hash_token(&claim.raw_claim_token)),
        Value::Null,
        Value::Text(claim.claim_expires_at.clone()),
        Value::Null,
    ])
}

pub(super) fn complete_reconcile_poll_on(
    connection: &Connection,
    claim: &ExternalPoolAdapterTaskPollClaim,
    completed_at: &str,
) -> Result<()> {
    let next_revision = claim
        .claim_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("V278 reconcile completion revision overflow"))?;
    let before = read_reconcile_poll_on(connection, &claim.poll_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 reconcile poll disappeared before completion"))?;
    let token_digest = hash_token(&claim.raw_claim_token);
    ensure!(
        before.claim.status == CLAIM_STATUS_CLAIMED
            && before.claim.revision == claim.claim_revision
            && before.claim.generation == claim.claim_generation
            && before.claim.owner_id.as_deref() == Some(claim.claim_owner_id.as_str())
            && before.claim.token_digest.as_deref() == Some(token_digest.as_str())
            && before.claim.expires_at.as_deref() == Some(claim.claim_expires_at.as_str())
            && completed_at < claim.claim_expires_at.as_str()
            && completed_at < before.envelope.poll.not_after.as_str(),
        "V278 reconcile poll completion claim is stale"
    );
    let changed = connection.execute(
        "UPDATE compute_external_pool_adapter_task_reconcile_polls
            SET claim_status='delivery_observed',claim_revision=claim_revision+1,
                claim_owner_id=NULL,claim_token_digest=NULL,claim_expires_at=NULL
          WHERE reconcile_poll_id=?1 AND reconcile_poll_digest=?2
            AND claim_status='claimed' AND claim_revision=?3 AND claim_generation=?4
            AND claim_owner_id=?5 AND claim_token_digest=?6 AND claim_expires_at=?7
            AND ?8<claim_expires_at
            AND EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts receipt
                         WHERE receipt.source_kind='reconcile_poll'
                           AND receipt.source_id=?1 AND receipt.source_digest=?2)",
        params![
            claim.poll_id,
            claim.poll_digest,
            i64::try_from(claim.claim_revision)?,
            i64::try_from(claim.claim_generation)?,
            claim.claim_owner_id,
            token_digest,
            claim.claim_expires_at,
            completed_at,
        ],
    )?;
    ensure!(changed == 1, "V278 reconcile poll completion CAS lost");
    let after = read_reconcile_poll_on(connection, &claim.poll_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 reconcile poll disappeared after completion"))?;
    ensure!(
        after.claim.status == CLAIM_STATUS_DELIVERY_OBSERVED
            && after.claim.revision == next_revision
            && after.claim.generation == claim.claim_generation
            && after.claim.owner_id.is_none()
            && after.claim.token_digest.is_none()
            && after.claim.expires_at.is_none(),
        "V278 reconcile poll completion readback is not exact"
    );
    Ok(())
}
