//! Ordinal-one event poll derived from one authenticated commit receipt.

use anyhow::{ensure, Result};
use rusqlite::Transaction;

use crate::compute_federation::{
    external_pool_adapter_broker_tls::ExternalPoolAdapterBrokerTaskVerifiedObservation,
    external_pool_adapter_task_protocol_production::{
        validate_task_production_event_poll, ExternalPoolAdapterTaskEventPollEnvelope,
        ExternalPoolAdapterTaskExchangeReceiptEnvelope,
    },
};

use super::{
    install_external_pool_adapter_task_reachability_pending_plan_on,
    mapping::event_poll_values,
    reachability_pending_plan::{
        ExternalPoolAdapterTaskReachabilityPendingPlan,
        ExternalPoolAdapterTaskReachabilityPendingWrite,
        ExternalPoolAdapterTaskReachabilityPendingWriteKind,
    },
    receipt_ingress::PendingExternalPoolAdapterTaskReceiptIngress,
    types::{
        ExternalPoolAdapterTaskLedgerWriteDisposition, PollClaimProjection, CLAIM_STATUS_PENDING,
    },
    write::{event_poll_needs_insert_on, insert_external_pool_adapter_task_event_poll_on},
};

pub(in crate::store) struct ExternalPoolAdapterTaskFirstEventPollFactory<'a, T> {
    receipt: &'a ExternalPoolAdapterTaskExchangeReceiptEnvelope,
    semantic: &'a T,
    cleanup_expires_at: &'a str,
}

impl<'a, T: ExternalPoolAdapterBrokerTaskVerifiedObservation>
    ExternalPoolAdapterTaskFirstEventPollFactory<'a, T>
{
    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterTaskExchangeReceiptEnvelope {
        self.receipt
    }

    pub(in crate::store) fn semantic(&self) -> &T {
        self.semantic
    }

    pub(in crate::store) fn seal(
        self,
        poll: ExternalPoolAdapterTaskEventPollEnvelope,
    ) -> Result<ExternalPoolAdapterTaskEventPollEnvelope> {
        self.semantic.validate_event_poll(&poll)?;
        validate_first_event_poll(
            self.receipt,
            "idempotent_commit",
            self.cleanup_expires_at,
            &poll,
        )?;
        Ok(poll)
    }
}

pub(super) fn validate_first_event_poll(
    receipt_envelope: &ExternalPoolAdapterTaskExchangeReceiptEnvelope,
    expected_operation: &str,
    cleanup_expires_at: &str,
    poll: &ExternalPoolAdapterTaskEventPollEnvelope,
) -> Result<()> {
    validate_task_production_event_poll(poll)?;
    let receipt = &receipt_envelope.receipt;
    let command = &poll.poll.command;
    ensure!(
        receipt.identity.operation_kind == expected_operation
            && poll.poll.lineage.predecessor_id.is_none()
            && poll.poll.lineage.predecessor_digest.is_none()
            && poll.poll.lineage.poll_ordinal == 1
            && poll.poll.source_exchange_receipt_id == receipt_envelope.exchange_receipt_id
            && poll.poll.source_exchange_receipt_digest == receipt_envelope.exchange_receipt_digest
            && poll.poll.authenticated_subject_sha256 == receipt.semantic_observation_sha256
            && poll.poll.requested_cursor.remote_sequence == 0
            && poll.poll.requested_cursor.previous_event_root.is_none()
            && command.command_id == receipt.identity.command.command_id
            && command.command_digest == receipt.identity.command.command_digest
            && command.outbox_id == receipt.identity.command.outbox_id
            && command.outbox_digest == receipt.identity.command.outbox_digest
            && command.send_attempt_id == receipt.identity.command.send_attempt_id
            && command.send_attempt_digest == receipt.identity.command.send_attempt_digest
            && command.route_authorization_id == receipt.identity.route.route_authorization_id
            && command.route_authorization_digest
                == receipt.identity.route.route_authorization_digest
            && command.executor_binding_digest == receipt.identity.executor_binding_digest
            && command.fencing_generation == receipt.identity.fencing_generation
            && command.fence_digest == receipt.identity.fence_digest
            && poll.poll.created_at == receipt.recorded_at
            && poll.poll.not_before.as_str() <= receipt.recorded_at.as_str()
            && receipt.recorded_at.as_str() < poll.poll.not_after.as_str()
            && poll.poll.not_after.as_str() <= cleanup_expires_at,
        "V278 first event poll does not bind its exact receipt and cleanup window"
    );
    Ok(())
}

pub(in crate::store) fn insert_first_external_pool_adapter_task_event_poll_on<
    'tx,
    'conn,
    T: ExternalPoolAdapterBrokerTaskVerifiedObservation,
>(
    connection: &'tx Transaction<'conn>,
    pending_receipt: PendingExternalPoolAdapterTaskReceiptIngress<'tx, 'conn, T>,
    build: impl FnOnce(
        ExternalPoolAdapterTaskFirstEventPollFactory<'_, T>,
    ) -> Result<ExternalPoolAdapterTaskEventPollEnvelope>,
) -> Result<ExternalPoolAdapterTaskEventPollEnvelope> {
    let disposition = pending_receipt.disposition();
    let cleanup_expires_at = pending_receipt.cleanup_expires_at().to_string();
    let (receipt, semantic, obligation) = pending_receipt.into_parts_on(connection)?;
    let poll = build(ExternalPoolAdapterTaskFirstEventPollFactory {
        receipt: &receipt,
        semantic: &semantic,
        cleanup_expires_at: &cleanup_expires_at,
    })?;
    let needs_insert = event_poll_needs_insert_on(connection, &poll)?;
    ensure!(
        needs_insert == (disposition == ExternalPoolAdapterTaskLedgerWriteDisposition::Inserted),
        "V278 first event poll mixes fresh receipt custody with durable replay"
    );
    let plan = needs_insert
        .then(|| {
            ExternalPoolAdapterTaskReachabilityPendingPlan::new(vec![
                ExternalPoolAdapterTaskReachabilityPendingWrite::new(
                    ExternalPoolAdapterTaskReachabilityPendingWriteKind::EventPoll,
                    event_poll_values(&poll, &initial_claim())?,
                )?,
            ])
        })
        .transpose()?;
    let guard = plan
        .map(|plan| {
            install_external_pool_adapter_task_reachability_pending_plan_on(connection, plan)
        })
        .transpose()?;
    insert_external_pool_adapter_task_event_poll_on(connection, guard.as_ref(), &poll)?;
    if let Some(guard) = guard {
        guard.ensure_fully_consumed()?;
    }
    obligation.resolve(connection)?;
    Ok(poll)
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
