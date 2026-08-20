//! First reconcile intent derived from one sealed cancel receipt.

use anyhow::{ensure, Result};
use rusqlite::Transaction;

use crate::compute_federation::{
    external_pool_adapter_broker_tls::ExternalPoolAdapterBrokerTaskVerifiedObservation,
    external_pool_adapter_task_protocol_production::{
        canonical_task_production_reconcile_poll_json_and_digest,
        ExternalPoolAdapterTaskPollCommandBinding, ExternalPoolAdapterTaskPollLineage,
        ExternalPoolAdapterTaskProductionBoundary, ExternalPoolAdapterTaskProductionEffects,
        ExternalPoolAdapterTaskProductionReadiness, ExternalPoolAdapterTaskReconcilePollEnvelope,
        ExternalPoolAdapterTaskReconcilePollIntent, TASK_PRODUCTION_CANONICALIZATION,
        TASK_PRODUCTION_DIGEST_ALGORITHM, TASK_PRODUCTION_NO_V213_AUTHORITY,
        TASK_PRODUCTION_RECONCILE_POLL_SCHEMA,
    },
};

use super::{
    first_reconcile::ExternalPoolAdapterTaskFirstReconcilePollRequest,
    historical_cleanup::HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority,
    install_external_pool_adapter_task_reachability_pending_plan_on,
    mapping::reconcile_poll_values,
    reachability_pending_plan::{
        ExternalPoolAdapterTaskReachabilityPendingPlan,
        ExternalPoolAdapterTaskReachabilityPendingWrite,
        ExternalPoolAdapterTaskReachabilityPendingWriteKind,
    },
    receipt_ingress::PendingExternalPoolAdapterTaskReceiptIngress,
    types::{
        ExternalPoolAdapterTaskLedgerWriteDisposition, PollClaimProjection, CLAIM_STATUS_PENDING,
    },
    write::{insert_external_pool_adapter_task_reconcile_poll_on, reconcile_poll_needs_insert_on},
};

pub(in crate::store) fn insert_first_external_pool_adapter_task_cancel_reconcile_poll_on<
    'tx,
    'conn,
    T: ExternalPoolAdapterBrokerTaskVerifiedObservation,
>(
    transaction: &'tx Transaction<'conn>,
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn>,
    pending: PendingExternalPoolAdapterTaskReceiptIngress<'tx, 'conn, T>,
    mut request: ExternalPoolAdapterTaskFirstReconcilePollRequest,
) -> Result<ExternalPoolAdapterTaskReconcilePollEnvelope> {
    let disposition = pending.disposition();
    let (receipt, semantic, obligation) = pending.into_parts_on(transaction)?;
    let attempt = authority.exchange_attempt();
    let identity = &attempt.attempt.identity;
    ensure!(
        identity.operation_kind == "cancel_no_start"
            && identity.source.source_kind == "start_outbox_send_attempt"
            && receipt.receipt.exchange_attempt_id == attempt.exchange_attempt_id
            && receipt.receipt.exchange_attempt_digest == attempt.exchange_attempt_digest
            && receipt.receipt.identity == attempt.attempt.identity
            && request.created_at == authority.checked_at()
            && request.not_before.as_str() <= authority.checked_at()
            && authority.checked_at() < request.not_after.as_str()
            && request.not_after.as_str() <= authority.cleanup_expires_at(),
        "V278 cancel first reconcile does not bind its sealed receipt and cleanup authority"
    );
    request.authenticated_subject_sha256 =
        Some(receipt.receipt.semantic_observation_sha256.clone());
    let mut envelope = ExternalPoolAdapterTaskReconcilePollEnvelope {
        schema: TASK_PRODUCTION_RECONCILE_POLL_SCHEMA.to_string(),
        reconcile_poll_id: format!(
            "external_pool_reconcile_{}_1",
            attempt.exchange_attempt_digest
        ),
        reconcile_poll_digest: String::new(),
        canonicalization: TASK_PRODUCTION_CANONICALIZATION.to_string(),
        digest_algorithm: TASK_PRODUCTION_DIGEST_ALGORITHM.to_string(),
        poll: ExternalPoolAdapterTaskReconcilePollIntent {
            lineage: ExternalPoolAdapterTaskPollLineage {
                predecessor_id: None,
                predecessor_digest: None,
                poll_ordinal: 1,
            },
            uncertain_exchange_attempt_id: attempt.exchange_attempt_id.clone(),
            uncertain_exchange_attempt_digest: attempt.exchange_attempt_digest.clone(),
            command: ExternalPoolAdapterTaskPollCommandBinding {
                command_id: identity.command.command_id.clone(),
                command_digest: identity.command.command_digest.clone(),
                outbox_id: identity.command.outbox_id.clone(),
                outbox_digest: identity.command.outbox_digest.clone(),
                send_attempt_id: identity.command.send_attempt_id.clone(),
                send_attempt_digest: identity.command.send_attempt_digest.clone(),
                route_authorization_id: identity.route.route_authorization_id.clone(),
                route_authorization_digest: identity.route.route_authorization_digest.clone(),
                executor_binding_digest: identity.executor_binding_digest.clone(),
                fencing_generation: identity.fencing_generation,
                fence_digest: identity.fence_digest.clone(),
            },
            remote: request.remote,
            authenticated_subject_sha256: request.authenticated_subject_sha256,
            request_digest: request.request_digest,
            not_before: request.not_before,
            not_after: request.not_after,
            created_at: request.created_at,
            boundary: ExternalPoolAdapterTaskProductionBoundary {
                authority_status: TASK_PRODUCTION_NO_V213_AUTHORITY.to_string(),
                effects: ExternalPoolAdapterTaskProductionEffects::none(),
                readiness: ExternalPoolAdapterTaskProductionReadiness::none(),
            },
        },
    };
    envelope.reconcile_poll_digest =
        canonical_task_production_reconcile_poll_json_and_digest(&envelope)?.1;
    semantic.validate_reconcile_poll(&envelope)?;
    let needs_insert = reconcile_poll_needs_insert_on(transaction, &envelope)?;
    ensure!(
        matches!(
            (disposition, needs_insert),
            (
                ExternalPoolAdapterTaskLedgerWriteDisposition::Inserted,
                true
            ) | (
                ExternalPoolAdapterTaskLedgerWriteDisposition::ExactReplay,
                false
            )
        ),
        "V278 cancel receipt/reconcile replay disposition is inconsistent"
    );
    if needs_insert {
        let values = reconcile_poll_values(&envelope, &initial_claim())?;
        let plan = ExternalPoolAdapterTaskReachabilityPendingPlan::new(vec![
            ExternalPoolAdapterTaskReachabilityPendingWrite::new(
                ExternalPoolAdapterTaskReachabilityPendingWriteKind::ReconcilePoll,
                values,
            )?,
        ])?;
        let plan =
            install_external_pool_adapter_task_reachability_pending_plan_on(transaction, plan)?;
        insert_external_pool_adapter_task_reconcile_poll_on(transaction, Some(&plan), &envelope)?;
        plan.ensure_fully_consumed()?;
    } else {
        insert_external_pool_adapter_task_reconcile_poll_on(transaction, None, &envelope)?;
    }
    obligation.resolve(transaction)?;
    Ok(envelope)
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
