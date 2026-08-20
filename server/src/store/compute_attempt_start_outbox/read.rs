use anyhow::{anyhow, bail, ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::start_outbox::{
    canonical_start_outbox_operation_json_and_digest, ComputeStartOutboxClaimProjection,
    ComputeStartOutboxOperationEnvelope,
};

use super::types::{NoStartProofSource, StoredStartOutboxOperation};

pub(super) fn outbox_by_id_on(
    connection: &Connection,
    outbox_id: &str,
) -> Result<Option<StoredStartOutboxOperation>> {
    stored_outbox_on(connection, "o.outbox_id=?1", params![outbox_id])
}

pub(super) fn prepare_by_command_on(
    connection: &Connection,
    command_id: &str,
) -> Result<Option<StoredStartOutboxOperation>> {
    stored_outbox_on(
        connection,
        "o.command_id=?1 AND o.operation_kind='prepare'",
        params![command_id],
    )
}

pub(super) fn operation_by_command_kind_on(
    connection: &Connection,
    command_id: &str,
    operation_kind: &str,
) -> Result<Option<StoredStartOutboxOperation>> {
    stored_outbox_on(
        connection,
        "o.command_id=?1 AND o.operation_kind=?2",
        params![command_id, operation_kind],
    )
}

fn stored_outbox_on<P: rusqlite::Params>(
    connection: &Connection,
    predicate: &str,
    parameters: P,
) -> Result<Option<StoredStartOutboxOperation>> {
    let sql = format!(
        "SELECT o.outbox_json, o.outbox_digest, o.provider_id, o.adapter_id,
                o.state, o.state_revision, o.attempt_count, o.next_attempt_at,
                o.claim_owner_id, o.claim_token_digest, o.claim_generation,
                o.claim_expires_at, o.last_failure_code, o.created_at, o.updated_at,
                route.provider_id, route.adapter_id, route.adapter_binding_digest
           FROM compute_attempt_start_outbox o
           JOIN compute_route_authorization_receipts route
             ON route.route_authorization_id=o.route_authorization_id
            AND route.route_authorization_digest=o.route_authorization_digest
          WHERE {predicate}"
    );
    let stored = connection
        .query_row(&sql, parameters, |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                ComputeStartOutboxClaimProjection {
                    state: row.get(4)?,
                    state_revision: row.get(5)?,
                    attempt_count: row.get(6)?,
                    next_attempt_at: row.get(7)?,
                    claim_owner_id: row.get(8)?,
                    claim_token_digest: row.get(9)?,
                    claim_generation: row.get(10)?,
                    claim_expires_at: row.get(11)?,
                    last_failure_code: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                },
                row.get::<_, String>(15)?,
                row.get::<_, String>(16)?,
                row.get::<_, String>(17)?,
            ))
        })
        .optional()?;
    let Some((
        json,
        digest,
        provider_id,
        adapter_id,
        projection,
        route_provider,
        route_adapter,
        route_binding,
    )) = stored
    else {
        return Ok(None);
    };
    let envelope: ComputeStartOutboxOperationEnvelope = serde_json::from_str(&json)?;
    let (canonical, recomputed) = canonical_start_outbox_operation_json_and_digest(&envelope)?;
    if canonical != json
        || envelope.outbox_digest != digest
        || recomputed != digest
        || provider_id != route_provider
        || adapter_id != route_adapter
        || envelope.adapter_binding_digest != route_binding
    {
        bail!("stored Start outbox operation failed exact canonical audit");
    }
    ensure_claim_projection_shape(&projection)?;
    Ok(Some(StoredStartOutboxOperation {
        envelope,
        provider_id,
        adapter_id,
        projection,
    }))
}

fn ensure_claim_projection_shape(projection: &ComputeStartOutboxClaimProjection) -> Result<()> {
    let carries_claim = matches!(projection.state.as_str(), "claimed" | "in_flight_unknown");
    let complete_claim = projection
        .claim_owner_id
        .as_deref()
        .is_some_and(|v| !v.is_empty())
        && projection
            .claim_token_digest
            .as_deref()
            .is_some_and(|v| v.len() == 64)
        && projection.claim_generation > 0
        && projection.claim_expires_at.is_some();
    if projection.state_revision < 1
        || projection.attempt_count < 0
        || projection.claim_generation < 0
        || carries_claim != complete_claim
    {
        return Err(anyhow!("stored Start outbox mutable projection is invalid"));
    }
    Ok(())
}

pub(super) fn no_start_source_on(
    connection: &Connection,
    command_id: &str,
) -> Result<Option<NoStartProofSource>> {
    connection
        .query_row(
            "SELECT prepare.outbox_id, prepare.outbox_digest, command.command_id,
                    command.command_digest, command.execution_plan_id,
                    command.execution_plan_digest, command.provider_id,
                    command.reservation_id, command.reservation_revision,
                    command.reservation_digest, command.job_id, command.job_revision,
                    command.job_digest, command.capacity_claim_id, command.claim_revision,
                    command.claim_digest, command.budget_reservation_id,
                    command.budget_reserved_fen, command.broker_request_digest,
                    command.lease_id, command.fencing_generation, command.adapter_id,
                    route.adapter_revision, route.adapter_registry_digest,
                    command.adapter_binding_digest, route.route_authorization_id,
                    route.route_authorization_digest, prepare.state,
                    prepare.state_revision, prepare.attempt_count, prepare.claim_generation,
                    prepare.claim_expires_at, prepare.not_after
               FROM compute_attempt_dispatch_commands command
               JOIN compute_attempt_start_outbox prepare
                 ON prepare.command_id=command.command_id AND prepare.operation_kind='prepare'
               JOIN compute_route_authorization_receipts route
                 ON route.route_authorization_id=prepare.route_authorization_id
                AND route.route_authorization_digest=prepare.route_authorization_digest
              WHERE command.command_id=?1",
            params![command_id],
            |row| {
                Ok(NoStartProofSource {
                    outbox_id: row.get(0)?,
                    outbox_digest: row.get(1)?,
                    command_id: row.get(2)?,
                    command_digest: row.get(3)?,
                    plan_id: row.get(4)?,
                    plan_digest: row.get(5)?,
                    provider_id: row.get(6)?,
                    reservation_id: row.get(7)?,
                    reservation_revision: row.get(8)?,
                    reservation_digest: row.get(9)?,
                    job_id: row.get(10)?,
                    job_revision: row.get(11)?,
                    job_digest: row.get(12)?,
                    capacity_claim_id: row.get(13)?,
                    capacity_claim_revision: row.get(14)?,
                    capacity_claim_digest: row.get(15)?,
                    budget_reservation_id: row.get(16)?,
                    budget_reserved_fen: row.get(17)?,
                    broker_request_digest: row.get(18)?,
                    lease_id: row.get(19)?,
                    fencing_generation: row.get(20)?,
                    adapter_id: row.get(21)?,
                    adapter_revision: row.get(22)?,
                    adapter_registry_digest: row.get(23)?,
                    adapter_binding_digest: row.get(24)?,
                    route_authorization_id: row.get(25)?,
                    route_authorization_digest: row.get(26)?,
                    prepare_state: row.get(27)?,
                    prepare_state_revision: row.get(28)?,
                    prepare_attempt_count: row.get(29)?,
                    prepare_claim_generation: row.get(30)?,
                    prepare_claim_expires_at: row.get(31)?,
                    prepare_not_after: row.get(32)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

pub(super) fn no_start_semantics_exact_on(
    connection: &Connection,
    proof: &crate::compute_federation::start_outbox::ComputeStartNoStartProofEnvelope,
) -> Result<bool> {
    connection
        .query_row(
            "SELECT 1
               FROM compute_attempt_dispatch_commands command
               JOIN compute_attempt_start_outbox prepare
                 ON prepare.outbox_id=?1 AND prepare.outbox_digest=?2
                AND prepare.command_id=command.command_id
              WHERE command.command_id=?3 AND command.command_digest=?4
                AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_activations activation
                     WHERE activation.lease_id=?5 OR activation.reservation_id=?6
                )
                AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_dispatch_applications application
                     WHERE application.command_id=?3 OR application.lease_id=?5
                )
                AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_start_outbox commit_intent
                    JOIN compute_attempt_start_send_attempts send
                      ON send.outbox_id=commit_intent.outbox_id
                    WHERE commit_intent.command_id=?3
                      AND commit_intent.operation_kind='commit'
                )
                AND NOT EXISTS (
                    SELECT 1 FROM compute_attempt_start_remote_observations contradiction
                     WHERE contradiction.command_id=?3
                       AND contradiction.remote_execution_state IN (
                            'committed','running','terminal_after_run'
                       )
                )
                AND (
                    (?7='local_never_sent' AND prepare.state='abandoned_no_send'
                        AND NOT EXISTS (
                            SELECT 1 FROM compute_attempt_start_send_attempts send
                             WHERE send.outbox_id=prepare.outbox_id
                        )
                        AND NOT EXISTS (
                            SELECT 1 FROM compute_attempt_start_remote_observations observation
                             WHERE observation.command_id=?3
                        )
                        AND NOT EXISTS (
                            SELECT 1 FROM compute_attempt_dispatch_acks ack
                             WHERE ack.command_id=?3
                        ))
                    OR (?7='prepare_rejected' AND EXISTS (
                        SELECT 1 FROM compute_attempt_start_remote_observations observation
                        JOIN compute_attempt_dispatch_acks ack
                          ON ack.command_id=observation.command_id
                        WHERE observation.observation_id=?8
                          AND observation.observation_digest=?9
                          AND observation.outbox_id=prepare.outbox_id
                          AND observation.outbox_digest=prepare.outbox_digest
                          AND observation.operation_kind='prepare'
                          AND observation.command_id=?3
                          AND observation.command_digest=?4
                          AND observation.provider_id=prepare.provider_id
                          AND observation.adapter_id=prepare.adapter_id
                          AND observation.adapter_binding_digest=prepare.adapter_binding_digest
                          AND observation.observation_kind='prepare_response'
                          AND observation.response_outcome='rejected'
                          AND observation.remote_execution_state='rejected'
                          AND observation.terminality='final'
                          AND ack.outcome='rejected' AND ack.disposition='rejected'
                          AND ack.command_id=?3 AND ack.command_digest=?4
                          AND ack.adapter_binding_digest=prepare.adapter_binding_digest
                          AND ack.adapter_ack_id=observation.adapter_observation_id
                    ))
                    OR (?7='remote_never_committed' AND (EXISTS (
                        SELECT 1 FROM compute_attempt_start_remote_observations observation
                        JOIN compute_attempt_start_outbox reconcile
                          ON reconcile.outbox_id=observation.outbox_id
                        JOIN compute_attempt_start_outbox cancel
                          ON cancel.outbox_id=reconcile.subject_outbox_id
                        WHERE observation.observation_id=?8
                          AND observation.observation_digest=?9
                          AND observation.outbox_id=reconcile.outbox_id
                          AND observation.outbox_digest=reconcile.outbox_digest
                          AND observation.operation_kind='reconcile'
                          AND observation.command_id=?3
                          AND observation.command_digest=?4
                          AND observation.provider_id=prepare.provider_id
                          AND observation.adapter_id=prepare.adapter_id
                          AND observation.adapter_binding_digest=prepare.adapter_binding_digest
                          AND observation.observation_kind='reconcile_attestation'
                          AND observation.response_outcome='observed'
                          AND observation.remote_execution_state='terminal_no_start'
                          AND observation.terminality='final'
                          AND cancel.state='delivery_observed'
                          AND reconcile.state='delivery_observed'
                          AND cancel.command_id=?3 AND cancel.command_digest=?4
                          AND reconcile.command_id=?3 AND reconcile.command_digest=?4
                          AND cancel.ack_id IS reconcile.ack_id
                          AND cancel.ack_digest IS reconcile.ack_digest
                          AND observation.no_commit_tombstone_id=?10
                           AND observation.no_commit_tombstone_digest=?11
                           AND cancel.subject_outbox_id=prepare.outbox_id
                    ) OR EXISTS (
                        SELECT 1 FROM compute_attempt_start_remote_observations observation
                        JOIN compute_external_pool_adapter_task_exchange_receipts receipt
                          ON receipt.exchange_receipt_id=observation.verifier_id
                         AND receipt.semantic_observation_sha256
                             =observation.verification_digest
                        JOIN compute_external_pool_adapter_task_reconcile_polls poll
                          ON poll.reconcile_poll_id=receipt.source_id
                         AND poll.reconcile_poll_digest=receipt.source_digest
                        JOIN compute_attempt_start_send_attempts send
                          ON send.send_attempt_id=poll.send_attempt_id
                         AND send.send_attempt_digest=poll.send_attempt_digest
                        JOIN compute_external_pool_adapter_task_exchange_receipts cancel_receipt
                          ON cancel_receipt.exchange_attempt_id
                             =poll.uncertain_exchange_attempt_id
                         AND cancel_receipt.exchange_attempt_digest
                             =poll.uncertain_exchange_attempt_digest
                         AND cancel_receipt.operation_kind='cancel_no_start'
                         AND cancel_receipt.source_kind='start_outbox_send_attempt'
                         AND cancel_receipt.source_id=send.send_attempt_id
                         AND cancel_receipt.source_digest=send.send_attempt_digest
                        JOIN compute_attempt_start_outbox source_outbox
                          ON source_outbox.outbox_id=send.outbox_id
                         AND source_outbox.outbox_digest=send.outbox_digest
                        WHERE observation.observation_id=?8
                          AND observation.observation_digest=?9
                          AND observation.command_id=?3
                          AND observation.command_digest=?4
                          AND observation.provider_id=prepare.provider_id
                          AND observation.adapter_id=prepare.adapter_id
                          AND observation.adapter_binding_digest=prepare.adapter_binding_digest
                          AND observation.observation_kind='reconcile_attestation'
                          AND observation.response_outcome='observed'
                          AND observation.remote_execution_state='terminal_no_start'
                          AND observation.terminality='final'
                          AND observation.verification_kind
                              ='external_pool_adapter_task_receipt.v1'
                          AND observation.no_commit_tombstone_id=?10
                          AND observation.no_commit_tombstone_digest=?11
                          AND receipt.operation_kind='reconcile'
                          AND receipt.source_kind='reconcile_poll'
                          AND receipt.command_id=observation.command_id
                          AND receipt.command_digest=observation.command_digest
                          AND receipt.outbox_id=observation.outbox_id
                          AND receipt.outbox_digest=observation.outbox_digest
                          AND receipt.send_attempt_id=observation.send_attempt_id
                          AND receipt.authenticated_at=observation.authenticated_at
                          AND receipt.received_at=observation.received_at
                          AND receipt.recorded_at=observation.recorded_at
                          AND poll.claim_status='delivery_observed'
                          AND poll.command_id=receipt.command_id
                          AND poll.command_digest=receipt.command_digest
                          AND poll.outbox_id=receipt.outbox_id
                          AND poll.outbox_digest=receipt.outbox_digest
                          AND poll.authenticated_subject_sha256
                              =cancel_receipt.semantic_observation_sha256
                          AND source_outbox.state='delivery_observed'
                          AND send.operation_kind='cancel'
                          AND observation.operation_kind=send.operation_kind
                          AND source_outbox.subject_outbox_id=prepare.outbox_id
                    )))
                )",
            params![
                proof.outbox_id,
                proof.outbox_digest,
                proof.command_id,
                proof.command_digest,
                proof.lease_id,
                proof.reservation_id,
                proof.proof_kind,
                proof.observation_id,
                proof.observation_digest,
                proof.no_commit_tombstone_id,
                proof.no_commit_tombstone_digest,
            ],
            |_| Ok(()),
        )
        .optional()
        .map(|value| value.is_some())
        .map_err(Into::into)
}

pub(super) fn route_capabilities_exact_on(
    connection: &Connection,
    route_authorization_id: &str,
    route_json: &str,
) -> Result<bool> {
    connection
        .query_row(
            "SELECT count(*)=6 AND NOT EXISTS (
                SELECT 1 FROM compute_route_authorization_capabilities cap
                 WHERE cap.route_authorization_id=?1
                   AND NOT EXISTS (
                        SELECT 1 FROM json_each(?2, '$.authorization.capabilities') item
                         WHERE json_extract(item.value,'$.ordinal')=cap.ordinal
                           AND json_extract(item.value,'$.capability_id')=cap.capability_id
                           AND json_extract(item.value,'$.capability_revision')
                                =cap.capability_revision
                   )
             ) FROM compute_route_authorization_capabilities
             WHERE route_authorization_id=?1",
            params![route_authorization_id, route_json],
            |row| row.get::<_, bool>(0),
        )
        .map_err(Into::into)
}
