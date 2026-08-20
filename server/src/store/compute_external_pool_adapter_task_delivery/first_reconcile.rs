//! First direct reconcile intent for one already-durable uncertain V273 exchange.

use anyhow::{ensure, Result};
use rusqlite::{params, Transaction};

use crate::compute_federation::external_pool_adapter_task_protocol_production::{
    canonical_task_production_reconcile_poll_json_and_digest,
    ExternalPoolAdapterTaskPollCommandBinding, ExternalPoolAdapterTaskPollLineage,
    ExternalPoolAdapterTaskProductionBoundary, ExternalPoolAdapterTaskProductionEffects,
    ExternalPoolAdapterTaskProductionReadiness, ExternalPoolAdapterTaskReconcilePollEnvelope,
    ExternalPoolAdapterTaskReconcilePollIntent, ExternalPoolAdapterTaskRemoteIdentity,
    TASK_PRODUCTION_CANONICALIZATION, TASK_PRODUCTION_DIGEST_ALGORITHM,
    TASK_PRODUCTION_NO_V213_AUTHORITY, TASK_PRODUCTION_RECONCILE_POLL_SCHEMA,
};

use super::{
    historical_cleanup::HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority,
    install_external_pool_adapter_task_reachability_pending_plan_on,
    mapping::reconcile_poll_values,
    reachability_pending_plan::{
        ExternalPoolAdapterTaskReachabilityPendingPlan,
        ExternalPoolAdapterTaskReachabilityPendingWrite,
        ExternalPoolAdapterTaskReachabilityPendingWriteKind,
    },
    types::{PollClaimProjection, CLAIM_STATUS_PENDING},
    write::{insert_external_pool_adapter_task_reconcile_poll_on, reconcile_poll_needs_insert_on},
};

pub(in crate::store) struct ExternalPoolAdapterTaskFirstReconcilePollRequest {
    pub(super) request_digest: String,
    pub(super) remote: ExternalPoolAdapterTaskRemoteIdentity,
    pub(super) authenticated_subject_sha256: Option<String>,
    pub(super) not_before: String,
    pub(super) not_after: String,
    pub(super) created_at: String,
}

impl ExternalPoolAdapterTaskFirstReconcilePollRequest {
    pub(in crate::store) fn new(
        request_digest: String,
        remote: ExternalPoolAdapterTaskRemoteIdentity,
        authenticated_subject_sha256: Option<String>,
        not_before: String,
        not_after: String,
        created_at: String,
    ) -> Self {
        Self {
            request_digest,
            remote,
            authenticated_subject_sha256,
            not_before,
            not_after,
            created_at,
        }
    }
}

pub(in crate::store) fn insert_first_external_pool_adapter_task_reconcile_poll_on<'tx, 'conn>(
    connection: &'tx Transaction<'conn>,
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn>,
    request: ExternalPoolAdapterTaskFirstReconcilePollRequest,
) -> Result<ExternalPoolAdapterTaskReconcilePollEnvelope> {
    let attempt = authority.exchange_attempt();
    ensure!(
        matches!(
            attempt.attempt.identity.operation_kind.as_str(),
            "prepare" | "idempotent_commit"
        ) && attempt.attempt.identity.source.source_kind == "start_outbox_send_attempt",
        "V278 first reconcile source is not an outbound execution exchange"
    );
    ensure!(
        request.created_at == authority.checked_at()
            && request.not_before.as_str() <= authority.checked_at()
            && authority.checked_at() < request.not_after.as_str()
            && request.not_after.as_str() <= authority.cleanup_expires_at(),
        "V278 first reconcile intent does not use its historical reproof time"
    );
    let identity = &attempt.attempt.identity;
    let is_uncertain = connection.query_row(
        "SELECT EXISTS(
            SELECT 1
              FROM compute_attempt_start_send_attempts send
              JOIN compute_attempt_start_outbox outbox
                ON outbox.outbox_id=send.outbox_id
               AND outbox.outbox_digest=send.outbox_digest
             WHERE send.send_attempt_id=?1 AND send.send_attempt_digest=?2
               AND send.command_id=?3 AND send.command_digest=?4
               AND send.outbox_id=?5 AND send.outbox_digest=?6
               AND send.route_authorization_id=?7
               AND send.route_authorization_digest=?8
               AND outbox.state='in_flight_unknown'
               AND outbox.attempt_count=send.attempt_no
               AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_start_remote_observations observation
                     WHERE observation.send_attempt_id=send.send_attempt_id))",
        params![
            identity.command.send_attempt_id,
            identity.command.send_attempt_digest,
            identity.command.command_id,
            identity.command.command_digest,
            identity.command.outbox_id,
            identity.command.outbox_digest,
            identity.route.route_authorization_id,
            identity.route.route_authorization_digest,
        ],
        |row| row.get::<_, bool>(0),
    )?;
    ensure!(
        is_uncertain,
        "V278 first reconcile source is not an exact unknown durable send"
    );
    let receipt_ids = connection
        .prepare(
            "SELECT exchange_receipt_id
               FROM compute_external_pool_adapter_task_exchange_receipts
              WHERE exchange_attempt_id=?1 AND exchange_attempt_digest=?2
              ORDER BY exchange_receipt_id LIMIT 2",
        )?
        .query_map(
            params![attempt.exchange_attempt_id, attempt.exchange_attempt_digest],
            |row| row.get::<_, String>(0),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ensure!(
        receipt_ids.is_empty(),
        "V278 first reconcile source already has a receipt"
    );
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
    if !reconcile_poll_needs_insert_on(connection, &envelope)? {
        insert_external_pool_adapter_task_reconcile_poll_on(connection, None, &envelope)?;
        return Ok(envelope);
    }
    let values = reconcile_poll_values(&envelope, &initial_claim())?;
    let pending = ExternalPoolAdapterTaskReachabilityPendingPlan::new(vec![
        ExternalPoolAdapterTaskReachabilityPendingWrite::new(
            ExternalPoolAdapterTaskReachabilityPendingWriteKind::ReconcilePoll,
            values,
        )?,
    ])?;
    let pending =
        install_external_pool_adapter_task_reachability_pending_plan_on(connection, pending)?;
    insert_external_pool_adapter_task_reconcile_poll_on(connection, Some(&pending), &envelope)?;
    pending.ensure_fully_consumed()?;
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
