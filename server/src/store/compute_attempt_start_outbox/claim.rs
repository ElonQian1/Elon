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

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use crate::store::compute_external_pool_adapter_runtime_bundle::ReprovedExternalPoolAdapterRouteAndActiveSuccessorAuthority;

use super::{read::outbox_by_id_on, types::StartOutboxClaimHandle};

use crate::store::compute_external_pool_adapter_task_delivery::HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority;

pub(super) fn try_claim_on(
    connection: &Connection,
    claim_owner_id: &str,
    claimed_at: &str,
    claim_expires_at: &str,
) -> Result<Option<StartOutboxClaimHandle>> {
    try_claim_scoped_on(
        connection,
        None,
        claim_owner_id,
        claimed_at,
        claim_expires_at,
    )
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in crate::store) fn try_claim_external_pool_on<'authority, 'tx, 'conn, 'runtime>(
    connection: &'tx rusqlite::Transaction<'conn>,
    authority: &ReprovedExternalPoolAdapterRouteAndActiveSuccessorAuthority<
        'authority,
        'tx,
        'conn,
        'runtime,
    >,
    claim_owner_id: &str,
    claim_expires_at: &str,
) -> Result<Option<StartOutboxClaimHandle>> {
    let provider_id = &authority
        .route_authorization()
        .envelope()
        .authorization
        .provider
        .provider_id;
    ensure!(
        !provider_id.is_empty() && provider_id.trim() == provider_id && provider_id.len() <= 240,
        "external-pool Start outbox provider id is invalid"
    );
    try_claim_scoped_on(
        connection,
        Some(provider_id.as_str()),
        claim_owner_id,
        authority.checked_at(),
        claim_expires_at,
    )
}

pub(in crate::store) fn try_claim_historical_external_pool_cleanup_on<'tx, 'conn>(
    connection: &'tx rusqlite::Transaction<'conn>,
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn>,
    claim_owner_id: &str,
    claim_expires_at: &str,
) -> Result<Option<StartOutboxClaimHandle>> {
    let claimed_at = authority.checked_at();
    validate_claim_input(claim_owner_id, claimed_at, claim_expires_at)?;
    ensure!(
        claim_expires_at <= authority.cleanup_expires_at(),
        "historical external-pool cleanup claim exceeds its route cleanup horizon"
    );
    let identity = &authority.exchange_attempt().attempt.identity;
    if release_one_expired_claim_on(connection, claimed_at, Some(&identity.adapter.provider_id))? {
        return Ok(None);
    }
    let mut statement = connection.prepare(
        "SELECT cleanup.outbox_id
           FROM compute_attempt_start_outbox cleanup
           JOIN compute_providers provider ON provider.provider_id=cleanup.provider_id
           JOIN compute_attempt_start_outbox source
             ON source.outbox_id=?4 AND source.outbox_digest=?5
            AND source.command_id=?2 AND source.command_digest=?3
            AND source.provider_id=?6 AND source.route_authorization_id=?7
            AND source.route_authorization_digest=?8
          WHERE cleanup.operation_kind='cancel' AND cleanup.state='pending'
            AND cleanup.command_id=?2 AND cleanup.command_digest=?3
            AND cleanup.subject_outbox_id=source.outbox_id
            AND cleanup.provider_id=source.provider_id
            AND cleanup.adapter_id=source.adapter_id
            AND cleanup.route_authorization_id=?7
            AND cleanup.route_authorization_digest=?8
            AND provider.provider_kind='external_pool'
            AND cleanup.next_attempt_at<=?1 AND cleanup.not_before<=?1
            AND ?1<cleanup.not_after
            AND NOT EXISTS (
                SELECT 1 FROM compute_attempt_start_send_attempts send
                 WHERE send.outbox_id=cleanup.outbox_id)
            AND NOT EXISTS (
                SELECT 1 FROM compute_attempt_no_start_proofs proof
                 WHERE proof.command_id=cleanup.command_id)
          ORDER BY cleanup.next_attempt_at,cleanup.outbox_id LIMIT 2",
    )?;
    let mut candidates = statement
        .query_map(
            params![
                claimed_at,
                identity.command.command_id,
                identity.command.command_digest,
                identity.command.outbox_id,
                identity.command.outbox_digest,
                identity.adapter.provider_id,
                identity.route.route_authorization_id,
                identity.route.route_authorization_digest,
            ],
            |row| row.get::<_, String>(0),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ensure!(
        candidates.len() <= 1,
        "historical external-pool cleanup has multiple exact cancel candidates"
    );
    let Some(candidate_id) = candidates.pop() else {
        return Ok(None);
    };
    claim_candidate_on(
        connection,
        &candidate_id,
        claim_owner_id,
        claimed_at,
        claim_expires_at,
    )
}

fn try_claim_scoped_on(
    connection: &Connection,
    external_pool_provider_id: Option<&str>,
    claim_owner_id: &str,
    claimed_at: &str,
    claim_expires_at: &str,
) -> Result<Option<StartOutboxClaimHandle>> {
    validate_claim_input(claim_owner_id, claimed_at, claim_expires_at)?;
    if release_one_expired_claim_on(connection, claimed_at, external_pool_provider_id)? {
        // A second state transition at the same fixed timestamp would violate the monotonic
        // revision clock. The next poll may claim the released operation.
        return Ok(None);
    }
    let candidate_id = if let Some(provider_id) = external_pool_provider_id {
        connection
            .query_row(
                "SELECT outbox.outbox_id
                   FROM compute_attempt_start_outbox outbox
                   JOIN compute_providers provider ON provider.provider_id=outbox.provider_id
                  WHERE outbox.provider_id=?2 AND provider.provider_kind='external_pool'
                    AND provider.status='active'
                    AND outbox.operation_kind IN ('prepare','commit')
                    AND outbox.state='pending' AND outbox.next_attempt_at<=?1
                    AND outbox.not_before<=?1 AND ?1<outbox.not_after
                    AND NOT EXISTS (
                        SELECT 1 FROM compute_attempt_no_start_proofs proof
                         WHERE proof.command_id=outbox.command_id)
                  ORDER BY outbox.next_attempt_at,outbox.outbox_id LIMIT 1",
                params![claimed_at, provider_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
    } else {
        connection
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
            .optional()?
    };
    let Some(candidate_id) = candidate_id else {
        return Ok(None);
    };
    claim_candidate_on(
        connection,
        &candidate_id,
        claim_owner_id,
        claimed_at,
        claim_expires_at,
    )
}

fn claim_candidate_on(
    connection: &Connection,
    candidate_id: &str,
    claim_owner_id: &str,
    claimed_at: &str,
    claim_expires_at: &str,
) -> Result<Option<StartOutboxClaimHandle>> {
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

fn validate_claim_input(
    claim_owner_id: &str,
    claimed_at: &str,
    claim_expires_at: &str,
) -> Result<()> {
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
    Ok(())
}

fn release_one_expired_claim_on(
    connection: &Connection,
    released_at: &str,
    external_pool_provider_id: Option<&str>,
) -> Result<bool> {
    let expired = if let Some(provider_id) = external_pool_provider_id {
        connection
            .query_row(
                "SELECT claimed.outbox_id, claimed.state_revision,
                        claimed.attempt_count, claimed.claim_generation
                   FROM compute_attempt_start_outbox claimed
                   JOIN compute_providers provider ON provider.provider_id=claimed.provider_id
                  WHERE claimed.provider_id=?2 AND provider.provider_kind='external_pool'
                    AND claimed.operation_kind IN ('prepare','commit','cancel')
                    AND claimed.state='claimed' AND claimed.claim_expires_at<=?1
                    AND claimed.updated_at<?1
                    AND NOT EXISTS (
                        SELECT 1 FROM compute_attempt_start_send_attempts attempt
                         WHERE attempt.outbox_id=claimed.outbox_id)
                  ORDER BY claimed.claim_expires_at,claimed.outbox_id LIMIT 1",
                params![released_at, provider_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .optional()?
    } else {
        connection
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
            .optional()?
    };
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
