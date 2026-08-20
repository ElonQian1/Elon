//! Historical reconcile/event poll claim to one durable V273 exchange attempt.

use anyhow::{ensure, Result};
use rusqlite::Transaction;

use crate::{
    compute_federation::external_pool_adapter_task_protocol_production::{
        ExternalPoolAdapterTaskCommandBinding, ExternalPoolAdapterTaskEventPollEnvelope,
        ExternalPoolAdapterTaskExchangeAttemptEnvelope, ExternalPoolAdapterTaskPollCommandBinding,
        ExternalPoolAdapterTaskReconcilePollEnvelope,
    },
    store::hash_token,
};

use super::{
    historical_cleanup::HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority,
    install_external_pool_adapter_task_reachability_pending_plan_on,
    mapping::exchange_attempt_values,
    polls::ensure_poll_binds_history,
    reachability_pending_plan::{
        ExternalPoolAdapterTaskReachabilityPendingPlan,
        ExternalPoolAdapterTaskReachabilityPendingWrite,
        ExternalPoolAdapterTaskReachabilityPendingWriteKind,
    },
    read::{read_event_poll_on, read_reconcile_poll_on},
    types::{
        CommittedExternalPoolAdapterTaskPollExchange, ExternalPoolAdapterTaskPollClaim,
        PollClaimProjection, CLAIM_STATUS_CLAIMED,
    },
    write::{
        exchange_attempt_needs_insert_on, insert_external_pool_adapter_task_exchange_attempt_on,
    },
};

pub(in crate::store) struct ExternalPoolAdapterTaskPollExchangeAttemptFactory<'poll> {
    poll: PollEnvelopeRef<'poll>,
    started_at: &'poll str,
}

pub(in crate::store) struct SealedExternalPoolAdapterTaskPollExchangeAttempt {
    envelope: ExternalPoolAdapterTaskExchangeAttemptEnvelope,
}

#[derive(Clone, Copy)]
enum PollEnvelopeRef<'poll> {
    Reconcile(&'poll ExternalPoolAdapterTaskReconcilePollEnvelope),
    Event(&'poll ExternalPoolAdapterTaskEventPollEnvelope),
}

impl<'poll> ExternalPoolAdapterTaskPollExchangeAttemptFactory<'poll> {
    pub(in crate::store) fn reconcile_poll(
        &self,
    ) -> Option<&ExternalPoolAdapterTaskReconcilePollEnvelope> {
        match self.poll {
            PollEnvelopeRef::Reconcile(poll) => Some(poll),
            PollEnvelopeRef::Event(_) => None,
        }
    }

    pub(in crate::store) fn event_poll(&self) -> Option<&ExternalPoolAdapterTaskEventPollEnvelope> {
        match self.poll {
            PollEnvelopeRef::Reconcile(_) => None,
            PollEnvelopeRef::Event(poll) => Some(poll),
        }
    }

    pub(in crate::store) fn started_at(&self) -> &str {
        self.started_at
    }

    pub(in crate::store) fn seal(
        self,
        envelope: ExternalPoolAdapterTaskExchangeAttemptEnvelope,
    ) -> Result<SealedExternalPoolAdapterTaskPollExchangeAttempt> {
        let (operation, source_kind, source_id, source_digest, command, request_digest) =
            match self.poll {
                PollEnvelopeRef::Reconcile(poll) => (
                    "reconcile",
                    "reconcile_poll",
                    poll.reconcile_poll_id.as_str(),
                    poll.reconcile_poll_digest.as_str(),
                    &poll.poll.command,
                    poll.poll.request_digest.as_str(),
                ),
                PollEnvelopeRef::Event(poll) => (
                    "authenticated_events",
                    "event_poll",
                    poll.event_poll_id.as_str(),
                    poll.event_poll_digest.as_str(),
                    &poll.poll.command,
                    poll.poll.request_digest.as_str(),
                ),
            };
        let identity = &envelope.attempt.identity;
        ensure!(
            identity.operation_kind == operation
                && identity.source.source_kind == source_kind
                && identity.source.source_id == source_id
                && identity.source.source_digest == source_digest
                && envelope.attempt.started_at == self.started_at
                && identity.request_digest == request_digest
                && command_is_exact(&identity.command, command),
            "V278 poll exchange does not bind the exact claimed intent"
        );
        Ok(SealedExternalPoolAdapterTaskPollExchangeAttempt { envelope })
    }
}

pub(in crate::store) fn record_external_pool_adapter_task_reconcile_exchange_attempt_on<
    'tx,
    'conn,
>(
    connection: &'tx Transaction<'conn>,
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn>,
    claim: ExternalPoolAdapterTaskPollClaim,
    build: impl FnOnce(
        ExternalPoolAdapterTaskPollExchangeAttemptFactory<'_>,
    ) -> Result<SealedExternalPoolAdapterTaskPollExchangeAttempt>,
) -> Result<CommittedExternalPoolAdapterTaskPollExchange> {
    let poll = read_reconcile_poll_on(connection, &claim.poll_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 reconcile poll disappeared before exchange"))?;
    ensure_poll_binds_history(&poll.envelope.poll.command, authority)?;
    ensure_claim_is_exact(&poll.claim, &claim, authority.checked_at())?;
    record_poll_exchange_on(
        connection,
        authority,
        claim,
        PollEnvelopeRef::Reconcile(&poll.envelope),
        build,
    )
}

pub(in crate::store) fn record_external_pool_adapter_task_event_exchange_attempt_on<'tx, 'conn>(
    connection: &'tx Transaction<'conn>,
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn>,
    claim: ExternalPoolAdapterTaskPollClaim,
    build: impl FnOnce(
        ExternalPoolAdapterTaskPollExchangeAttemptFactory<'_>,
    ) -> Result<SealedExternalPoolAdapterTaskPollExchangeAttempt>,
) -> Result<CommittedExternalPoolAdapterTaskPollExchange> {
    let poll = read_event_poll_on(connection, &claim.poll_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 event poll disappeared before exchange"))?;
    ensure_poll_binds_history(&poll.envelope.poll.command, authority)?;
    ensure_claim_is_exact(&poll.claim, &claim, authority.checked_at())?;
    record_poll_exchange_on(
        connection,
        authority,
        claim,
        PollEnvelopeRef::Event(&poll.envelope),
        build,
    )
}

fn record_poll_exchange_on(
    connection: &Transaction<'_>,
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'_, '_>,
    claim: ExternalPoolAdapterTaskPollClaim,
    poll: PollEnvelopeRef<'_>,
    build: impl FnOnce(
        ExternalPoolAdapterTaskPollExchangeAttemptFactory<'_>,
    ) -> Result<SealedExternalPoolAdapterTaskPollExchangeAttempt>,
) -> Result<CommittedExternalPoolAdapterTaskPollExchange> {
    let sealed = build(ExternalPoolAdapterTaskPollExchangeAttemptFactory {
        poll,
        started_at: authority.checked_at(),
    })?;
    let envelope = sealed.envelope;
    let identity = &envelope.attempt.identity;
    let historical = &authority.exchange_attempt().attempt.identity;
    ensure!(
        identity.adapter == historical.adapter
            && identity.command.command_id == historical.command.command_id
            && identity.command.command_digest == historical.command.command_digest
            && identity.command.outbox_id == historical.command.outbox_id
            && identity.command.outbox_digest == historical.command.outbox_digest
            && identity.command.send_attempt_id == historical.command.send_attempt_id
            && identity.command.send_attempt_digest == historical.command.send_attempt_digest
            && identity.route == historical.route
            && identity.executor_binding_digest == historical.executor_binding_digest
            && identity.fencing_generation == historical.fencing_generation
            && identity.fence_digest == historical.fence_digest,
        "V278 poll exchange changed immutable execution roots"
    );
    if !exchange_attempt_needs_insert_on(connection, &envelope)? {
        insert_external_pool_adapter_task_exchange_attempt_on(connection, None, &envelope)?;
        return Ok(CommittedExternalPoolAdapterTaskPollExchange::new(
            envelope, claim,
        ));
    }
    let plan = ExternalPoolAdapterTaskReachabilityPendingPlan::new(vec![
        ExternalPoolAdapterTaskReachabilityPendingWrite::new(
            ExternalPoolAdapterTaskReachabilityPendingWriteKind::ExchangeAttempt,
            exchange_attempt_values(&envelope)?,
        )?,
    ])?;
    let plan = install_external_pool_adapter_task_reachability_pending_plan_on(connection, plan)?;
    insert_external_pool_adapter_task_exchange_attempt_on(connection, Some(&plan), &envelope)?;
    plan.ensure_fully_consumed()?;
    Ok(CommittedExternalPoolAdapterTaskPollExchange::new(
        envelope, claim,
    ))
}

fn ensure_claim_is_exact(
    stored: &PollClaimProjection,
    claim: &ExternalPoolAdapterTaskPollClaim,
    checked_at: &str,
) -> Result<()> {
    ensure!(
        stored.status == CLAIM_STATUS_CLAIMED
            && stored.revision == claim.claim_revision
            && stored.generation == claim.claim_generation
            && stored.owner_id.as_deref() == Some(claim.claim_owner_id.as_str())
            && stored.token_digest.as_deref() == Some(hash_token(&claim.raw_claim_token).as_str())
            && stored.expires_at.as_deref() == Some(claim.claim_expires_at.as_str())
            && checked_at < claim.claim_expires_at.as_str(),
        "V278 poll exchange claim is stale or unauthenticated"
    );
    Ok(())
}

fn command_is_exact(
    identity: &ExternalPoolAdapterTaskCommandBinding,
    command: &ExternalPoolAdapterTaskPollCommandBinding,
) -> bool {
    identity.command_id == command.command_id
        && identity.command_digest == command.command_digest
        && identity.outbox_id == command.outbox_id
        && identity.outbox_digest == command.outbox_digest
        && identity.send_attempt_id == command.send_attempt_id
        && identity.send_attempt_digest == command.send_attempt_digest
}
