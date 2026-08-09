use anyhow::{bail, ensure, Result};
use chrono::{DateTime, FixedOffset};
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    compute_federation::start_outbox::{
        canonical_start_outbox_claim_receipt_json_and_digest,
        ComputeStartOutboxClaimReceiptEnvelope, COMPUTE_OUTBOX_STATE_CLAIMED,
        COMPUTE_START_OUTBOX_CANONICALIZATION, COMPUTE_START_OUTBOX_CLAIM_RECEIPT_SCHEMA,
        COMPUTE_START_OUTBOX_DIGEST_ALGORITHM,
    },
    store::{hash_token, new_id},
};

use super::{read::outbox_by_id_on, types::StartOutboxClaimHandle};

pub(super) fn try_claim_on(
    connection: &Connection,
    claim_owner_id: &str,
    claimed_at: &str,
    claim_expires_at: &str,
) -> Result<Option<StartOutboxClaimHandle>> {
    ensure_fixed_timestamp(claimed_at)?;
    ensure_fixed_timestamp(claim_expires_at)?;
    ensure!(
        !claim_owner_id.is_empty()
            && claim_owner_id.trim() == claim_owner_id
            && claim_owner_id.len() <= 160,
        "Start outbox claim owner is invalid"
    );
    ensure!(
        claimed_at < claim_expires_at,
        "Start outbox claim expiry must be after claim time"
    );
    if release_one_expired_claim_on(connection, claimed_at)? {
        // A second state transition at the same fixed timestamp would violate the monotonic
        // revision clock. The next poll may claim the released operation.
        return Ok(None);
    }
    let candidate_id = connection
        .query_row(
            "SELECT outbox_id
               FROM compute_attempt_start_outbox
              WHERE state='pending' AND next_attempt_at<=?1 AND not_before<=?1
                AND ?1<not_after
                AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_no_start_proofs proof
                     WHERE proof.command_id=compute_attempt_start_outbox.command_id
                )
              ORDER BY next_attempt_at, outbox_id LIMIT 1",
            params![claimed_at],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(candidate_id) = candidate_id else {
        return Ok(None);
    };
    let before = outbox_by_id_on(connection, &candidate_id)?
        .ok_or_else(|| anyhow::anyhow!("Start outbox candidate disappeared before claim"))?;
    ensure!(
        claimed_at > before.projection.updated_at.as_str()
            && claim_expires_at <= before.envelope.not_after.as_str(),
        "Start outbox claim clock or expiry exceeds the operation window"
    );
    let raw_claim_token = new_id("start_claim");
    let claim_token_digest = hash_token(&raw_claim_token);
    let changed = connection.execute(
        "UPDATE compute_attempt_start_outbox
            SET state='claimed', state_revision=state_revision+1,
                claim_owner_id=?1, claim_token_digest=?2,
                claim_generation=claim_generation+1, claim_expires_at=?3,
                last_failure_code=NULL, updated_at=?4
          WHERE outbox_id=?5 AND state='pending' AND state_revision=?6
            AND attempt_count=?7 AND claim_generation=?8
            AND next_attempt_at<=?4 AND not_before<=?4 AND ?4<not_after",
        params![
            claim_owner_id,
            claim_token_digest,
            claim_expires_at,
            claimed_at,
            candidate_id,
            before.projection.state_revision,
            before.projection.attempt_count,
            before.projection.claim_generation,
        ],
    )?;
    if changed == 0 {
        return Ok(None);
    }
    if changed != 1 {
        bail!("Start outbox claim CAS changed an unexpected number of rows");
    }
    let operation = outbox_by_id_on(connection, &candidate_id)?
        .ok_or_else(|| anyhow::anyhow!("claimed Start outbox row disappeared"))?;
    ensure!(
        operation.projection.state == COMPUTE_OUTBOX_STATE_CLAIMED
            && operation.projection.state_revision == before.projection.state_revision + 1
            && operation.projection.attempt_count == before.projection.attempt_count
            && operation.projection.claim_generation == before.projection.claim_generation + 1
            && operation.projection.claim_owner_id.as_deref() == Some(claim_owner_id)
            && operation.projection.claim_token_digest.as_deref()
                == Some(claim_token_digest.as_str())
            && operation.projection.claim_expires_at.as_deref() == Some(claim_expires_at),
        "Start outbox claim readback failed exact custody audit"
    );
    let mut receipt = ComputeStartOutboxClaimReceiptEnvelope {
        schema: COMPUTE_START_OUTBOX_CLAIM_RECEIPT_SCHEMA.to_string(),
        claim_receipt_id: new_id("start_claim_receipt"),
        claim_receipt_digest: String::new(),
        canonicalization: COMPUTE_START_OUTBOX_CANONICALIZATION.to_string(),
        digest_algorithm: COMPUTE_START_OUTBOX_DIGEST_ALGORITHM.to_string(),
        outbox_id: operation.envelope.outbox_id.clone(),
        outbox_digest: operation.envelope.outbox_digest.clone(),
        state_revision: operation.projection.state_revision,
        attempt_no: operation.projection.attempt_count + 1,
        claim_owner_id: claim_owner_id.to_string(),
        claim_token_digest,
        claim_generation: operation.projection.claim_generation,
        claimed_at: claimed_at.to_string(),
        claim_expires_at: claim_expires_at.to_string(),
    };
    let (_, digest) = canonical_start_outbox_claim_receipt_json_and_digest(&receipt)?;
    receipt.claim_receipt_digest = digest;
    let (_, recomputed) = canonical_start_outbox_claim_receipt_json_and_digest(&receipt)?;
    ensure!(
        recomputed == receipt.claim_receipt_digest,
        "Start outbox claim receipt failed canonical audit"
    );
    Ok(Some(StartOutboxClaimHandle {
        operation,
        receipt,
        raw_claim_token,
    }))
}

fn release_one_expired_claim_on(connection: &Connection, released_at: &str) -> Result<bool> {
    let expired = connection
        .query_row(
            "SELECT outbox_id, state_revision, attempt_count, claim_generation
               FROM compute_attempt_start_outbox claimed
              WHERE state='claimed' AND claim_expires_at<=?1 AND updated_at<?1
                AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_start_send_attempts attempt
                     WHERE attempt.outbox_id=claimed.outbox_id
                )
              ORDER BY claim_expires_at, outbox_id LIMIT 1",
            params![released_at],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()?;
    let Some((outbox_id, revision, attempt_count, generation)) = expired else {
        return Ok(false);
    };
    let changed = connection.execute(
        "UPDATE compute_attempt_start_outbox
            SET state='pending', state_revision=state_revision+1,
                claim_owner_id=NULL, claim_token_digest=NULL, claim_expires_at=NULL,
                last_failure_code='CLAIM_EXPIRED_BEFORE_SEND', updated_at=?1
          WHERE outbox_id=?2 AND state='claimed' AND state_revision=?3
            AND attempt_count=?4 AND claim_generation=?5 AND claim_expires_at<=?1
            AND NOT EXISTS (
                SELECT 1 FROM compute_attempt_start_send_attempts attempt
                 WHERE attempt.outbox_id=compute_attempt_start_outbox.outbox_id
            )",
        params![released_at, outbox_id, revision, attempt_count, generation],
    )?;
    Ok(changed == 1)
}

fn ensure_fixed_timestamp(value: &str) -> Result<()> {
    ensure!(
        value.len() == 30 && value.as_bytes().get(19) == Some(&b'.') && value.ends_with('Z'),
        "Start outbox claim timestamp must be fixed UTC nanoseconds"
    );
    let _: DateTime<FixedOffset> = DateTime::parse_from_rfc3339(value)?;
    Ok(())
}
