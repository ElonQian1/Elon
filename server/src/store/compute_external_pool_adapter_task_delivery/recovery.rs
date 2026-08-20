mod audit;

use anyhow::{ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::{
    install_external_pool_adapter_task_reachability_pending_plan_on,
    poll_plan::poll_cas_values,
    reachability_pending_plan::{
        ExternalPoolAdapterTaskReachabilityPendingPlan,
        ExternalPoolAdapterTaskReachabilityPendingWrite,
        ExternalPoolAdapterTaskReachabilityPendingWriteKind,
    },
    read::{read_event_poll_on, read_reconcile_poll_on},
    types::{
        ExternalPoolAdapterTaskDeliveryRecoveryReport, PollClaimProjection, CLAIM_STATUS_CLAIMED,
        CLAIM_STATUS_DELIVERY_OBSERVED, CLAIM_STATUS_IN_FLIGHT_UNKNOWN, CLAIM_STATUS_PENDING,
    },
};
use audit::{audit_event_target_on, audit_reconcile_target_on};

pub(super) fn recover_on(
    conn: &Connection,
) -> Result<ExternalPoolAdapterTaskDeliveryRecoveryReport> {
    let mut report = ExternalPoolAdapterTaskDeliveryRecoveryReport::default();
    if let Some(audited_rows) = recover_reconcile_on(conn)? {
        report.audited_rows += audited_rows;
        report.recovered_rows += 1;
    }
    if let Some(audited_rows) = recover_event_on(conn)? {
        report.audited_rows += audited_rows;
        report.recovered_rows += 1;
    }
    report.eligible_rows = 0;
    Ok(report)
}

fn recover_reconcile_on(conn: &Connection) -> Result<Option<usize>> {
    let candidate = conn
        .query_row(
            "SELECT poll.reconcile_poll_id
               FROM compute_external_pool_adapter_task_reconcile_polls poll
              WHERE poll.claim_status='claimed' AND (
                EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts receipt
                         WHERE receipt.source_kind='reconcile_poll'
                           AND receipt.source_id=poll.reconcile_poll_id
                           AND receipt.source_digest=poll.reconcile_poll_digest)
                OR (poll.claim_expires_at<=strftime('%Y-%m-%dT%H:%M:%f000000Z','now')
                    AND EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts attempt
                           WHERE attempt.source_kind='reconcile_poll'
                             AND attempt.source_id=poll.reconcile_poll_id
                             AND attempt.source_digest=poll.reconcile_poll_digest
                             AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts receipt
                                             WHERE receipt.exchange_attempt_id=attempt.exchange_attempt_id
                                               AND receipt.exchange_attempt_digest=attempt.exchange_attempt_digest)))
                OR (poll.claim_expires_at<=strftime('%Y-%m-%dT%H:%M:%f000000Z','now')
                    AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts attempt
                                    WHERE attempt.source_kind='reconcile_poll'
                                      AND attempt.source_id=poll.reconcile_poll_id
                                      AND attempt.source_digest=poll.reconcile_poll_digest)))
              ORDER BY poll.claim_expires_at,poll.reconcile_poll_id LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(id) = candidate else {
        return Ok(None);
    };
    let before = read_reconcile_poll_on(conn, &id)?
        .ok_or_else(|| anyhow::anyhow!("V273 reconcile recovery candidate disappeared"))?;
    let target = reconcile_target_on(conn, &id)?
        .ok_or_else(|| anyhow::anyhow!("V273 reconcile recovery target disappeared"))?;
    let source_audits = audit_reconcile_target_on(conn, &before, target)?;
    transition_reconcile_on(
        conn,
        &before.claim,
        &id,
        &before.envelope.reconcile_poll_digest,
        target,
    )?;
    let after = read_reconcile_poll_on(conn, &id)?
        .ok_or_else(|| anyhow::anyhow!("V273 recovered reconcile poll disappeared"))?;
    ensure_recovered(&before.claim, &after.claim, target)?;
    Ok(Some(source_audits + 2))
}

fn recover_event_on(conn: &Connection) -> Result<Option<usize>> {
    let candidate = conn
        .query_row(
            "SELECT poll.event_poll_id
               FROM compute_external_pool_adapter_task_event_polls poll
              WHERE poll.claim_status='claimed' AND (
                EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_event_batches batch
                         WHERE batch.event_poll_id=poll.event_poll_id
                           AND batch.event_poll_digest=poll.event_poll_digest
                           AND (SELECT count(*) FROM compute_external_pool_adapter_task_events event
                                WHERE event.event_batch_id=batch.event_batch_id)=batch.event_count
                           AND COALESCE((SELECT max(event.event_ordinal)
                                         FROM compute_external_pool_adapter_task_events event
                                        WHERE event.event_batch_id=batch.event_batch_id),0)=batch.event_count)
                OR (poll.claim_expires_at<=strftime('%Y-%m-%dT%H:%M:%f000000Z','now')
                    AND EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts attempt
                           WHERE attempt.source_kind='event_poll'
                             AND attempt.source_id=poll.event_poll_id
                             AND attempt.source_digest=poll.event_poll_digest
                             AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts receipt
                                             WHERE receipt.exchange_attempt_id=attempt.exchange_attempt_id
                                               AND receipt.exchange_attempt_digest=attempt.exchange_attempt_digest)))
                OR (poll.claim_expires_at<=strftime('%Y-%m-%dT%H:%M:%f000000Z','now')
                    AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts attempt
                                    WHERE attempt.source_kind='event_poll'
                                      AND attempt.source_id=poll.event_poll_id
                                      AND attempt.source_digest=poll.event_poll_digest)))
              ORDER BY poll.claim_expires_at,poll.event_poll_id LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(id) = candidate else {
        return Ok(None);
    };
    let before = read_event_poll_on(conn, &id)?
        .ok_or_else(|| anyhow::anyhow!("V273 event recovery candidate disappeared"))?;
    let target = event_target_on(conn, &id)?
        .ok_or_else(|| anyhow::anyhow!("V273 event recovery target disappeared"))?;
    let source_audits = audit_event_target_on(conn, &before, target)?;
    transition_event_on(
        conn,
        &before.claim,
        &id,
        &before.envelope.event_poll_digest,
        target,
    )?;
    let after = read_event_poll_on(conn, &id)?
        .ok_or_else(|| anyhow::anyhow!("V273 recovered event poll disappeared"))?;
    ensure_recovered(&before.claim, &after.claim, target)?;
    Ok(Some(source_audits + 2))
}

fn reconcile_target_on(conn: &Connection, id: &str) -> Result<Option<&'static str>> {
    let value: Option<i64> = conn
        .query_row(
            "SELECT CASE
              WHEN EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts receipt
                           WHERE receipt.source_kind='reconcile_poll' AND receipt.source_id=poll.reconcile_poll_id
                             AND receipt.source_digest=poll.reconcile_poll_digest) THEN 1
              WHEN poll.claim_expires_at<=strftime('%Y-%m-%dT%H:%M:%f000000Z','now')
               AND EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts attempt
                           WHERE attempt.source_kind='reconcile_poll' AND attempt.source_id=poll.reconcile_poll_id
                             AND attempt.source_digest=poll.reconcile_poll_digest
                             AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts receipt
                                             WHERE receipt.exchange_attempt_id=attempt.exchange_attempt_id
                                               AND receipt.exchange_attempt_digest=attempt.exchange_attempt_digest)) THEN 2
              WHEN poll.claim_expires_at<=strftime('%Y-%m-%dT%H:%M:%f000000Z','now')
               AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts attempt
                               WHERE attempt.source_kind='reconcile_poll' AND attempt.source_id=poll.reconcile_poll_id
                                 AND attempt.source_digest=poll.reconcile_poll_digest) THEN 3 END
             FROM compute_external_pool_adapter_task_reconcile_polls poll
            WHERE poll.reconcile_poll_id=?1 AND poll.claim_status='claimed'",
            params![id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    Ok(target_status(value))
}

fn event_target_on(conn: &Connection, id: &str) -> Result<Option<&'static str>> {
    let value: Option<i64> = conn
        .query_row(
            "SELECT CASE
              WHEN EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_event_batches batch
                           WHERE batch.event_poll_id=poll.event_poll_id AND batch.event_poll_digest=poll.event_poll_digest
                             AND (SELECT count(*) FROM compute_external_pool_adapter_task_events event WHERE event.event_batch_id=batch.event_batch_id)=batch.event_count
                             AND COALESCE((SELECT max(event.event_ordinal) FROM compute_external_pool_adapter_task_events event WHERE event.event_batch_id=batch.event_batch_id),0)=batch.event_count) THEN 1
              WHEN poll.claim_expires_at<=strftime('%Y-%m-%dT%H:%M:%f000000Z','now')
               AND EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts attempt
                           WHERE attempt.source_kind='event_poll' AND attempt.source_id=poll.event_poll_id
                             AND attempt.source_digest=poll.event_poll_digest
                             AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts receipt
                                             WHERE receipt.exchange_attempt_id=attempt.exchange_attempt_id
                                               AND receipt.exchange_attempt_digest=attempt.exchange_attempt_digest)) THEN 2
              WHEN poll.claim_expires_at<=strftime('%Y-%m-%dT%H:%M:%f000000Z','now')
               AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts attempt
                               WHERE attempt.source_kind='event_poll' AND attempt.source_id=poll.event_poll_id
                                 AND attempt.source_digest=poll.event_poll_digest) THEN 3 END
             FROM compute_external_pool_adapter_task_event_polls poll
            WHERE poll.event_poll_id=?1 AND poll.claim_status='claimed'",
            params![id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    Ok(target_status(value))
}

fn target_status(value: Option<i64>) -> Option<&'static str> {
    match value {
        Some(1) => Some(CLAIM_STATUS_DELIVERY_OBSERVED),
        Some(2) => Some(CLAIM_STATUS_IN_FLIGHT_UNKNOWN),
        Some(3) => Some(CLAIM_STATUS_PENDING),
        _ => None,
    }
}

fn transition_reconcile_on(
    conn: &Connection,
    claim: &PollClaimProjection,
    id: &str,
    digest: &str,
    target: &str,
) -> Result<()> {
    let after = recovered_projection(claim, target)?;
    let pending = install_recovery_plan_on(
        conn,
        ExternalPoolAdapterTaskReachabilityPendingWriteKind::ReconcilePollCas,
        id,
        digest,
        claim,
        &after,
    )?;
    let changed = conn.execute(
        "UPDATE compute_external_pool_adapter_task_reconcile_polls
            SET claim_status=?1,claim_revision=claim_revision+1,
                claim_generation=claim_generation,claim_owner_id=NULL,
                claim_token_digest=NULL,claim_expires_at=NULL
          WHERE reconcile_poll_id=?2 AND reconcile_poll_digest=?3
            AND claim_status='claimed' AND claim_revision=?4 AND claim_generation=?5
            AND claim_owner_id=?6 AND claim_token_digest=?7 AND claim_expires_at=?8",
        params![
            target,
            id,
            digest,
            i64::try_from(claim.revision)?,
            i64::try_from(claim.generation)?,
            claim.owner_id.as_deref(),
            claim.token_digest.as_deref(),
            claim.expires_at.as_deref()
        ],
    )?;
    ensure!(changed == 1, "V273 reconcile poll recovery CAS lost");
    pending.ensure_fully_consumed()?;
    Ok(())
}

fn transition_event_on(
    conn: &Connection,
    claim: &PollClaimProjection,
    id: &str,
    digest: &str,
    target: &str,
) -> Result<()> {
    let after = recovered_projection(claim, target)?;
    let pending = install_recovery_plan_on(
        conn,
        ExternalPoolAdapterTaskReachabilityPendingWriteKind::EventPollCas,
        id,
        digest,
        claim,
        &after,
    )?;
    let changed = conn.execute(
        "UPDATE compute_external_pool_adapter_task_event_polls
            SET claim_status=?1,claim_revision=claim_revision+1,
                claim_generation=claim_generation,claim_owner_id=NULL,
                claim_token_digest=NULL,claim_expires_at=NULL
          WHERE event_poll_id=?2 AND event_poll_digest=?3
            AND claim_status='claimed' AND claim_revision=?4 AND claim_generation=?5
            AND claim_owner_id=?6 AND claim_token_digest=?7 AND claim_expires_at=?8",
        params![
            target,
            id,
            digest,
            i64::try_from(claim.revision)?,
            i64::try_from(claim.generation)?,
            claim.owner_id.as_deref(),
            claim.token_digest.as_deref(),
            claim.expires_at.as_deref()
        ],
    )?;
    ensure!(changed == 1, "V273 event poll recovery CAS lost");
    pending.ensure_fully_consumed()?;
    Ok(())
}

fn recovered_projection(claim: &PollClaimProjection, target: &str) -> Result<PollClaimProjection> {
    Ok(PollClaimProjection {
        status: target.to_string(),
        revision: claim
            .revision
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("V273 poll recovery revision overflow"))?,
        generation: claim.generation,
        owner_id: None,
        token_digest: None,
        expires_at: None,
    })
}

fn install_recovery_plan_on(
    conn: &Connection,
    kind: ExternalPoolAdapterTaskReachabilityPendingWriteKind,
    id: &str,
    digest: &str,
    before: &PollClaimProjection,
    after: &PollClaimProjection,
) -> Result<super::reachability_pending_plan::ExternalPoolAdapterTaskReachabilityPendingPlanGuard> {
    let pending = ExternalPoolAdapterTaskReachabilityPendingPlan::new(vec![
        ExternalPoolAdapterTaskReachabilityPendingWrite::new(
            kind,
            poll_cas_values(id, digest, before, after)?,
        )?,
    ])?;
    install_external_pool_adapter_task_reachability_pending_plan_on(conn, pending)
}

fn ensure_recovered(
    before: &PollClaimProjection,
    after: &PollClaimProjection,
    target: &str,
) -> Result<()> {
    let expected_revision = before
        .revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("V273 poll recovery revision overflow"))?;
    ensure!(
        before.status == CLAIM_STATUS_CLAIMED
            && after.status == target
            && after.revision == expected_revision
            && after.generation == before.generation
            && after.owner_id.is_none()
            && after.token_digest.is_none()
            && after.expires_at.is_none(),
        "V273 poll recovery readback is not exact"
    );
    Ok(())
}
