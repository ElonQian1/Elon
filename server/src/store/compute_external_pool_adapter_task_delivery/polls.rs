use anyhow::{ensure, Result};
use chrono::{DateTime, FixedOffset, SecondsFormat};
use rusqlite::{params, Connection};

use crate::store::{hash_token, new_id};

use super::{
    read::{read_event_poll_on, read_reconcile_poll_on},
    types::{ExternalPoolAdapterTaskPollClaim, CLAIM_STATUS_CLAIMED, CLAIM_STATUS_PENDING},
};

#[allow(dead_code)]
pub(in crate::store) fn try_claim_reconcile_poll_on(
    conn: &Connection,
    poll_id: &str,
    poll_digest: &str,
    expected_revision: u64,
    expected_generation: u64,
    claim_owner_id: &str,
    claim_expires_at: &str,
) -> Result<Option<ExternalPoolAdapterTaskPollClaim>> {
    validate_claim_input(claim_owner_id, claim_expires_at)?;
    let before = read_reconcile_poll_on(conn, poll_id)?
        .ok_or_else(|| anyhow::anyhow!("V273 reconcile poll disappeared before claim"))?;
    ensure!(
        before.envelope.reconcile_poll_digest == poll_digest
            && before.claim.status == CLAIM_STATUS_PENDING
            && before.claim.revision == expected_revision
            && before.claim.generation == expected_generation,
        "V273 reconcile poll claim expectation is stale"
    );
    let raw_claim_token = new_id("v273_reconcile_claim");
    let claim_token_digest = hash_token(&raw_claim_token);
    let changed = conn.execute(
        "UPDATE compute_external_pool_adapter_task_reconcile_polls
            SET claim_status='claimed',claim_revision=claim_revision+1,
                claim_generation=claim_generation+1,claim_owner_id=?1,
                claim_token_digest=?2,claim_expires_at=?3
          WHERE reconcile_poll_id=?4 AND reconcile_poll_digest=?5
            AND claim_status='pending' AND claim_revision=?6 AND claim_generation=?7
            AND not_before<=strftime('%Y-%m-%dT%H:%M:%f000000Z','now')
            AND strftime('%Y-%m-%dT%H:%M:%f000000Z','now')<not_after
            AND strftime('%Y-%m-%dT%H:%M:%f000000Z','now')<?3 AND ?3<=not_after",
        params![
            claim_owner_id,
            claim_token_digest,
            claim_expires_at,
            poll_id,
            poll_digest,
            i64::try_from(expected_revision)?,
            i64::try_from(expected_generation)?,
        ],
    )?;
    if changed == 0 {
        return Ok(None);
    }
    ensure!(
        changed == 1,
        "V273 reconcile poll claim changed multiple rows"
    );
    let after = read_reconcile_poll_on(conn, poll_id)?
        .ok_or_else(|| anyhow::anyhow!("V273 claimed reconcile poll disappeared"))?;
    finish_claim(
        poll_id,
        poll_digest,
        expected_revision,
        expected_generation,
        claim_owner_id,
        raw_claim_token,
        claim_token_digest,
        claim_expires_at,
        after.envelope.reconcile_poll_digest,
        after.claim,
    )
    .map(Some)
}

#[allow(dead_code)]
pub(in crate::store) fn try_claim_event_poll_on(
    conn: &Connection,
    poll_id: &str,
    poll_digest: &str,
    expected_revision: u64,
    expected_generation: u64,
    claim_owner_id: &str,
    claim_expires_at: &str,
) -> Result<Option<ExternalPoolAdapterTaskPollClaim>> {
    validate_claim_input(claim_owner_id, claim_expires_at)?;
    let before = read_event_poll_on(conn, poll_id)?
        .ok_or_else(|| anyhow::anyhow!("V273 event poll disappeared before claim"))?;
    ensure!(
        before.envelope.event_poll_digest == poll_digest
            && before.claim.status == CLAIM_STATUS_PENDING
            && before.claim.revision == expected_revision
            && before.claim.generation == expected_generation,
        "V273 event poll claim expectation is stale"
    );
    let raw_claim_token = new_id("v273_event_claim");
    let claim_token_digest = hash_token(&raw_claim_token);
    let changed = conn.execute(
        "UPDATE compute_external_pool_adapter_task_event_polls
            SET claim_status='claimed',claim_revision=claim_revision+1,
                claim_generation=claim_generation+1,claim_owner_id=?1,
                claim_token_digest=?2,claim_expires_at=?3
          WHERE event_poll_id=?4 AND event_poll_digest=?5
            AND claim_status='pending' AND claim_revision=?6 AND claim_generation=?7
            AND not_before<=strftime('%Y-%m-%dT%H:%M:%f000000Z','now')
            AND strftime('%Y-%m-%dT%H:%M:%f000000Z','now')<not_after
            AND strftime('%Y-%m-%dT%H:%M:%f000000Z','now')<?3 AND ?3<=not_after",
        params![
            claim_owner_id,
            claim_token_digest,
            claim_expires_at,
            poll_id,
            poll_digest,
            i64::try_from(expected_revision)?,
            i64::try_from(expected_generation)?,
        ],
    )?;
    if changed == 0 {
        return Ok(None);
    }
    ensure!(changed == 1, "V273 event poll claim changed multiple rows");
    let after = read_event_poll_on(conn, poll_id)?
        .ok_or_else(|| anyhow::anyhow!("V273 claimed event poll disappeared"))?;
    finish_claim(
        poll_id,
        poll_digest,
        expected_revision,
        expected_generation,
        claim_owner_id,
        raw_claim_token,
        claim_token_digest,
        claim_expires_at,
        after.envelope.event_poll_digest,
        after.claim,
    )
    .map(Some)
}

#[allow(clippy::too_many_arguments)]
fn finish_claim(
    poll_id: &str,
    poll_digest: &str,
    expected_revision: u64,
    expected_generation: u64,
    claim_owner_id: &str,
    raw_claim_token: String,
    claim_token_digest: String,
    claim_expires_at: &str,
    actual_poll_digest: String,
    actual: super::types::PollClaimProjection,
) -> Result<ExternalPoolAdapterTaskPollClaim> {
    ensure!(
        actual_poll_digest == poll_digest
            && actual.status == CLAIM_STATUS_CLAIMED
            && actual.revision
                == expected_revision
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("V273 poll claim revision overflow"))?
            && actual.generation
                == expected_generation
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("V273 poll claim generation overflow"))?
            && actual.owner_id.as_deref() == Some(claim_owner_id)
            && actual.token_digest.as_deref() == Some(claim_token_digest.as_str())
            && actual.expires_at.as_deref() == Some(claim_expires_at),
        "V273 poll claim readback is not exact"
    );
    Ok(ExternalPoolAdapterTaskPollClaim {
        poll_id: poll_id.to_string(),
        poll_digest: poll_digest.to_string(),
        claim_revision: actual.revision,
        claim_generation: actual.generation,
        claim_owner_id: claim_owner_id.to_string(),
        raw_claim_token,
        claim_expires_at: claim_expires_at.to_string(),
    })
}

fn validate_claim_input(owner: &str, expires_at: &str) -> Result<()> {
    ensure!(
        !owner.is_empty()
            && owner.trim() == owner
            && owner.chars().count() <= 240
            && !owner.chars().any(char::is_control),
        "V273 poll claim owner is invalid"
    );
    ensure!(
        expires_at.len() == 30,
        "V273 poll claim expiry is not UTC nanos"
    );
    let parsed: DateTime<FixedOffset> = DateTime::parse_from_rfc3339(expires_at)?;
    ensure!(
        parsed.offset().local_minus_utc() == 0
            && parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) == expires_at,
        "V273 poll claim expiry is not canonical UTC nanos"
    );
    Ok(())
}
