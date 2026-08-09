use anyhow::{anyhow, bail, ensure, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::{
    attempt_gateway::{VerifiedComputeAttemptAdapterAck, COMPUTE_ATTEMPT_ADAPTER_ACK_ACCEPTED},
    start_outbox::{
        canonical_start_outbox_send_attempt_json_and_digest,
        ComputeStartOutboxRemoteObservationEnvelope, ComputeStartOutboxSendAttemptEnvelope,
        COMPUTE_OBSERVATION_CANCEL_RESPONSE, COMPUTE_OBSERVATION_PREPARE_RESPONSE,
        COMPUTE_OUTBOX_STATE_BLOCKED, COMPUTE_OUTBOX_STATE_DELIVERY_OBSERVED,
        COMPUTE_OUTBOX_STATE_IN_FLIGHT_UNKNOWN, COMPUTE_OUTBOX_STATE_PENDING,
        COMPUTE_REMOTE_EXECUTION_PREPARED, COMPUTE_REMOTE_TERMINALITY_NON_TERMINAL,
        COMPUTE_START_OPERATION_CANCEL, COMPUTE_START_OPERATION_RECONCILE,
    },
};

use super::{
    read::{operation_by_command_kind_on, prepare_by_command_on},
    types::{StartOutboxCleanupReceipt, StoredStartOutboxOperation},
};

mod currentness;
mod persist;

pub(super) use currentness::ensure_cleanup_send_source_exact_on;
use persist::{cleanup_envelope, cleanup_receipt, persist_cleanup_on};

#[derive(Clone, Copy)]
enum AckExpectation<'a> {
    Absent,
    ExactOrAbsent {
        ack_id: &'a str,
        ack_digest: &'a str,
    },
    PairOnly,
}

struct CleanupSource {
    prepare: StoredStartOutboxOperation,
    cleanup_expires_at: String,
}

pub(in crate::store) fn enqueue_quarantined_cleanup_on(
    connection: &Connection,
    verified: &VerifiedComputeAttemptAdapterAck,
    issued_at: &str,
) -> Result<StartOutboxCleanupReceipt> {
    let ack = verified.ack();
    let observation = verified.prepare_observation().envelope();
    ensure!(
        ack.outcome == COMPUTE_ATTEMPT_ADAPTER_ACK_ACCEPTED
            && observation.observation_kind == COMPUTE_OBSERVATION_PREPARE_RESPONSE
            && observation.response_outcome == COMPUTE_ATTEMPT_ADAPTER_ACK_ACCEPTED
            && observation.remote_execution_state == COMPUTE_REMOTE_EXECUTION_PREPARED
            && observation.terminality == COMPUTE_REMOTE_TERMINALITY_NON_TERMINAL
            && observation.command_id == ack.command_id
            && observation.adapter_observation_id == ack.adapter_ack_id,
        "quarantined cleanup requires the exact authenticated accepted response"
    );
    ensure_durable_observation_on(connection, observation)?;
    let source = cleanup_source_on(connection, &ack.command_id)?;
    ensure_cleanup_pair_on(
        connection,
        &source,
        AckExpectation::ExactOrAbsent {
            ack_id: &ack.ack_id,
            ack_digest: &ack.ack_digest,
        },
        issued_at,
    )
}

pub(super) fn ensure_unknown_prepare_cleanup_on(
    connection: &Connection,
    command_id: &str,
) -> Result<Option<StartOutboxCleanupReceipt>> {
    let prepare = prepare_by_command_on(connection, command_id)?
        .ok_or_else(|| anyhow!("unknown-delivery recovery lacks an exact prepare operation"))?;
    if prepare.projection.state != COMPUTE_OUTBOX_STATE_IN_FLIGHT_UNKNOWN {
        return Ok(None);
    }
    ensure_exact_prepare_send_on(connection, &prepare)?;
    let source = cleanup_source_for_prepare_on(connection, prepare)?;
    if let Some((cancel, reconcile)) = cleanup_pair_on(connection, &source, AckExpectation::Absent)?
    {
        ensure!(
            source
                .prepare
                .projection
                .claim_expires_at
                .as_deref()
                .is_some_and(|expiry| expiry <= cancel.envelope.issued_at.as_str()),
            "ACK-null cleanup pair predates the expired prepare claim"
        );
        return Ok(Some(cleanup_receipt(&cancel, &reconcile, true)));
    }
    let issued_at = now_nanos();
    let claim_expires_at = source
        .prepare
        .projection
        .claim_expires_at
        .as_deref()
        .ok_or_else(|| anyhow!("unknown-delivery prepare lost its claim expiry"))?;
    if claim_expires_at > issued_at.as_str() {
        return Ok(None);
    }
    ensure_unknown_source_absences_on(connection, &source.prepare)?;
    ensure_cleanup_pair_on(connection, &source, AckExpectation::Absent, &issued_at).map(Some)
}

pub(super) fn unlock_reconcile_after_cancel_on(
    connection: &Connection,
    observation: &ComputeStartOutboxRemoteObservationEnvelope,
) -> Result<()> {
    if observation.observation_kind != COMPUTE_OBSERVATION_CANCEL_RESPONSE {
        return Ok(());
    }
    let source = cleanup_source_on(connection, &observation.command_id)?;
    let (cancel, reconcile) = cleanup_pair_on(connection, &source, AckExpectation::PairOnly)?
        .ok_or_else(|| anyhow!("cancel observation lacks its durable cleanup pair"))?;
    ensure!(
        cancel.envelope.outbox_id == observation.outbox_id
            && cancel.envelope.outbox_digest == observation.outbox_digest
            && cancel.envelope.operation_kind == observation.operation_kind
            && cancel.projection.state == COMPUTE_OUTBOX_STATE_DELIVERY_OBSERVED,
        "cancel observation does not bind the exact delivered cancel intent"
    );
    if reconcile.projection.state != COMPUTE_OUTBOX_STATE_BLOCKED {
        return Ok(());
    }
    let transitioned_at = next_store_time_after(&reconcile.projection.updated_at)?;
    let changed = connection.execute(
        "UPDATE compute_attempt_start_outbox
            SET state='pending', state_revision=state_revision+1,
                next_attempt_at=?1, last_failure_code=NULL, updated_at=?1
          WHERE outbox_id=?2 AND outbox_digest=?3 AND state='blocked'
            AND state_revision=?4 AND attempt_count=?5 AND claim_generation=?6",
        params![
            transitioned_at,
            reconcile.envelope.outbox_id,
            reconcile.envelope.outbox_digest,
            reconcile.projection.state_revision,
            reconcile.projection.attempt_count,
            reconcile.projection.claim_generation,
        ],
    )?;
    ensure!(changed == 1, "reconcile unlock lost its exact blocked CAS");
    let stored = operation_by_command_kind_on(
        connection,
        &observation.command_id,
        COMPUTE_START_OPERATION_RECONCILE,
    )?
    .ok_or_else(|| anyhow!("reconcile disappeared after unlock"))?;
    ensure!(
        stored.projection.state == COMPUTE_OUTBOX_STATE_PENDING
            && stored.projection.state_revision == reconcile.projection.state_revision + 1
            && stored.projection.attempt_count == reconcile.projection.attempt_count
            && stored.projection.claim_generation == reconcile.projection.claim_generation
            && stored.projection.next_attempt_at == transitioned_at
            && stored.projection.updated_at == transitioned_at
            && stored.projection.claim_owner_id.is_none()
            && stored.projection.claim_token_digest.is_none()
            && stored.projection.claim_expires_at.is_none(),
        "reconcile unlock failed durable readback audit"
    );
    Ok(())
}

fn cleanup_source_on(connection: &Connection, command_id: &str) -> Result<CleanupSource> {
    let prepare = prepare_by_command_on(connection, command_id)?
        .ok_or_else(|| anyhow!("cleanup lacks an exact prepare operation"))?;
    cleanup_source_for_prepare_on(connection, prepare)
}

fn cleanup_source_for_prepare_on(
    connection: &Connection,
    prepare: StoredStartOutboxOperation,
) -> Result<CleanupSource> {
    let envelope = &prepare.envelope;
    let cleanup_expires_at = connection
        .query_row(
            "SELECT route.cleanup_expires_at
               FROM compute_attempt_dispatch_commands command
               JOIN compute_route_authorization_receipts route
                 ON route.route_authorization_id=?1 AND route.route_authorization_digest=?2
              WHERE command.command_id=?3 AND command.command_digest=?4
                AND command.provider_id=?5 AND command.adapter_id=?6
                AND command.adapter_binding_digest=?7
                AND command.execution_plan_id=?8 AND command.execution_plan_digest=?9
                AND command.lease_id=?10 AND command.fencing_generation=?11
                AND route.provider_id=?5 AND route.adapter_id=?6
                AND route.adapter_binding_digest=?7
                AND EXISTS (SELECT 1 FROM compute_route_authorization_capabilities cap
                     WHERE cap.route_authorization_id=route.route_authorization_id
                       AND cap.capability_id='cancel_no_start')
                AND EXISTS (SELECT 1 FROM compute_route_authorization_capabilities cap
                     WHERE cap.route_authorization_id=route.route_authorization_id
                       AND cap.capability_id='reconcile')
                AND NOT EXISTS (SELECT 1 FROM compute_attempt_activations activation
                     WHERE activation.lease_id=?10 OR activation.reservation_id=command.reservation_id)
                AND NOT EXISTS (SELECT 1 FROM compute_attempt_dispatch_applications application
                     WHERE application.command_id=?3 OR application.lease_id=?10)
                AND NOT EXISTS (SELECT 1 FROM compute_attempt_start_outbox commit_intent
                     JOIN compute_attempt_start_send_attempts commit_send
                       ON commit_send.outbox_id=commit_intent.outbox_id
                     WHERE commit_intent.command_id=?3 AND commit_intent.operation_kind='commit')",
            params![
                envelope.route_authorization_id,
                envelope.route_authorization_digest,
                envelope.command_id,
                envelope.command_digest,
                prepare.provider_id,
                prepare.adapter_id,
                envelope.adapter_binding_digest,
                envelope.plan_id,
                envelope.plan_digest,
                envelope.lease_id,
                envelope.fencing_generation,
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("cleanup source failed exact command and route audit"))?;
    Ok(CleanupSource {
        prepare,
        cleanup_expires_at,
    })
}

fn ensure_unknown_source_absences_on(
    connection: &Connection,
    prepare: &StoredStartOutboxOperation,
) -> Result<()> {
    let envelope = &prepare.envelope;
    let exact = connection.query_row(
        "SELECT NOT EXISTS (SELECT 1 FROM compute_attempt_dispatch_acks ack
                    WHERE ack.command_id=?1)
             AND NOT EXISTS (SELECT 1 FROM compute_attempt_start_remote_observations observation
                    WHERE observation.outbox_id=?2 AND observation.operation_kind='prepare'
                      AND observation.command_id=?1)
             AND NOT EXISTS (SELECT 1 FROM compute_attempt_activations activation
                    JOIN compute_attempt_dispatch_commands command ON command.command_id=?1
                    WHERE activation.lease_id=?3 OR activation.reservation_id=command.reservation_id)
             AND NOT EXISTS (SELECT 1 FROM compute_attempt_dispatch_applications application
                    WHERE application.command_id=?1 OR application.lease_id=?3)
             AND NOT EXISTS (SELECT 1 FROM compute_attempt_start_outbox commit_intent
                    JOIN compute_attempt_start_send_attempts send
                      ON send.outbox_id=commit_intent.outbox_id
                    WHERE commit_intent.command_id=?1 AND commit_intent.operation_kind='commit')",
        params![envelope.command_id, envelope.outbox_id, envelope.lease_id],
        |row| row.get::<_, bool>(0),
    )?;
    ensure!(exact, "unknown-delivery cleanup source is contradicted");
    Ok(())
}

fn ensure_exact_prepare_send_on(
    connection: &Connection,
    prepare: &StoredStartOutboxOperation,
) -> Result<()> {
    let (json, stored_digest) = connection
        .query_row(
            "SELECT send_attempt_json, send_attempt_digest
               FROM compute_attempt_start_send_attempts
              WHERE outbox_id=?1 AND outbox_digest=?2 AND operation_kind='prepare'
                AND attempt_no=?3 AND claim_generation=?4 AND claim_token_digest=?5",
            params![
                prepare.envelope.outbox_id,
                prepare.envelope.outbox_digest,
                prepare.projection.attempt_count,
                prepare.projection.claim_generation,
                prepare.projection.claim_token_digest,
            ],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("unknown-delivery cleanup lacks the exact prepare send-attempt"))?;
    let send: ComputeStartOutboxSendAttemptEnvelope = serde_json::from_str(&json)?;
    let (canonical, digest) = canonical_start_outbox_send_attempt_json_and_digest(&send)?;
    ensure!(
        canonical == json
            && digest == stored_digest
            && digest == send.send_attempt_digest
            && send.outbox_id == prepare.envelope.outbox_id
            && send.command_id == prepare.envelope.command_id
            && send.command_digest == prepare.envelope.command_digest
            && send.route_authorization_id == prepare.envelope.route_authorization_id
            && send.route_authorization_digest == prepare.envelope.route_authorization_digest,
        "unknown-delivery prepare send-attempt failed canonical exact audit"
    );
    Ok(())
}

fn ensure_cleanup_pair_on(
    connection: &Connection,
    source: &CleanupSource,
    expected_ack: AckExpectation<'_>,
    issued_at: &str,
) -> Result<StartOutboxCleanupReceipt> {
    if let Some((cancel, reconcile)) = cleanup_pair_on(connection, source, expected_ack)? {
        return Ok(cleanup_receipt(&cancel, &reconcile, true));
    }
    ensure!(
        issued_at < source.cleanup_expires_at.as_str(),
        "new cleanup pair is outside the sealed route horizon"
    );
    let ack = match expected_ack {
        AckExpectation::Absent => None,
        AckExpectation::ExactOrAbsent { ack_id, ack_digest } => Some((ack_id, ack_digest)),
        AckExpectation::PairOnly => bail!("cleanup pair is missing"),
    };
    let cancel = cleanup_envelope(
        source,
        COMPUTE_START_OPERATION_CANCEL,
        &source.prepare.envelope.outbox_id,
        ack,
        issued_at,
    )?;
    persist_cleanup_on(
        connection,
        source,
        &cancel,
        COMPUTE_OUTBOX_STATE_PENDING,
        issued_at,
    )?;
    let reconcile = cleanup_envelope(
        source,
        COMPUTE_START_OPERATION_RECONCILE,
        &cancel.outbox_id,
        ack,
        issued_at,
    )?;
    persist_cleanup_on(
        connection,
        source,
        &reconcile,
        COMPUTE_OUTBOX_STATE_BLOCKED,
        issued_at,
    )?;
    let (cancel, reconcile) = cleanup_pair_on(connection, source, expected_ack)?
        .ok_or_else(|| anyhow!("cleanup pair is not visible after insert"))?;
    Ok(cleanup_receipt(&cancel, &reconcile, false))
}

fn cleanup_pair_on(
    connection: &Connection,
    source: &CleanupSource,
    expected_ack: AckExpectation<'_>,
) -> Result<Option<(StoredStartOutboxOperation, StoredStartOutboxOperation)>> {
    let cancel = operation_by_command_kind_on(
        connection,
        &source.prepare.envelope.command_id,
        COMPUTE_START_OPERATION_CANCEL,
    )?;
    let reconcile = operation_by_command_kind_on(
        connection,
        &source.prepare.envelope.command_id,
        COMPUTE_START_OPERATION_RECONCILE,
    )?;
    let (cancel, reconcile) = match (cancel, reconcile) {
        (Some(cancel), Some(reconcile)) => (cancel, reconcile),
        (None, None) => return Ok(None),
        _ => bail!("cleanup pair is incomplete"),
    };
    ensure_cleanup_operation(
        &cancel,
        source,
        COMPUTE_START_OPERATION_CANCEL,
        &source.prepare.envelope.outbox_id,
    )?;
    ensure_cleanup_operation(
        &reconcile,
        source,
        COMPUTE_START_OPERATION_RECONCILE,
        &cancel.envelope.outbox_id,
    )?;
    let pair_ack = (
        cancel.envelope.ack_id.as_deref(),
        cancel.envelope.ack_digest.as_deref(),
    );
    ensure!(
        pair_ack
            == (
                reconcile.envelope.ack_id.as_deref(),
                reconcile.envelope.ack_digest.as_deref(),
            )
            && matches!(pair_ack, (None, None) | (Some(_), Some(_))),
        "cleanup pair has inconsistent ACK custody"
    );
    ensure!(
        cancel.envelope.issued_at == reconcile.envelope.issued_at
            && cancel.envelope.not_before == reconcile.envelope.not_before
            && cancel.envelope.not_after == reconcile.envelope.not_after,
        "cleanup pair has inconsistent delivery windows"
    );
    match expected_ack {
        AckExpectation::Absent => ensure!(pair_ack == (None, None), "cleanup pair is ACK-bound"),
        AckExpectation::ExactOrAbsent { ack_id, ack_digest } => ensure!(
            pair_ack == (None, None) || pair_ack == (Some(ack_id), Some(ack_digest)),
            "cleanup pair binds a different ACK"
        ),
        AckExpectation::PairOnly => {}
    }
    Ok(Some((cancel, reconcile)))
}

fn ensure_cleanup_operation(
    operation: &StoredStartOutboxOperation,
    source: &CleanupSource,
    operation_kind: &str,
    subject_outbox_id: &str,
) -> Result<()> {
    let envelope = &operation.envelope;
    let prepare = &source.prepare;
    ensure!(
        envelope.operation_kind == operation_kind
            && envelope.operation_generation == 1
            && envelope.subject_outbox_id.as_deref() == Some(subject_outbox_id)
            && envelope.command_id == prepare.envelope.command_id
            && envelope.command_digest == prepare.envelope.command_digest
            && operation.provider_id == prepare.provider_id
            && operation.adapter_id == prepare.adapter_id
            && envelope.adapter_binding_digest == prepare.envelope.adapter_binding_digest
            && envelope.route_authorization_id == prepare.envelope.route_authorization_id
            && envelope.route_authorization_digest == prepare.envelope.route_authorization_digest
            && envelope.actor_receipt_id == prepare.envelope.actor_receipt_id
            && envelope.actor_receipt_digest == prepare.envelope.actor_receipt_digest
            && envelope.plan_id == prepare.envelope.plan_id
            && envelope.plan_digest == prepare.envelope.plan_digest
            && envelope.lease_id == prepare.envelope.lease_id
            && envelope.fencing_generation == prepare.envelope.fencing_generation
            && envelope.application_id.is_none()
            && envelope.application_digest.is_none()
            && envelope.lease_authority_id.is_none()
            && envelope.lease_authority_revision.is_none()
            && envelope.lease_authority_digest.is_none()
            && envelope.issued_at == envelope.not_before
            && envelope.issued_at < envelope.not_after
            && envelope.not_after.as_str() <= source.cleanup_expires_at.as_str(),
        "cleanup operation failed exact source audit"
    );
    Ok(())
}

fn ensure_durable_observation_on(
    connection: &Connection,
    observation: &ComputeStartOutboxRemoteObservationEnvelope,
) -> Result<()> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM compute_attempt_start_remote_observations
              WHERE observation_id=?1 AND observation_digest=?2 AND outbox_id=?3
                AND command_id=?4 AND adapter_observation_id=?5
                AND observation_kind='prepare_response' AND response_outcome='accepted'
                AND remote_execution_state='prepared' AND terminality='non_terminal'",
            params![
                observation.observation_id,
                observation.observation_digest,
                observation.outbox_id,
                observation.command_id,
                observation.adapter_observation_id,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    ensure!(
        exists,
        "quarantined cleanup lacks durable accepted observation"
    );
    Ok(())
}

fn now_nanos() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}

fn next_store_time_after(value: &str) -> Result<String> {
    let floor = DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc) + Duration::nanoseconds(1);
    Ok(std::cmp::max(Utc::now(), floor).to_rfc3339_opts(SecondsFormat::Nanos, true))
}
