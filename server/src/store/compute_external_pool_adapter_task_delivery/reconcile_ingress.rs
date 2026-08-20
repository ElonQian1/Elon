//! Reconcile receipt closure: exact predecessor CAS, then successor or terminal handoff.

use anyhow::{ensure, Result};
use std::marker::PhantomData;

use rusqlite::Transaction;

use crate::compute_federation::external_pool_adapter_broker_tls::ExternalPoolAdapterBrokerTaskVerifiedObservation;
use crate::compute_federation::external_pool_adapter_task_protocol_production::{
    validate_task_production_reconcile_poll, ExternalPoolAdapterTaskEventPollEnvelope,
    ExternalPoolAdapterTaskExchangeReceiptEnvelope, ExternalPoolAdapterTaskReconcilePollEnvelope,
};

use super::{
    first_event::validate_first_event_poll,
    ingress_obligation::PendingTaskIngressObligation,
    install_external_pool_adapter_task_reachability_pending_plan_on,
    mapping::{event_poll_values, reconcile_poll_values},
    reachability_pending_plan::{
        ExternalPoolAdapterTaskReachabilityPendingPlan,
        ExternalPoolAdapterTaskReachabilityPendingWrite,
        ExternalPoolAdapterTaskReachabilityPendingWriteKind,
    },
    read::read_reconcile_poll_on,
    receipt_ingress::PendingExternalPoolAdapterTaskReceiptIngress,
    reconcile_completion::{complete_reconcile_poll_on, reconcile_poll_cas_values},
    reconcile_source::reconcile_source_operation_on,
    types::{
        ExternalPoolAdapterTaskLedgerWriteDisposition, ExternalPoolAdapterTaskPollClaim,
        PollClaimProjection, CLAIM_STATUS_PENDING,
    },
    write::{
        event_poll_needs_insert_on, insert_external_pool_adapter_task_event_poll_on,
        insert_external_pool_adapter_task_reconcile_poll_on, reconcile_poll_needs_insert_on,
    },
};

pub(in crate::store) struct ExternalPoolAdapterTaskReconcileIngressFactory<'a, T> {
    receipt: &'a ExternalPoolAdapterTaskExchangeReceiptEnvelope,
    semantic: &'a T,
    predecessor: &'a ExternalPoolAdapterTaskReconcilePollEnvelope,
    cleanup_expires_at: &'a str,
    event_poll_allowed: bool,
    no_start_allowed: bool,
    terminal_ack_allowed: bool,
}

pub(in crate::store) enum ExternalPoolAdapterTaskReconcileIngressOutcome<'tx, 'conn, T> {
    Successor(ExternalPoolAdapterTaskReconcilePollEnvelope),
    EventPoll(ExternalPoolAdapterTaskEventPollEnvelope),
    NoStart(PendingExternalPoolAdapterTaskNoStartIngress<'tx, 'conn, T>),
    Terminal(PendingExternalPoolAdapterTaskTerminalIngress<'tx, 'conn, T>),
}

pub(in crate::store) struct PendingExternalPoolAdapterTaskTerminalIngress<'tx, 'conn, T> {
    receipt: ExternalPoolAdapterTaskExchangeReceiptEnvelope,
    semantic: T,
    cleanup_expires_at: String,
    obligation: PendingTaskIngressObligation,
    connection_key: usize,
    _transaction: PhantomData<&'tx Transaction<'conn>>,
}

pub(in crate::store) struct PendingExternalPoolAdapterTaskNoStartIngress<'tx, 'conn, T>(
    PendingExternalPoolAdapterTaskTerminalIngress<'tx, 'conn, T>,
);

pub(in crate::store) enum SealedReconcileClosure {
    Successor(ExternalPoolAdapterTaskReconcilePollEnvelope),
    EventPoll(ExternalPoolAdapterTaskEventPollEnvelope),
    NoStart,
    Terminal,
}

impl<'a, T: ExternalPoolAdapterBrokerTaskVerifiedObservation>
    ExternalPoolAdapterTaskReconcileIngressFactory<'a, T>
{
    pub(super) fn new(
        receipt: &'a ExternalPoolAdapterTaskExchangeReceiptEnvelope,
        semantic: &'a T,
        predecessor: &'a ExternalPoolAdapterTaskReconcilePollEnvelope,
        cleanup_expires_at: &'a str,
        event_poll_allowed: bool,
        no_start_allowed: bool,
        terminal_ack_allowed: bool,
    ) -> Self {
        Self {
            receipt,
            semantic,
            predecessor,
            cleanup_expires_at,
            event_poll_allowed,
            no_start_allowed,
            terminal_ack_allowed,
        }
    }

    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterTaskExchangeReceiptEnvelope {
        self.receipt
    }

    pub(in crate::store) fn semantic(&self) -> &T {
        self.semantic
    }

    pub(in crate::store) fn predecessor(&self) -> &ExternalPoolAdapterTaskReconcilePollEnvelope {
        self.predecessor
    }

    pub(in crate::store) fn seal_successor(
        self,
        successor: ExternalPoolAdapterTaskReconcilePollEnvelope,
    ) -> Result<SealedReconcileClosure> {
        self.semantic.validate_reconcile_poll(&successor)?;
        validate_task_production_reconcile_poll(&successor)?;
        let expected_ordinal = self
            .predecessor
            .poll
            .lineage
            .poll_ordinal
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("V278 reconcile poll ordinal overflow"))?;
        ensure!(
            successor.poll.lineage.predecessor_id.as_deref()
                == Some(self.predecessor.reconcile_poll_id.as_str())
                && successor.poll.lineage.predecessor_digest.as_deref()
                    == Some(self.predecessor.reconcile_poll_digest.as_str())
                && successor.poll.lineage.poll_ordinal == expected_ordinal
                && successor.poll.uncertain_exchange_attempt_id
                    == self.receipt.receipt.exchange_attempt_id
                && successor.poll.uncertain_exchange_attempt_digest
                    == self.receipt.receipt.exchange_attempt_digest
                && successor.poll.command == self.predecessor.poll.command
                && successor.poll.authenticated_subject_sha256.as_deref()
                    == Some(self.receipt.receipt.semantic_observation_sha256.as_str()),
            "V278 reconcile successor does not bind the exact predecessor receipt"
        );
        Ok(SealedReconcileClosure::Successor(successor))
    }

    pub(in crate::store) fn seal_terminal(self) -> Result<SealedReconcileClosure> {
        ensure!(
            self.terminal_ack_allowed,
            "V278 reconcile source cannot produce a new accepted terminal ACK"
        );
        Ok(SealedReconcileClosure::Terminal)
    }

    pub(in crate::store) fn seal_no_start(self) -> Result<SealedReconcileClosure> {
        ensure!(
            self.no_start_allowed,
            "V278 reconcile source cannot establish terminal no-start"
        );
        Ok(SealedReconcileClosure::NoStart)
    }

    pub(in crate::store) fn seal_event_poll(
        self,
        poll: ExternalPoolAdapterTaskEventPollEnvelope,
    ) -> Result<SealedReconcileClosure> {
        ensure!(
            self.event_poll_allowed,
            "V278 reconcile source operation cannot start event polling"
        );
        self.semantic.validate_event_poll(&poll)?;
        validate_first_event_poll(self.receipt, "reconcile", self.cleanup_expires_at, &poll)?;
        Ok(SealedReconcileClosure::EventPoll(poll))
    }
}

impl<'tx, 'conn, T> PendingExternalPoolAdapterTaskTerminalIngress<'tx, 'conn, T> {
    pub(super) fn new(
        transaction: &'tx Transaction<'conn>,
        receipt: ExternalPoolAdapterTaskExchangeReceiptEnvelope,
        semantic: T,
        cleanup_expires_at: String,
        obligation: PendingTaskIngressObligation,
    ) -> Self {
        Self {
            receipt,
            semantic,
            cleanup_expires_at,
            obligation,
            connection_key: connection_key(transaction),
            _transaction: PhantomData,
        }
    }

    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterTaskExchangeReceiptEnvelope {
        &self.receipt
    }

    pub(in crate::store) fn semantic(&self) -> &T {
        &self.semantic
    }

    pub(super) fn into_parts_on(
        self,
        transaction: &Transaction<'_>,
    ) -> Result<(
        ExternalPoolAdapterTaskExchangeReceiptEnvelope,
        T,
        String,
        PendingTaskIngressObligation,
    )> {
        ensure!(
            self.connection_key == connection_key(transaction),
            "V278 terminal ingress changed SQLite transaction connection"
        );
        Ok((
            self.receipt,
            self.semantic,
            self.cleanup_expires_at,
            self.obligation,
        ))
    }
}

impl<'tx, 'conn, T> PendingExternalPoolAdapterTaskNoStartIngress<'tx, 'conn, T> {
    pub(super) fn new(inner: PendingExternalPoolAdapterTaskTerminalIngress<'tx, 'conn, T>) -> Self {
        Self(inner)
    }

    pub(super) fn into_inner(self) -> PendingExternalPoolAdapterTaskTerminalIngress<'tx, 'conn, T> {
        self.0
    }
}

pub(in crate::store) fn close_external_pool_adapter_task_reconcile_ingress_on<
    'tx,
    'conn,
    T: ExternalPoolAdapterBrokerTaskVerifiedObservation,
>(
    connection: &'tx Transaction<'conn>,
    pending_receipt: PendingExternalPoolAdapterTaskReceiptIngress<'tx, 'conn, T>,
    claim: ExternalPoolAdapterTaskPollClaim,
    completed_at: &str,
    classify: impl FnOnce(
        ExternalPoolAdapterTaskReconcileIngressFactory<'_, T>,
    ) -> Result<SealedReconcileClosure>,
) -> Result<ExternalPoolAdapterTaskReconcileIngressOutcome<'tx, 'conn, T>> {
    ensure!(
        pending_receipt.disposition() == ExternalPoolAdapterTaskLedgerWriteDisposition::Inserted,
        "fresh V278 reconcile ingress requires a freshly inserted receipt"
    );
    let cleanup_expires_at = pending_receipt.cleanup_expires_at().to_string();
    let (receipt, semantic, obligation) = pending_receipt.into_parts_on(connection)?;
    let predecessor = read_reconcile_poll_on(connection, &claim.poll_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 reconcile poll disappeared before ingress"))?;
    ensure!(
        receipt.receipt.identity.operation_kind == "reconcile"
            && receipt.receipt.identity.source.source_kind == "reconcile_poll"
            && receipt.receipt.identity.source.source_id == claim.poll_id
            && receipt.receipt.identity.source.source_digest == claim.poll_digest
            && predecessor.envelope.reconcile_poll_digest == claim.poll_digest,
        "V278 reconcile receipt does not close the exact claimed poll"
    );
    let (source_operation, accepted_ack_exists) =
        reconcile_source_operation_on(connection, &receipt)?;
    let closure = classify(ExternalPoolAdapterTaskReconcileIngressFactory::new(
        &receipt,
        &semantic,
        &predecessor.envelope,
        &cleanup_expires_at,
        source_operation == "commit" && accepted_ack_exists,
        source_operation == "cancel" && !accepted_ack_exists,
        source_operation == "prepare" && !accepted_ack_exists,
    ))?;
    let cas_values = reconcile_poll_cas_values(&claim, completed_at)?;
    let mut writes = vec![ExternalPoolAdapterTaskReachabilityPendingWrite::new(
        ExternalPoolAdapterTaskReachabilityPendingWriteKind::ReconcilePollCas,
        cas_values.clone(),
    )?];
    let successor_needs_insert = match &closure {
        SealedReconcileClosure::Successor(successor) => {
            reconcile_poll_needs_insert_on(connection, successor)?
        }
        SealedReconcileClosure::EventPoll(poll) => {
            ensure!(
                source_operation == "commit" && accepted_ack_exists,
                "V278 reconcile event poll does not match original ACK state"
            );
            event_poll_needs_insert_on(connection, poll)?
        }
        SealedReconcileClosure::NoStart => false,
        SealedReconcileClosure::Terminal => false,
    };
    ensure!(
        matches!(
            &closure,
            SealedReconcileClosure::Terminal | SealedReconcileClosure::NoStart
        ) || successor_needs_insert,
        "V278 fresh reconcile closure cannot reuse a pre-existing successor"
    );
    if successor_needs_insert {
        if let SealedReconcileClosure::Successor(successor) = &closure {
            writes.push(ExternalPoolAdapterTaskReachabilityPendingWrite::new(
                ExternalPoolAdapterTaskReachabilityPendingWriteKind::ReconcilePoll,
                reconcile_poll_values(successor, &initial_claim())?,
            )?);
        } else if let SealedReconcileClosure::EventPoll(poll) = &closure {
            writes.push(ExternalPoolAdapterTaskReachabilityPendingWrite::new(
                ExternalPoolAdapterTaskReachabilityPendingWriteKind::EventPoll,
                event_poll_values(poll, &initial_claim())?,
            )?);
        }
    }
    let pending = ExternalPoolAdapterTaskReachabilityPendingPlan::new(writes)?;
    let pending =
        install_external_pool_adapter_task_reachability_pending_plan_on(connection, pending)?;
    complete_reconcile_poll_on(connection, &claim, completed_at)?;
    let outcome = match closure {
        SealedReconcileClosure::Successor(successor) => {
            insert_external_pool_adapter_task_reconcile_poll_on(
                connection,
                successor_needs_insert.then_some(&pending),
                &successor,
            )?;
            obligation.resolve(connection)?;
            ExternalPoolAdapterTaskReconcileIngressOutcome::Successor(successor)
        }
        SealedReconcileClosure::EventPoll(poll) => {
            insert_external_pool_adapter_task_event_poll_on(
                connection,
                successor_needs_insert.then_some(&pending),
                &poll,
            )?;
            obligation.resolve(connection)?;
            ExternalPoolAdapterTaskReconcileIngressOutcome::EventPoll(poll)
        }
        SealedReconcileClosure::NoStart => ExternalPoolAdapterTaskReconcileIngressOutcome::NoStart(
            PendingExternalPoolAdapterTaskNoStartIngress::new(
                PendingExternalPoolAdapterTaskTerminalIngress::new(
                    connection,
                    receipt,
                    semantic,
                    cleanup_expires_at,
                    obligation,
                ),
            ),
        ),
        SealedReconcileClosure::Terminal => {
            ExternalPoolAdapterTaskReconcileIngressOutcome::Terminal(
                PendingExternalPoolAdapterTaskTerminalIngress::new(
                    connection,
                    receipt,
                    semantic,
                    cleanup_expires_at,
                    obligation,
                ),
            )
        }
    };
    pending.ensure_fully_consumed()?;
    Ok(outcome)
}

fn connection_key(connection: &rusqlite::Connection) -> usize {
    // SAFETY: the handle is used only as identity while the transaction borrow is alive.
    unsafe { connection.handle() as usize }
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
