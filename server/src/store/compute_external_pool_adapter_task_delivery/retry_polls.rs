//! Successor poll intents after a committed poll exchange becomes durably outcome-unknown.

use anyhow::{ensure, Result};
use rusqlite::{params, Connection, Transaction};

use crate::compute_federation::external_pool_adapter_task_protocol_production::{
    canonical_task_production_event_poll_json_and_digest,
    canonical_task_production_reconcile_poll_json_and_digest,
    ExternalPoolAdapterTaskEventPollEnvelope, ExternalPoolAdapterTaskProductionBoundary,
    ExternalPoolAdapterTaskProductionEffects, ExternalPoolAdapterTaskProductionReadiness,
    ExternalPoolAdapterTaskReconcilePollEnvelope, TASK_PRODUCTION_NO_V213_AUTHORITY,
};

use super::{
    historical_cleanup::HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority,
    install_external_pool_adapter_task_reachability_pending_plan_on,
    mapping::{event_poll_values, reconcile_poll_values},
    polls::ensure_poll_binds_history,
    reachability_pending_plan::{
        ExternalPoolAdapterTaskReachabilityPendingPlan,
        ExternalPoolAdapterTaskReachabilityPendingWrite,
        ExternalPoolAdapterTaskReachabilityPendingWriteKind,
    },
    read::{read_event_poll_on, read_reconcile_poll_on},
    types::{PollClaimProjection, CLAIM_STATUS_IN_FLIGHT_UNKNOWN, CLAIM_STATUS_PENDING},
    write::{
        event_poll_needs_insert_on, insert_external_pool_adapter_task_event_poll_on,
        insert_external_pool_adapter_task_reconcile_poll_on, reconcile_poll_needs_insert_on,
    },
};

pub(in crate::store) struct ExternalPoolAdapterTaskRetryPollRequest {
    request_digest: String,
    not_before: String,
    not_after: String,
    created_at: String,
}

impl ExternalPoolAdapterTaskRetryPollRequest {
    pub(in crate::store) fn new(
        request_digest: String,
        not_before: String,
        not_after: String,
        created_at: String,
    ) -> Self {
        Self {
            request_digest,
            not_before,
            not_after,
            created_at,
        }
    }
}

pub(in crate::store) fn insert_external_pool_adapter_task_reconcile_retry_after_unknown_on<
    'tx,
    'conn,
>(
    connection: &'tx Transaction<'conn>,
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn>,
    predecessor_id: &str,
    predecessor_digest: &str,
    request: ExternalPoolAdapterTaskRetryPollRequest,
) -> Result<ExternalPoolAdapterTaskReconcilePollEnvelope> {
    validate_request(authority, &request)?;
    let predecessor = read_reconcile_poll_on(connection, predecessor_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 reconcile retry predecessor disappeared"))?;
    ensure!(
        predecessor.envelope.reconcile_poll_digest == predecessor_digest
            && predecessor.claim.status == CLAIM_STATUS_IN_FLIGHT_UNKNOWN,
        "V278 reconcile retry predecessor is not exact outcome-unknown"
    );
    ensure_poll_binds_history(&predecessor.envelope.poll.command, authority)?;
    let (uncertain_exchange_id, uncertain_exchange_digest) = ensure_unknown_exchange_on(
        connection,
        "reconcile_poll",
        predecessor_id,
        predecessor_digest,
    )?;
    let next_ordinal = predecessor
        .envelope
        .poll
        .lineage
        .poll_ordinal
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("V278 reconcile poll ordinal overflow"))?;
    let mut successor = predecessor.envelope.clone();
    successor.reconcile_poll_id = format!(
        "external_pool_reconcile_retry_{}_{}",
        predecessor_digest, next_ordinal
    );
    successor.reconcile_poll_digest.clear();
    successor.envelope_mut(next_ordinal, predecessor_id, predecessor_digest, request);
    successor.poll.uncertain_exchange_attempt_id = uncertain_exchange_id;
    successor.poll.uncertain_exchange_attempt_digest = uncertain_exchange_digest;
    successor.reconcile_poll_digest =
        canonical_task_production_reconcile_poll_json_and_digest(&successor)?.1;
    insert_reconcile_successor_on(connection, successor)
}

pub(in crate::store) fn insert_external_pool_adapter_task_event_retry_after_unknown_on<
    'tx,
    'conn,
>(
    connection: &'tx Transaction<'conn>,
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn>,
    predecessor_id: &str,
    predecessor_digest: &str,
    request: ExternalPoolAdapterTaskRetryPollRequest,
) -> Result<ExternalPoolAdapterTaskEventPollEnvelope> {
    validate_request(authority, &request)?;
    let predecessor = read_event_poll_on(connection, predecessor_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 event retry predecessor disappeared"))?;
    ensure!(
        predecessor.envelope.event_poll_digest == predecessor_digest
            && predecessor.claim.status == CLAIM_STATUS_IN_FLIGHT_UNKNOWN,
        "V278 event retry predecessor is not exact outcome-unknown"
    );
    ensure_poll_binds_history(&predecessor.envelope.poll.command, authority)?;
    let _unknown =
        ensure_unknown_exchange_on(connection, "event_poll", predecessor_id, predecessor_digest)?;
    let next_ordinal = predecessor
        .envelope
        .poll
        .lineage
        .poll_ordinal
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("V278 event poll ordinal overflow"))?;
    let mut successor = predecessor.envelope.clone();
    successor.event_poll_id = format!(
        "external_pool_event_retry_{}_{}",
        predecessor_digest, next_ordinal
    );
    successor.event_poll_digest.clear();
    successor.envelope_mut(next_ordinal, predecessor_id, predecessor_digest, request);
    successor.event_poll_digest =
        canonical_task_production_event_poll_json_and_digest(&successor)?.1;
    insert_event_successor_on(connection, successor)
}

trait RetryPollEnvelope {
    fn envelope_mut(
        &mut self,
        ordinal: u64,
        predecessor_id: &str,
        predecessor_digest: &str,
        request: ExternalPoolAdapterTaskRetryPollRequest,
    );
}

impl RetryPollEnvelope for ExternalPoolAdapterTaskReconcilePollEnvelope {
    fn envelope_mut(
        &mut self,
        ordinal: u64,
        predecessor_id: &str,
        predecessor_digest: &str,
        request: ExternalPoolAdapterTaskRetryPollRequest,
    ) {
        set_lineage_and_request(
            &mut self.poll.lineage,
            &mut self.poll.request_digest,
            &mut self.poll.not_before,
            &mut self.poll.not_after,
            &mut self.poll.created_at,
            &mut self.poll.boundary,
            ordinal,
            predecessor_id,
            predecessor_digest,
            request,
        );
    }
}

impl RetryPollEnvelope for ExternalPoolAdapterTaskEventPollEnvelope {
    fn envelope_mut(
        &mut self,
        ordinal: u64,
        predecessor_id: &str,
        predecessor_digest: &str,
        request: ExternalPoolAdapterTaskRetryPollRequest,
    ) {
        set_lineage_and_request(
            &mut self.poll.lineage,
            &mut self.poll.request_digest,
            &mut self.poll.not_before,
            &mut self.poll.not_after,
            &mut self.poll.created_at,
            &mut self.poll.boundary,
            ordinal,
            predecessor_id,
            predecessor_digest,
            request,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn set_lineage_and_request(
    lineage: &mut crate::compute_federation::external_pool_adapter_task_protocol_production::ExternalPoolAdapterTaskPollLineage,
    request_digest: &mut String,
    not_before: &mut String,
    not_after: &mut String,
    created_at: &mut String,
    boundary: &mut ExternalPoolAdapterTaskProductionBoundary,
    ordinal: u64,
    predecessor_id: &str,
    predecessor_digest: &str,
    request: ExternalPoolAdapterTaskRetryPollRequest,
) {
    lineage.predecessor_id = Some(predecessor_id.to_string());
    lineage.predecessor_digest = Some(predecessor_digest.to_string());
    lineage.poll_ordinal = ordinal;
    *request_digest = request.request_digest;
    *not_before = request.not_before;
    *not_after = request.not_after;
    *created_at = request.created_at;
    *boundary = no_authority_boundary();
}

fn validate_request(
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'_, '_>,
    request: &ExternalPoolAdapterTaskRetryPollRequest,
) -> Result<()> {
    ensure!(
        request.created_at == authority.checked_at()
            && request.not_before.as_str() <= authority.checked_at()
            && authority.checked_at() < request.not_after.as_str()
            && request.not_after.as_str() <= authority.cleanup_expires_at(),
        "V278 retry poll does not use its historical reproof time"
    );
    Ok(())
}

fn ensure_unknown_exchange_on(
    connection: &Connection,
    source_kind: &str,
    source_id: &str,
    source_digest: &str,
) -> Result<(String, String)> {
    let mut statement = connection.prepare(
        "SELECT attempt.exchange_attempt_id,attempt.exchange_attempt_digest
           FROM compute_external_pool_adapter_task_exchange_attempts attempt
          WHERE attempt.source_kind=?1 AND attempt.source_id=?2 AND attempt.source_digest=?3
            AND NOT EXISTS (
                SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts receipt
                 WHERE receipt.exchange_attempt_id=attempt.exchange_attempt_id
                   AND receipt.exchange_attempt_digest=attempt.exchange_attempt_digest)",
    )?;
    let mut rows = statement
        .query_map(params![source_kind, source_id, source_digest], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ensure!(
        rows.len() == 1,
        "V278 retry poll lacks exactly one durable unknown exchange"
    );
    rows.pop()
        .ok_or_else(|| anyhow::anyhow!("V278 exact unknown exchange disappeared"))
}

fn insert_reconcile_successor_on(
    connection: &Connection,
    successor: ExternalPoolAdapterTaskReconcilePollEnvelope,
) -> Result<ExternalPoolAdapterTaskReconcilePollEnvelope> {
    let fresh = reconcile_poll_needs_insert_on(connection, &successor)?;
    let plan = fresh
        .then(|| {
            ExternalPoolAdapterTaskReachabilityPendingPlan::new(vec![
                ExternalPoolAdapterTaskReachabilityPendingWrite::new(
                    ExternalPoolAdapterTaskReachabilityPendingWriteKind::ReconcilePoll,
                    reconcile_poll_values(&successor, &initial_claim())?,
                )?,
            ])
        })
        .transpose()?;
    let guard = plan
        .map(|plan| {
            install_external_pool_adapter_task_reachability_pending_plan_on(connection, plan)
        })
        .transpose()?;
    insert_external_pool_adapter_task_reconcile_poll_on(connection, guard.as_ref(), &successor)?;
    if let Some(guard) = guard {
        guard.ensure_fully_consumed()?;
    }
    Ok(successor)
}

fn insert_event_successor_on(
    connection: &Connection,
    successor: ExternalPoolAdapterTaskEventPollEnvelope,
) -> Result<ExternalPoolAdapterTaskEventPollEnvelope> {
    let fresh = event_poll_needs_insert_on(connection, &successor)?;
    let plan = fresh
        .then(|| {
            ExternalPoolAdapterTaskReachabilityPendingPlan::new(vec![
                ExternalPoolAdapterTaskReachabilityPendingWrite::new(
                    ExternalPoolAdapterTaskReachabilityPendingWriteKind::EventPoll,
                    event_poll_values(&successor, &initial_claim())?,
                )?,
            ])
        })
        .transpose()?;
    let guard = plan
        .map(|plan| {
            install_external_pool_adapter_task_reachability_pending_plan_on(connection, plan)
        })
        .transpose()?;
    insert_external_pool_adapter_task_event_poll_on(connection, guard.as_ref(), &successor)?;
    if let Some(guard) = guard {
        guard.ensure_fully_consumed()?;
    }
    Ok(successor)
}

fn initial_claim() -> PollClaimProjection {
    PollClaimProjection {
        status: CLAIM_STATUS_PENDING.to_string(),
        revision: 1,
        generation: 0,
        owner_id: None,
        token_digest: None,
        expires_at: None,
    }
}

fn no_authority_boundary() -> ExternalPoolAdapterTaskProductionBoundary {
    ExternalPoolAdapterTaskProductionBoundary {
        authority_status: TASK_PRODUCTION_NO_V213_AUTHORITY.to_string(),
        effects: ExternalPoolAdapterTaskProductionEffects::none(),
        readiness: ExternalPoolAdapterTaskProductionReadiness::none(),
    }
}
