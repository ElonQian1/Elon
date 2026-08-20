//! Same-transaction V213 send + V273 exchange-attempt paired commit.

use anyhow::{ensure, Result};
use rusqlite::Transaction;

use crate::store::compute_attempt_start_outbox::{
    finish_prepared_send_started_on, insert_prepared_send_started_on, prepare_send_started_at_on,
    prepared_send_attempt_envelope, prepared_send_attempt_values, prepared_send_outbox_cas_values,
    CommittedStartSendAuthority, PreparedStartSendRequest, StartOutboxClaimHandle,
};

use super::{
    super::compute_external_pool_adapter_runtime_bundle::ReprovedExternalPoolAdapterRouteAndActiveSuccessorAuthority,
    historical_cleanup::HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority,
    install_external_pool_adapter_task_reachability_pending_plan_on,
    mapping::exchange_attempt_values,
    reachability_pending_plan::{
        ExternalPoolAdapterTaskReachabilityPendingPlan,
        ExternalPoolAdapterTaskReachabilityPendingWrite,
        ExternalPoolAdapterTaskReachabilityPendingWriteKind,
    },
    sealed::{
        ExternalPoolAdapterTaskExchangeAttemptFactory, SealedExternalPoolAdapterTaskExchangeAttempt,
    },
    types::CommittedExternalPoolAdapterTaskOutbound,
    write::insert_external_pool_adapter_task_exchange_attempt_on,
};

pub(in crate::store) fn record_external_pool_adapter_task_outbound_on<
    'authority,
    'tx,
    'conn,
    'runtime,
>(
    connection: &'tx Transaction<'conn>,
    authority: &ReprovedExternalPoolAdapterRouteAndActiveSuccessorAuthority<
        'authority,
        'tx,
        'conn,
        'runtime,
    >,
    claim: StartOutboxClaimHandle,
    request: PreparedStartSendRequest,
    build_attempt: impl FnOnce(
        ExternalPoolAdapterTaskExchangeAttemptFactory<'_>,
    ) -> Result<SealedExternalPoolAdapterTaskExchangeAttempt>,
) -> Result<CommittedExternalPoolAdapterTaskOutbound> {
    let route = authority.route_authorization().envelope();
    ensure!(
        claim.provider_id() == route.authorization.provider.provider_id
            && claim.route_authorization_id() == route.route_authorization_id
            && claim.route_authorization_digest() == route.route_authorization_digest,
        "V278 outbound claim does not bind the current composite route authority"
    );
    record_external_pool_adapter_task_outbound_at_on(
        connection,
        claim,
        request,
        authority.checked_at(),
        build_attempt,
    )
}

pub(in crate::store) fn record_historical_external_pool_adapter_task_cleanup_outbound_on<
    'tx,
    'conn,
>(
    connection: &'tx Transaction<'conn>,
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn>,
    claim: StartOutboxClaimHandle,
    request: PreparedStartSendRequest,
    build_attempt: impl FnOnce(
        ExternalPoolAdapterTaskExchangeAttemptFactory<'_>,
    ) -> Result<SealedExternalPoolAdapterTaskExchangeAttempt>,
) -> Result<CommittedExternalPoolAdapterTaskOutbound> {
    let identity = &authority.exchange_attempt().attempt.identity;
    ensure!(
        claim.operation_kind() == "cancel"
            && claim.outbox_id() != identity.command.outbox_id
            && claim.subject_outbox_id() == Some(identity.command.outbox_id.as_str())
            && claim.command_id() == identity.command.command_id
            && claim.command_digest() == identity.command.command_digest
            && claim.provider_id() == identity.adapter.provider_id
            && claim.route_authorization_id() == identity.route.route_authorization_id
            && claim.route_authorization_digest() == identity.route.route_authorization_digest,
        "V278 historical cleanup claim does not descend from the exact durable exchange"
    );
    record_external_pool_adapter_task_outbound_at_on(
        connection,
        claim,
        request,
        authority.checked_at(),
        build_attempt,
    )
}

fn record_external_pool_adapter_task_outbound_at_on(
    connection: &Transaction<'_>,
    claim: StartOutboxClaimHandle,
    request: PreparedStartSendRequest,
    checked_at: &str,
    build_attempt: impl FnOnce(
        ExternalPoolAdapterTaskExchangeAttemptFactory<'_>,
    ) -> Result<SealedExternalPoolAdapterTaskExchangeAttempt>,
) -> Result<CommittedExternalPoolAdapterTaskOutbound> {
    let mutation = prepare_send_started_at_on(connection, &claim, &request, checked_at)?;
    let sealed_attempt = build_attempt(ExternalPoolAdapterTaskExchangeAttemptFactory::new(
        prepared_send_attempt_envelope(&mutation),
    ))?;
    let attempt_values = exchange_attempt_values(sealed_attempt.envelope())?;
    let send_values = prepared_send_attempt_values(&mutation);
    let cas_values = prepared_send_outbox_cas_values(&mutation, &claim);
    let plan = ExternalPoolAdapterTaskReachabilityPendingPlan::new(vec![
        ExternalPoolAdapterTaskReachabilityPendingWrite::new(
            ExternalPoolAdapterTaskReachabilityPendingWriteKind::StartSendAttempt,
            send_values.clone(),
        )?,
        ExternalPoolAdapterTaskReachabilityPendingWrite::new(
            ExternalPoolAdapterTaskReachabilityPendingWriteKind::ExchangeAttempt,
            attempt_values,
        )?,
        ExternalPoolAdapterTaskReachabilityPendingWrite::new(
            ExternalPoolAdapterTaskReachabilityPendingWriteKind::StartOutboxCas,
            cas_values.clone(),
        )?,
    ])?;
    let plan = install_external_pool_adapter_task_reachability_pending_plan_on(connection, plan)?;

    insert_prepared_send_started_on(connection, &mutation)?;
    insert_external_pool_adapter_task_exchange_attempt_on(
        connection,
        Some(&plan),
        sealed_attempt.envelope(),
    )?;
    let send_attempt = finish_prepared_send_started_on(connection, &claim, mutation)?;
    plan.ensure_fully_consumed()?;

    let attempt = sealed_attempt.into_envelope();
    Ok(CommittedExternalPoolAdapterTaskOutbound::new(
        CommittedStartSendAuthority::from_external_pool_adapter_task(send_attempt, claim, request),
        attempt,
    ))
}
