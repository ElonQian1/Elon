use anyhow::{ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::super::types::StoredStartOutboxOperation;

pub(in crate::store::compute_attempt_start_outbox) fn ensure_cleanup_send_source_exact_on(
    connection: &Connection,
    stored: &StoredStartOutboxOperation,
) -> Result<()> {
    if ack_bound_source_exact_on(connection, stored)? {
        audit_pair_member_on(connection, stored)?;
        return Ok(());
    }
    ensure!(
        stored.envelope.ack_id.is_none() && stored.envelope.ack_digest.is_none(),
        "cleanup operation has incomplete ACK custody"
    );
    let envelope = &stored.envelope;
    let exact = connection.query_row(
        "SELECT EXISTS(
            SELECT 1
              FROM compute_attempt_dispatch_commands command
              JOIN compute_attempt_start_outbox prepare
                ON prepare.command_id=command.command_id
               AND prepare.operation_kind='prepare'
              JOIN compute_attempt_start_send_attempts prepare_send
                ON prepare_send.outbox_id=prepare.outbox_id
               AND prepare_send.outbox_digest=prepare.outbox_digest
               AND prepare_send.operation_kind='prepare'
               AND prepare_send.command_id=prepare.command_id
               AND prepare_send.command_digest=prepare.command_digest
               AND prepare_send.route_authorization_id=prepare.route_authorization_id
               AND prepare_send.route_authorization_digest=prepare.route_authorization_digest
               AND prepare_send.attempt_no=prepare.attempt_count
               AND prepare_send.claim_generation=prepare.claim_generation
             WHERE command.command_id=?1 AND command.command_digest=?2
               AND command.provider_id=?3 AND command.adapter_id=?4
               AND command.adapter_binding_digest=?5
               AND command.execution_plan_id=?6 AND command.execution_plan_digest=?7
               AND command.lease_id=?8 AND command.fencing_generation=?9
               AND prepare.provider_id=?3 AND prepare.adapter_id=?4
               AND prepare.adapter_binding_digest=?5
               AND prepare.route_authorization_id=?10
               AND prepare.route_authorization_digest=?11
               AND prepare.actor_receipt_id=?12 AND prepare.actor_receipt_digest=?13
               AND prepare.plan_id=?6 AND prepare.plan_digest=?7
               AND prepare.lease_id=?8 AND prepare.fencing_generation=?9
               AND NOT EXISTS (SELECT 1 FROM compute_attempt_activations activation
                    WHERE activation.lease_id=?8 OR activation.reservation_id=command.reservation_id)
               AND NOT EXISTS (SELECT 1 FROM compute_attempt_dispatch_applications application
                    WHERE application.command_id=?1 OR application.lease_id=?8)
               AND NOT EXISTS (SELECT 1 FROM compute_attempt_start_outbox commit_intent
                    JOIN compute_attempt_start_send_attempts commit_send
                      ON commit_send.outbox_id=commit_intent.outbox_id
                    WHERE commit_intent.command_id=?1 AND commit_intent.operation_kind='commit')
               AND ((?14='cancel' AND ?15=prepare.outbox_id)
                 OR (?14='reconcile' AND EXISTS (
                    SELECT 1
                      FROM compute_attempt_start_outbox cancel
                      JOIN compute_attempt_start_remote_observations observation
                        ON observation.outbox_id=cancel.outbox_id
                       AND observation.outbox_digest=cancel.outbox_digest
                       AND observation.operation_kind='cancel'
                       AND observation.observation_kind='cancel_response'
                       AND observation.command_id=cancel.command_id
                       AND observation.command_digest=cancel.command_digest
                       AND observation.provider_id=cancel.provider_id
                       AND observation.adapter_id=cancel.adapter_id
                       AND observation.adapter_binding_digest=cancel.adapter_binding_digest
                     WHERE cancel.outbox_id=?15 AND cancel.operation_kind='cancel'
                       AND cancel.subject_outbox_id=prepare.outbox_id
                       AND cancel.ack_id IS NULL AND cancel.ack_digest IS NULL
                       AND cancel.command_id=?1 AND cancel.command_digest=?2
                       AND cancel.provider_id=?3 AND cancel.adapter_id=?4
                       AND cancel.adapter_binding_digest=?5
                       AND cancel.route_authorization_id=?10
                       AND cancel.route_authorization_digest=?11
                       AND cancel.actor_receipt_id=?12 AND cancel.actor_receipt_digest=?13
                       AND cancel.plan_id=?6 AND cancel.plan_digest=?7
                       AND cancel.lease_id=?8 AND cancel.fencing_generation=?9
                       AND cancel.issued_at=?16 AND cancel.not_before=?17
                       AND cancel.not_after=?18 AND cancel.state='delivery_observed'
               )))
               AND (
                    (prepare.state='in_flight_unknown'
                       AND prepare.claim_expires_at<=?16
                       AND prepare_send.claim_token_digest=prepare.claim_token_digest
                       AND NOT EXISTS (SELECT 1 FROM compute_attempt_dispatch_acks ack
                            WHERE ack.command_id=?1)
                       AND NOT EXISTS (
                            SELECT 1 FROM compute_attempt_start_remote_observations observation
                             WHERE observation.outbox_id=prepare.outbox_id))
                    OR (prepare.state='delivery_observed' AND EXISTS (
                        SELECT 1
                          FROM compute_attempt_dispatch_acks ack
                          JOIN compute_attempt_start_remote_observations observation
                            ON observation.command_id=ack.command_id
                           AND observation.adapter_observation_id=ack.adapter_ack_id
                         WHERE ack.command_id=?1 AND ack.command_digest=?2
                           AND ack.provider_id=?3 AND ack.adapter_id=?4
                           AND ack.adapter_binding_digest=?5
                           AND ack.outcome='accepted' AND ack.disposition='quarantined'
                           AND ?16<=ack.created_at
                           AND observation.send_attempt_id=prepare_send.send_attempt_id
                           AND observation.outbox_id=prepare.outbox_id
                           AND observation.outbox_digest=prepare.outbox_digest
                           AND observation.operation_kind='prepare'
                           AND observation.command_id=?1 AND observation.command_digest=?2
                           AND observation.provider_id=?3 AND observation.adapter_id=?4
                           AND observation.adapter_binding_digest=?5
                           AND observation.observation_kind='prepare_response'
                           AND observation.response_outcome='accepted'
                           AND observation.remote_execution_state='prepared'
                           AND observation.terminality='non_terminal'
                    ))
               )
        )",
        params![
            envelope.command_id,
            envelope.command_digest,
            stored.provider_id,
            stored.adapter_id,
            envelope.adapter_binding_digest,
            envelope.plan_id,
            envelope.plan_digest,
            envelope.lease_id,
            envelope.fencing_generation,
            envelope.route_authorization_id,
            envelope.route_authorization_digest,
            envelope.actor_receipt_id,
            envelope.actor_receipt_digest,
            envelope.operation_kind,
            envelope.subject_outbox_id,
            envelope.issued_at,
            envelope.not_before,
            envelope.not_after,
        ],
        |row| row.get::<_, bool>(0),
    )?;
    ensure!(
        exact,
        "ACK-null cleanup operation lacks its exact recovery source"
    );
    audit_pair_member_on(connection, stored)?;
    Ok(())
}

fn audit_pair_member_on(
    connection: &Connection,
    stored: &StoredStartOutboxOperation,
) -> Result<()> {
    let source = super::cleanup_source_on(connection, &stored.envelope.command_id)?;
    let (cancel, reconcile) =
        super::cleanup_pair_on(connection, &source, super::AckExpectation::PairOnly)?
            .ok_or_else(|| anyhow::anyhow!("ACK-bound cleanup pair is missing"))?;
    let expected = if stored.envelope.operation_kind == "cancel" {
        &cancel
    } else {
        ensure!(
            stored.envelope.operation_kind == "reconcile",
            "unsupported cleanup operation kind"
        );
        ensure_cancel_observation_on(connection, &cancel)?;
        &reconcile
    };
    ensure!(
        expected.envelope == stored.envelope
            && expected.provider_id == stored.provider_id
            && expected.adapter_id == stored.adapter_id
            && expected.projection == stored.projection,
        "ACK-bound cleanup currentness does not bind the exact pair member"
    );
    Ok(())
}

fn ensure_cancel_observation_on(
    connection: &Connection,
    cancel: &StoredStartOutboxOperation,
) -> Result<()> {
    ensure!(
        cancel.projection.state == "delivery_observed",
        "reconcile requires its cancel delivery observation"
    );
    let exact = connection
        .query_row(
            "SELECT 1
               FROM compute_attempt_start_remote_observations observation
               JOIN compute_attempt_start_send_attempts send
                 ON send.send_attempt_id=observation.send_attempt_id
                AND send.outbox_id=observation.outbox_id
                AND send.outbox_digest=observation.outbox_digest
              WHERE observation.outbox_id=?1 AND observation.outbox_digest=?2
                AND observation.operation_kind='cancel'
                AND observation.observation_kind='cancel_response'
                AND observation.command_id=?3 AND observation.command_digest=?4
                AND observation.provider_id=?5 AND observation.adapter_id=?6
                AND observation.adapter_binding_digest=?7",
            params![
                cancel.envelope.outbox_id,
                cancel.envelope.outbox_digest,
                cancel.envelope.command_id,
                cancel.envelope.command_digest,
                cancel.provider_id,
                cancel.adapter_id,
                cancel.envelope.adapter_binding_digest,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    ensure!(exact, "reconcile lacks the exact durable cancel response");
    Ok(())
}

fn ack_bound_source_exact_on(
    connection: &Connection,
    stored: &StoredStartOutboxOperation,
) -> Result<bool> {
    let Some(ack_id) = stored.envelope.ack_id.as_deref() else {
        return Ok(false);
    };
    let ack_digest = stored
        .envelope
        .ack_digest
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("cleanup ACK digest is missing"))?;
    connection
        .query_row(
            "SELECT 1 FROM compute_attempt_dispatch_commands command
               JOIN compute_attempt_dispatch_acks ack ON ack.command_id=command.command_id
              WHERE command.command_id=?1 AND command.command_digest=?2
                AND command.provider_id=?3 AND command.adapter_id=?4
                AND command.adapter_binding_digest=?5
                AND ack.ack_id=?6 AND ack.ack_digest=?7
                AND ack.provider_id=?3 AND ack.adapter_id=?4
                AND ack.adapter_binding_digest=?5
                AND ack.outcome='accepted' AND ack.disposition='quarantined'",
            params![
                stored.envelope.command_id,
                stored.envelope.command_digest,
                stored.provider_id,
                stored.adapter_id,
                stored.envelope.adapter_binding_digest,
                ack_id,
                ack_digest,
            ],
            |_| Ok(()),
        )
        .optional()
        .map(|value| value.is_some())
        .map_err(Into::into)
}
