//! Authenticated-events receipt closure: batch, ordered events, then exact poll CAS.

use anyhow::{ensure, Result};
use rusqlite::{params, types::Value, OptionalExtension, Transaction};

use crate::{
    compute_federation::{
        external_pool_adapter_broker_tls::ExternalPoolAdapterBrokerTaskVerifiedObservation,
        external_pool_adapter_task_protocol_production::{
            validate_task_production_event, validate_task_production_event_batch,
            validate_task_production_event_poll, ExternalPoolAdapterTaskEventBatchEnvelope,
            ExternalPoolAdapterTaskEventEnvelope, ExternalPoolAdapterTaskEventPollEnvelope,
            ExternalPoolAdapterTaskExchangeReceiptEnvelope, TASK_PRODUCTION_MAX_EVENTS_PER_BATCH,
        },
    },
    store::hash_token,
};

use super::{
    install_external_pool_adapter_task_reachability_pending_plan_on,
    mapping::{event_batch_values, event_poll_values, event_values},
    reachability_pending_plan::{
        ExternalPoolAdapterTaskReachabilityPendingPlan,
        ExternalPoolAdapterTaskReachabilityPendingWrite,
        ExternalPoolAdapterTaskReachabilityPendingWriteKind,
    },
    read::read_event_poll_on,
    receipt_ingress::PendingExternalPoolAdapterTaskReceiptIngress,
    types::{
        ExternalPoolAdapterTaskLedgerWriteDisposition, ExternalPoolAdapterTaskPollClaim,
        PollClaimProjection, CLAIM_STATUS_CLAIMED, CLAIM_STATUS_DELIVERY_OBSERVED,
        CLAIM_STATUS_PENDING,
    },
    write::{
        event_batch_needs_insert_on, event_needs_insert_on, event_poll_needs_insert_on,
        insert_external_pool_adapter_task_event_batch_on,
        insert_external_pool_adapter_task_event_on,
        insert_external_pool_adapter_task_event_poll_on,
    },
};

pub(in crate::store) struct ExternalPoolAdapterTaskEventIngressFactory<'a, T> {
    receipt: &'a ExternalPoolAdapterTaskExchangeReceiptEnvelope,
    semantic: &'a T,
    predecessor: &'a ExternalPoolAdapterTaskEventPollEnvelope,
    cleanup_expires_at: &'a str,
}

pub(in crate::store) struct SealedExternalPoolAdapterTaskEventIngress {
    batch: ExternalPoolAdapterTaskEventBatchEnvelope,
    events: Vec<ExternalPoolAdapterTaskEventEnvelope>,
    successor: Option<ExternalPoolAdapterTaskEventPollEnvelope>,
}

impl<'a, T: ExternalPoolAdapterBrokerTaskVerifiedObservation>
    ExternalPoolAdapterTaskEventIngressFactory<'a, T>
{
    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterTaskExchangeReceiptEnvelope {
        self.receipt
    }

    pub(in crate::store) fn semantic(&self) -> &T {
        self.semantic
    }

    pub(in crate::store) fn seal(
        self,
        batch: ExternalPoolAdapterTaskEventBatchEnvelope,
        events: Vec<ExternalPoolAdapterTaskEventEnvelope>,
        successor: Option<ExternalPoolAdapterTaskEventPollEnvelope>,
    ) -> Result<SealedExternalPoolAdapterTaskEventIngress> {
        self.semantic
            .validate_event_ingress(&batch, &events, successor.as_ref())?;
        validate_task_production_event_batch(&batch)?;
        ensure!(
            self.receipt.receipt.identity.operation_kind == "authenticated_events"
                && batch.batch.exchange_receipt_id == self.receipt.exchange_receipt_id
                && batch.batch.exchange_receipt_digest == self.receipt.exchange_receipt_digest
                && batch.batch.authenticated_observation_sha256
                    == self.receipt.receipt.semantic_observation_sha256
                && batch.batch.event_count == u64::try_from(events.len())?
                && batch.batch.event_count <= TASK_PRODUCTION_MAX_EVENTS_PER_BATCH,
            "V278 event batch does not bind the exact receipt and bounded inventory"
        );
        for (index, event) in events.iter().enumerate() {
            validate_task_production_event(event)?;
            ensure!(
                event.event.event_batch_id == batch.event_batch_id
                    && event.event.event_batch_digest == batch.event_batch_digest
                    && event.event.event_ordinal == u64::try_from(index)? + 1
                    && batch.batch.event_roots.get(index) == Some(&event.event.event_root),
                "V278 event inventory is not exact and ordered"
            );
        }
        validate_event_successor(
            self.predecessor,
            &batch,
            successor.as_ref(),
            self.cleanup_expires_at,
        )?;
        Ok(SealedExternalPoolAdapterTaskEventIngress {
            batch,
            events,
            successor,
        })
    }
}

pub(in crate::store) fn insert_external_pool_adapter_task_event_ingress_on<
    'tx,
    'conn,
    T: ExternalPoolAdapterBrokerTaskVerifiedObservation,
>(
    connection: &'tx Transaction<'conn>,
    pending_receipt: PendingExternalPoolAdapterTaskReceiptIngress<'tx, 'conn, T>,
    claim: ExternalPoolAdapterTaskPollClaim,
    completed_at: &str,
    build: impl FnOnce(
        ExternalPoolAdapterTaskEventIngressFactory<'_, T>,
    ) -> Result<SealedExternalPoolAdapterTaskEventIngress>,
) -> Result<ExternalPoolAdapterTaskEventBatchEnvelope> {
    ensure!(
        pending_receipt.disposition() == ExternalPoolAdapterTaskLedgerWriteDisposition::Inserted,
        "fresh V278 event ingress requires a freshly inserted receipt"
    );
    let cleanup_expires_at = pending_receipt.cleanup_expires_at().to_string();
    let (receipt, semantic, obligation) = pending_receipt.into_parts_on(connection)?;
    ensure!(
        receipt.receipt.identity.operation_kind == "authenticated_events"
            && receipt.receipt.identity.source.source_kind == "event_poll"
            && receipt.receipt.identity.source.source_id == claim.poll_id
            && receipt.receipt.identity.source.source_digest == claim.poll_digest,
        "V278 authenticated-events receipt does not close the claimed poll"
    );
    let predecessor = read_event_poll_on(connection, &claim.poll_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 event poll disappeared before ingress"))?;
    let sealed = build(ExternalPoolAdapterTaskEventIngressFactory {
        receipt: &receipt,
        semantic: &semantic,
        predecessor: &predecessor.envelope,
        cleanup_expires_at: &cleanup_expires_at,
    })?;
    let batch_needs_insert = event_batch_needs_insert_on(connection, &sealed.batch)?;
    let event_insert_inventory = sealed
        .events
        .iter()
        .map(|event| event_needs_insert_on(connection, event))
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        event_insert_inventory
            .iter()
            .all(|needs_insert| *needs_insert == batch_needs_insert),
        "V278 event batch cannot mix exact replay with fresh event writes"
    );
    ensure!(
        batch_needs_insert,
        "fresh V278 event ingress cannot reuse a committed event inventory"
    );
    let mut pending_writes = Vec::with_capacity(sealed.events.len() + 2);
    pending_writes.push(ExternalPoolAdapterTaskReachabilityPendingWrite::new(
        ExternalPoolAdapterTaskReachabilityPendingWriteKind::EventBatch,
        event_batch_values(&sealed.batch)?,
    )?);
    for (event, needs_insert) in sealed.events.iter().zip(&event_insert_inventory) {
        ensure!(*needs_insert, "fresh V278 event inventory lost an event");
        pending_writes.push(ExternalPoolAdapterTaskReachabilityPendingWrite::new(
            ExternalPoolAdapterTaskReachabilityPendingWriteKind::Event,
            event_values(event)?,
        )?);
    }
    let cas_values = event_poll_cas_values(&claim, completed_at)?;
    pending_writes.push(ExternalPoolAdapterTaskReachabilityPendingWrite::new(
        ExternalPoolAdapterTaskReachabilityPendingWriteKind::EventPollCas,
        cas_values.clone(),
    )?);
    if let Some(successor) = &sealed.successor {
        ensure!(
            event_poll_needs_insert_on(connection, successor)?,
            "fresh V278 event ingress cannot reuse a successor poll"
        );
        pending_writes.push(ExternalPoolAdapterTaskReachabilityPendingWrite::new(
            ExternalPoolAdapterTaskReachabilityPendingWriteKind::EventPoll,
            event_poll_values(successor, &initial_claim())?,
        )?);
    }
    let pending = ExternalPoolAdapterTaskReachabilityPendingPlan::new(pending_writes)?;
    let pending =
        install_external_pool_adapter_task_reachability_pending_plan_on(connection, pending)?;

    insert_external_pool_adapter_task_event_batch_on(connection, Some(&pending), &sealed.batch)?;
    for event in &sealed.events {
        insert_external_pool_adapter_task_event_on(connection, Some(&pending), event)?;
    }
    complete_event_poll_on(connection, &claim, completed_at)?;
    if let Some(successor) = &sealed.successor {
        insert_external_pool_adapter_task_event_poll_on(connection, Some(&pending), successor)?;
    }
    pending.ensure_fully_consumed()?;
    obligation.resolve(connection)?;
    Ok(sealed.batch)
}

pub(in crate::store) fn read_external_pool_adapter_task_event_ingress_replay_on<
    'tx,
    'conn,
    T: ExternalPoolAdapterBrokerTaskVerifiedObservation,
>(
    connection: &'tx Transaction<'conn>,
    pending_receipt: PendingExternalPoolAdapterTaskReceiptIngress<'tx, 'conn, T>,
    build: impl FnOnce(
        ExternalPoolAdapterTaskEventIngressFactory<'_, T>,
    ) -> Result<SealedExternalPoolAdapterTaskEventIngress>,
) -> Result<ExternalPoolAdapterTaskEventBatchEnvelope> {
    ensure!(
        pending_receipt.disposition() == ExternalPoolAdapterTaskLedgerWriteDisposition::ExactReplay,
        "V278 event replay requires an exact durable receipt"
    );
    let cleanup_expires_at = pending_receipt.cleanup_expires_at().to_string();
    let (receipt, semantic, obligation) = pending_receipt.into_parts_on(connection)?;
    let source = &receipt.receipt.identity.source;
    ensure!(
        receipt.receipt.identity.operation_kind == "authenticated_events"
            && source.source_kind == "event_poll",
        "V278 event replay receipt has the wrong durable source"
    );
    let poll = read_event_poll_on(connection, &source.source_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 replayed event poll disappeared"))?;
    ensure!(
        poll.envelope.event_poll_digest == source.source_digest
            && poll.claim.status == CLAIM_STATUS_DELIVERY_OBSERVED
            && poll.claim.owner_id.is_none()
            && poll.claim.token_digest.is_none()
            && poll.claim.expires_at.is_none(),
        "V278 replayed event inventory lacks exact completed poll readback"
    );
    let sealed = build(ExternalPoolAdapterTaskEventIngressFactory {
        receipt: &receipt,
        semantic: &semantic,
        predecessor: &poll.envelope,
        cleanup_expires_at: &cleanup_expires_at,
    })?;
    ensure!(
        !event_batch_needs_insert_on(connection, &sealed.batch)?
            && sealed
                .events
                .iter()
                .map(|event| event_needs_insert_on(connection, event))
                .collect::<Result<Vec<_>>>()?
                .iter()
                .all(|needs_insert| !needs_insert),
        "V278 event replay does not match one complete durable inventory"
    );
    let durable_successor = durable_event_successor_on(connection, &poll.envelope)?;
    ensure!(
        durable_successor.as_ref() == sealed.successor.as_ref(),
        "V278 event replay successor differs from durable cursor disposition"
    );
    obligation.resolve(connection)?;
    Ok(sealed.batch)
}

fn validate_event_successor(
    predecessor: &ExternalPoolAdapterTaskEventPollEnvelope,
    batch: &ExternalPoolAdapterTaskEventBatchEnvelope,
    successor: Option<&ExternalPoolAdapterTaskEventPollEnvelope>,
    cleanup_expires_at: &str,
) -> Result<()> {
    let terminal = batch.batch.remote.remote_execution_state == "terminal_after_run";
    ensure!(
        terminal == successor.is_none(),
        "V278 event remote state does not uniquely classify successor polling"
    );
    let Some(successor) = successor else {
        return Ok(());
    };
    validate_task_production_event_poll(successor)?;
    let expected_ordinal = predecessor
        .poll
        .lineage
        .poll_ordinal
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("V278 event poll ordinal overflow"))?;
    ensure!(
        successor.poll.lineage.predecessor_id.as_deref()
            == Some(predecessor.event_poll_id.as_str())
            && successor.poll.lineage.predecessor_digest.as_deref()
                == Some(predecessor.event_poll_digest.as_str())
            && successor.poll.lineage.poll_ordinal == expected_ordinal
            && successor.poll.source_exchange_receipt_id
                == predecessor.poll.source_exchange_receipt_id
            && successor.poll.source_exchange_receipt_digest
                == predecessor.poll.source_exchange_receipt_digest
            && successor.poll.command == predecessor.poll.command
            && successor.poll.remote == batch.batch.remote
            && successor.poll.requested_cursor == batch.batch.cursor_after
            && successor.poll.created_at == batch.batch.recorded_at
            && successor.poll.not_before.as_str() <= batch.batch.recorded_at.as_str()
            && batch.batch.recorded_at.as_str() < successor.poll.not_after.as_str()
            && successor.poll.not_after.as_str() <= cleanup_expires_at,
        "V278 event successor does not bind the exact completed cursor and cleanup window"
    );
    Ok(())
}

fn durable_event_successor_on(
    connection: &rusqlite::Connection,
    predecessor: &ExternalPoolAdapterTaskEventPollEnvelope,
) -> Result<Option<ExternalPoolAdapterTaskEventPollEnvelope>> {
    let id = connection
        .query_row(
            "SELECT event_poll_id FROM compute_external_pool_adapter_task_event_polls
              WHERE predecessor_event_poll_id=?1 AND predecessor_event_poll_digest=?2",
            params![predecessor.event_poll_id, predecessor.event_poll_digest],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    id.map(|id| {
        read_event_poll_on(connection, &id)?
            .map(|stored| stored.envelope)
            .ok_or_else(|| anyhow::anyhow!("V278 event successor disappeared during readback"))
    })
    .transpose()
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

fn event_poll_cas_values(
    claim: &ExternalPoolAdapterTaskPollClaim,
    _completed_at: &str,
) -> Result<Vec<Value>> {
    let next_revision = claim
        .claim_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("V278 event poll completion revision overflow"))?;
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

fn complete_event_poll_on(
    connection: &rusqlite::Connection,
    claim: &ExternalPoolAdapterTaskPollClaim,
    completed_at: &str,
) -> Result<()> {
    let next_revision = claim
        .claim_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("V278 event poll completion revision overflow"))?;
    let before = read_event_poll_on(connection, &claim.poll_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 event poll disappeared before completion"))?;
    ensure!(
        before.claim.status == CLAIM_STATUS_CLAIMED
            && before.claim.revision == claim.claim_revision
            && before.claim.generation == claim.claim_generation
            && before.claim.owner_id.as_deref() == Some(claim.claim_owner_id.as_str())
            && before.claim.token_digest.as_deref()
                == Some(hash_token(&claim.raw_claim_token).as_str())
            && before.claim.expires_at.as_deref() == Some(claim.claim_expires_at.as_str())
            && before.envelope.event_poll_digest == claim.poll_digest
            && completed_at < claim.claim_expires_at.as_str()
            && completed_at < before.envelope.poll.not_after.as_str(),
        "V278 event poll completion claim is stale"
    );
    let changed = connection.execute(
        "UPDATE compute_external_pool_adapter_task_event_polls
            SET claim_status='delivery_observed',claim_revision=claim_revision+1,
                claim_owner_id=NULL,claim_token_digest=NULL,claim_expires_at=NULL
          WHERE event_poll_id=?1 AND event_poll_digest=?2 AND claim_status='claimed'
            AND claim_revision=?3 AND claim_generation=?4 AND claim_owner_id=?5
            AND claim_token_digest=?6 AND claim_expires_at=?7 AND ?8<claim_expires_at
            AND EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_event_batches batch
                         WHERE batch.event_poll_id=?1 AND batch.event_poll_digest=?2
                           AND (SELECT count(*) FROM compute_external_pool_adapter_task_events event
                                WHERE event.event_batch_id=batch.event_batch_id)=batch.event_count)",
        params![
            claim.poll_id,
            claim.poll_digest,
            i64::try_from(claim.claim_revision)?,
            i64::try_from(claim.claim_generation)?,
            claim.claim_owner_id,
            hash_token(&claim.raw_claim_token),
            claim.claim_expires_at,
            completed_at,
        ],
    )?;
    ensure!(changed == 1, "V278 event poll completion CAS lost");
    let after = read_event_poll_on(connection, &claim.poll_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 event poll disappeared after completion"))?;
    ensure!(
        after.claim.status == CLAIM_STATUS_DELIVERY_OBSERVED
            && after.claim.revision == next_revision
            && after.claim.generation == claim.claim_generation
            && after.claim.owner_id.is_none()
            && after.claim.token_digest.is_none()
            && after.claim.expires_at.is_none(),
        "V278 event poll completion readback is not exact"
    );
    Ok(())
}
