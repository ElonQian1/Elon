use anyhow::{bail, ensure, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::start_outbox::{
    canonical_start_outbox_remote_observation_json_and_digest,
    canonical_start_outbox_send_attempt_json_and_digest,
    ComputeStartOutboxRemoteObservationEnvelope, ComputeStartOutboxSendAttemptEnvelope,
    VerifiedComputeStartOutboxRemoteObservation, COMPUTE_OBSERVATION_CANCEL_RESPONSE,
    COMPUTE_OBSERVATION_RECONCILE_ATTESTATION, COMPUTE_OUTBOX_STATE_DELIVERY_OBSERVED,
    COMPUTE_OUTBOX_STATE_IN_FLIGHT_UNKNOWN, COMPUTE_REMOTE_EXECUTION_TERMINAL_NO_START,
    COMPUTE_REMOTE_TERMINALITY_FINAL,
};

use super::{
    cleanup::unlock_reconcile_after_cancel_on,
    no_start::record_remote_never_committed_no_start_on,
    read::outbox_by_id_on,
    types::{StartOutboxObservationReceipt, StoredVerifiedObservation},
};

pub(in crate::store) fn record_verified_observation_on(
    connection: &Connection,
    verified: &VerifiedComputeStartOutboxRemoteObservation,
) -> Result<StartOutboxObservationReceipt> {
    let envelope = verified.envelope();
    let send = verified.send_attempt().envelope();
    ensure_observation_shape(connection, envelope, send)?;
    if let Some(stored) = observation_replay_on(connection, envelope)? {
        ensure!(
            stored.envelope == *envelope,
            "authenticated Start observation conflicts with immutable replay"
        );
        apply_observation_effects_on(connection, verified)?;
        return Ok(receipt(envelope, true));
    }
    let (json, digest) = canonical_start_outbox_remote_observation_json_and_digest(envelope)?;
    ensure!(
        digest == envelope.observation_digest,
        "authenticated Start observation digest mismatch"
    );
    connection.execute(
        "INSERT INTO compute_attempt_start_remote_observations (
            observation_id, observation_schema, observation_digest, observation_json,
            canonicalization, digest_algorithm, send_attempt_id, outbox_id,
            outbox_digest, operation_kind, observation_kind, command_id,
            command_digest, provider_id, adapter_id, adapter_binding_digest,
            adapter_observation_id, response_outcome, remote_execution_state,
            terminality, remote_execution_ref, remote_sequence,
            no_commit_tombstone_id, no_commit_tombstone_digest, reason_code,
            verification_kind, verifier_id, verification_digest,
            authenticated_at, observed_at, received_at, recorded_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
            ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21,
            ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32
         )",
        params![
            envelope.observation_id,
            envelope.schema,
            envelope.observation_digest,
            json,
            envelope.canonicalization,
            envelope.digest_algorithm,
            envelope.send_attempt_id,
            envelope.outbox_id,
            envelope.outbox_digest,
            envelope.operation_kind,
            envelope.observation_kind,
            envelope.command_id,
            envelope.command_digest,
            envelope.provider_id,
            envelope.adapter_id,
            envelope.adapter_binding_digest,
            envelope.adapter_observation_id,
            envelope.response_outcome,
            envelope.remote_execution_state,
            envelope.terminality,
            envelope.remote_execution_ref,
            envelope.remote_sequence,
            envelope.no_commit_tombstone_id,
            envelope.no_commit_tombstone_digest,
            envelope.reason_code,
            envelope.verification_kind,
            envelope.verifier_id,
            envelope.verification_digest,
            envelope.authenticated_at,
            envelope.observed_at,
            envelope.received_at,
            envelope.recorded_at,
        ],
    )?;
    let before = outbox_by_id_on(connection, &envelope.outbox_id)?
        .ok_or_else(|| anyhow::anyhow!("observed Start outbox operation is missing"))?;
    ensure!(
        before.projection.state == COMPUTE_OUTBOX_STATE_IN_FLIGHT_UNKNOWN,
        "fresh Start observation does not follow unknown delivery"
    );
    let transitioned_at = next_store_time_after(&before.projection.updated_at)?;
    ensure!(
        envelope.recorded_at.as_str() <= transitioned_at.as_str(),
        "authenticated Start observation is recorded after Store ingestion"
    );
    // A reconcile poll may report a durable but non-terminal state. Preserve that evidence
    // without consuming the one-shot reconcile operation so a later final attestation from the
    // same send-attempt can still close the recovery chain.
    if envelope.observation_kind == COMPUTE_OBSERVATION_RECONCILE_ATTESTATION
        && envelope.terminality != COMPUTE_REMOTE_TERMINALITY_FINAL
    {
        apply_observation_effects_on(connection, verified)?;
        return Ok(receipt(envelope, false));
    }
    let changed = connection.execute(
        "UPDATE compute_attempt_start_outbox
            SET state='delivery_observed', state_revision=state_revision+1,
                claim_owner_id=NULL, claim_token_digest=NULL, claim_expires_at=NULL,
                last_failure_code=NULL, updated_at=?1
          WHERE outbox_id=?2 AND outbox_digest=?3 AND state='in_flight_unknown'
            AND state_revision=?4 AND attempt_count=?5 AND claim_generation=?6",
        params![
            transitioned_at,
            envelope.outbox_id,
            envelope.outbox_digest,
            before.projection.state_revision,
            before.projection.attempt_count,
            before.projection.claim_generation,
        ],
    )?;
    ensure!(
        changed == 1,
        "Start observation lost its exact outbox state CAS"
    );
    let after = outbox_by_id_on(connection, &envelope.outbox_id)?
        .ok_or_else(|| anyhow::anyhow!("Start outbox disappeared after observation"))?;
    ensure!(
        after.projection.state == COMPUTE_OUTBOX_STATE_DELIVERY_OBSERVED
            && after.projection.state_revision == before.projection.state_revision + 1
            && after.projection.attempt_count == before.projection.attempt_count
            && after.projection.claim_generation == before.projection.claim_generation
            && after.projection.claim_owner_id.is_none()
            && after.projection.claim_token_digest.is_none()
            && after.projection.claim_expires_at.is_none(),
        "Start observation durable readback failed exact audit"
    );
    apply_observation_effects_on(connection, verified)?;
    Ok(receipt(envelope, false))
}

fn ensure_observation_shape(
    connection: &Connection,
    envelope: &ComputeStartOutboxRemoteObservationEnvelope,
    expected_send: &ComputeStartOutboxSendAttemptEnvelope,
) -> Result<()> {
    if envelope.remote_execution_state == COMPUTE_REMOTE_EXECUTION_TERMINAL_NO_START
        && (envelope.observation_kind != COMPUTE_OBSERVATION_RECONCILE_ATTESTATION
            || envelope.response_outcome != "observed"
            || envelope.terminality != COMPUTE_REMOTE_TERMINALITY_FINAL
            || envelope.no_commit_tombstone_id.is_none()
            || envelope.no_commit_tombstone_digest.is_none())
    {
        bail!("terminal no-start requires an observed final reconcile tombstone");
    }
    if envelope.observation_kind == COMPUTE_OBSERVATION_CANCEL_RESPONSE
        && (envelope.remote_execution_state == COMPUTE_REMOTE_EXECUTION_TERMINAL_NO_START
            || envelope.terminality != "non_terminal"
            || envelope.no_commit_tombstone_id.is_some()
            || envelope.no_commit_tombstone_digest.is_some())
    {
        bail!("cancel response can never establish no-start");
    }
    let (stored_send_json, stored_send_digest) = connection
        .query_row(
            "SELECT send_attempt_json, send_attempt_digest
               FROM compute_attempt_start_send_attempts
              WHERE send_attempt_id=?1",
            params![envelope.send_attempt_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("authenticated observation lacks durable send-start"))?;
    let stored_send: ComputeStartOutboxSendAttemptEnvelope =
        serde_json::from_str(&stored_send_json)?;
    let (canonical_send, send_digest) =
        canonical_start_outbox_send_attempt_json_and_digest(&stored_send)?;
    ensure!(
        canonical_send == stored_send_json
            && send_digest == stored_send.send_attempt_digest
            && send_digest == stored_send_digest
            && stored_send == *expected_send
            && envelope.send_attempt_id == stored_send.send_attempt_id
            && envelope.outbox_id == stored_send.outbox_id
            && envelope.outbox_digest == stored_send.outbox_digest
            && envelope.operation_kind == stored_send.operation_kind
            && envelope.command_id == stored_send.command_id
            && envelope.command_digest == stored_send.command_digest,
        "authenticated observation does not bind the exact durable send-attempt"
    );
    let operation = outbox_by_id_on(connection, &envelope.outbox_id)?
        .ok_or_else(|| anyhow::anyhow!("authenticated observation outbox is missing"))?;
    ensure!(
        operation.envelope.outbox_digest == envelope.outbox_digest
            && operation.envelope.command_id == envelope.command_id
            && operation.envelope.command_digest == envelope.command_digest
            && operation.envelope.operation_kind == envelope.operation_kind
            && operation.provider_id == envelope.provider_id
            && operation.adapter_id == envelope.adapter_id
            && operation.envelope.adapter_binding_digest == envelope.adapter_binding_digest
            && operation.envelope.route_authorization_id == stored_send.route_authorization_id
            && operation.envelope.route_authorization_digest
                == stored_send.route_authorization_digest,
        "authenticated observation does not bind the exact route and command"
    );
    Ok(())
}

fn apply_observation_effects_on(
    connection: &Connection,
    verified: &VerifiedComputeStartOutboxRemoteObservation,
) -> Result<()> {
    let envelope = verified.envelope();
    if envelope.observation_kind == COMPUTE_OBSERVATION_CANCEL_RESPONSE {
        unlock_reconcile_after_cancel_on(connection, envelope)?;
    }
    let _ = record_remote_never_committed_no_start_on(connection, verified)?;
    Ok(())
}

fn observation_replay_on(
    connection: &Connection,
    expected: &ComputeStartOutboxRemoteObservationEnvelope,
) -> Result<Option<StoredVerifiedObservation>> {
    let mut statement = connection.prepare(
        "SELECT observation_json, observation_digest
           FROM compute_attempt_start_remote_observations
          WHERE observation_id=?1
             OR (provider_id=?2 AND adapter_id=?3 AND adapter_observation_id=?4)
          ORDER BY observation_id LIMIT 2",
    )?;
    let rows = statement
        .query_map(
            params![
                expected.observation_id,
                expected.provider_id,
                expected.adapter_id,
                expected.adapter_observation_id,
            ],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if rows.is_empty() {
        return Ok(None);
    }
    ensure!(
        rows.len() == 1,
        "Start observation replay identity is ambiguous"
    );
    let Some((json, stored_digest)) = rows.into_iter().next() else {
        bail!("Start observation replay row disappeared after cardinality audit");
    };
    let envelope: ComputeStartOutboxRemoteObservationEnvelope = serde_json::from_str(&json)?;
    let (canonical, digest) = canonical_start_outbox_remote_observation_json_and_digest(&envelope)?;
    ensure!(
        canonical == json && digest == envelope.observation_digest && digest == stored_digest,
        "stored Start observation failed canonical replay audit"
    );
    Ok(Some(StoredVerifiedObservation { envelope }))
}

fn receipt(
    envelope: &ComputeStartOutboxRemoteObservationEnvelope,
    replayed: bool,
) -> StartOutboxObservationReceipt {
    StartOutboxObservationReceipt {
        observation_id: envelope.observation_id.clone(),
        observation_digest: envelope.observation_digest.clone(),
        outbox_id: envelope.outbox_id.clone(),
        replayed,
    }
}

fn next_store_time_after(value: &str) -> Result<String> {
    let floor = DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc) + Duration::nanoseconds(1);
    Ok(std::cmp::max(Utc::now(), floor).to_rfc3339_opts(SecondsFormat::Nanos, true))
}
