use anyhow::{ensure, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, types::Value, Connection, OptionalExtension};

use crate::{
    compute_federation::start_outbox::{
        canonical_start_outbox_send_attempt_json_and_digest, ComputeStartOutboxSendAttemptEnvelope,
        COMPUTE_OUTBOX_STATE_CLAIMED, COMPUTE_OUTBOX_STATE_IN_FLIGHT_UNKNOWN,
        COMPUTE_START_OUTBOX_CANONICALIZATION, COMPUTE_START_OUTBOX_DIGEST_ALGORITHM,
        COMPUTE_START_OUTBOX_SEND_ATTEMPT_SCHEMA,
    },
    store::{hash_token, new_id},
};

use super::{
    currentness::ensure_send_current_on,
    read::outbox_by_id_on,
    types::{PreparedStartSendMutation, PreparedStartSendRequest, StartOutboxClaimHandle},
};

mod commit_source;

pub(super) use commit_source::ensure_commit_source_current_on;

pub(super) fn record_send_started_on(
    connection: &Connection,
    claim: &StartOutboxClaimHandle,
    request: &PreparedStartSendRequest,
) -> Result<ComputeStartOutboxSendAttemptEnvelope> {
    let started_at = now_nanos();
    let mutation = prepare_send_started_at_on(connection, claim, request, &started_at)?;
    insert_prepared_send_started_on(connection, &mutation)?;
    finish_prepared_send_started_on(connection, claim, mutation)
}

pub(in crate::store) fn prepare_send_started_at_on(
    connection: &Connection,
    claim: &StartOutboxClaimHandle,
    request: &PreparedStartSendRequest,
    started_at: &str,
) -> Result<PreparedStartSendMutation> {
    ensure_digest(&request.request_digest, "sealed Start request digest")?;
    ensure_fixed_timestamp(started_at)?;
    let stored = outbox_by_id_on(connection, &claim.receipt.outbox_id)?
        .ok_or_else(|| anyhow::anyhow!("claimed Start outbox operation is missing"))?;
    ensure!(
        stored.envelope == claim.operation.envelope
            && stored.projection.state == COMPUTE_OUTBOX_STATE_CLAIMED
            && stored.projection.state_revision == claim.receipt.state_revision
            && stored.projection.attempt_count + 1 == claim.receipt.attempt_no
            && stored.projection.claim_owner_id.as_deref()
                == Some(claim.receipt.claim_owner_id.as_str())
            && stored.projection.claim_generation == claim.receipt.claim_generation
            && stored.projection.claim_token_digest.as_deref()
                == Some(claim.receipt.claim_token_digest.as_str())
            && stored.projection.claim_expires_at.as_deref()
                == Some(claim.receipt.claim_expires_at.as_str())
            && hash_token(&claim.raw_claim_token) == claim.receipt.claim_token_digest
            && started_at > stored.projection.updated_at.as_str()
            && started_at < claim.receipt.claim_expires_at.as_str(),
        "Start send claim custody is stale or unauthenticated"
    );
    let prior_send = connection
        .query_row(
            "SELECT 1 FROM compute_attempt_start_send_attempts
              WHERE outbox_id=?1 LIMIT 1",
            params![stored.envelope.outbox_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    ensure!(
        !prior_send,
        "Start outbox already has durable send-start evidence; reconcile instead of resending"
    );
    ensure_send_current_on(connection, &stored, started_at)?;
    let mut envelope = ComputeStartOutboxSendAttemptEnvelope {
        schema: COMPUTE_START_OUTBOX_SEND_ATTEMPT_SCHEMA.to_string(),
        send_attempt_id: new_id("start_send"),
        send_attempt_digest: String::new(),
        canonicalization: COMPUTE_START_OUTBOX_CANONICALIZATION.to_string(),
        digest_algorithm: COMPUTE_START_OUTBOX_DIGEST_ALGORITHM.to_string(),
        outbox_id: stored.envelope.outbox_id.clone(),
        outbox_digest: stored.envelope.outbox_digest.clone(),
        attempt_no: stored.projection.attempt_count + 1,
        command_id: stored.envelope.command_id.clone(),
        command_digest: stored.envelope.command_digest.clone(),
        operation_kind: stored.envelope.operation_kind.clone(),
        route_authorization_id: stored.envelope.route_authorization_id.clone(),
        route_authorization_digest: stored.envelope.route_authorization_digest.clone(),
        claim_generation: stored.projection.claim_generation,
        claim_token_digest: claim.receipt.claim_token_digest.clone(),
        request_digest: request.request_digest.clone(),
        started_at: started_at.to_string(),
    };
    let (_, digest) = canonical_start_outbox_send_attempt_json_and_digest(&envelope)?;
    envelope.send_attempt_digest = digest;
    let (json, recomputed) = canonical_start_outbox_send_attempt_json_and_digest(&envelope)?;
    ensure!(
        recomputed == envelope.send_attempt_digest,
        "Start send-attempt failed canonical audit"
    );
    Ok(PreparedStartSendMutation {
        stored,
        envelope,
        canonical_json: json,
    })
}

pub(in crate::store) fn prepared_send_attempt_envelope(
    mutation: &PreparedStartSendMutation,
) -> &ComputeStartOutboxSendAttemptEnvelope {
    &mutation.envelope
}

pub(in crate::store) fn prepared_send_attempt_values(
    mutation: &PreparedStartSendMutation,
) -> Vec<Value> {
    let envelope = &mutation.envelope;
    vec![
        Value::Text(envelope.send_attempt_id.clone()),
        Value::Text(envelope.schema.clone()),
        Value::Text(envelope.send_attempt_digest.clone()),
        Value::Text(mutation.canonical_json.clone()),
        Value::Text(envelope.canonicalization.clone()),
        Value::Text(envelope.digest_algorithm.clone()),
        Value::Text(envelope.outbox_id.clone()),
        Value::Text(envelope.outbox_digest.clone()),
        Value::Integer(envelope.attempt_no),
        Value::Text(envelope.operation_kind.clone()),
        Value::Text(envelope.command_id.clone()),
        Value::Text(envelope.command_digest.clone()),
        Value::Text(envelope.route_authorization_id.clone()),
        Value::Text(envelope.route_authorization_digest.clone()),
        Value::Integer(envelope.claim_generation),
        Value::Text(envelope.claim_token_digest.clone()),
        Value::Text(envelope.request_digest.clone()),
        Value::Text(envelope.started_at.clone()),
    ]
}

pub(in crate::store) fn prepared_send_outbox_cas_values(
    mutation: &PreparedStartSendMutation,
    claim: &StartOutboxClaimHandle,
) -> Vec<Value> {
    vec![
        Value::Text(mutation.stored.envelope.outbox_id.clone()),
        Value::Text(mutation.stored.envelope.outbox_digest.clone()),
        Value::Text(COMPUTE_OUTBOX_STATE_CLAIMED.to_string()),
        Value::Text(COMPUTE_OUTBOX_STATE_IN_FLIGHT_UNKNOWN.to_string()),
        Value::Integer(mutation.stored.projection.state_revision),
        Value::Integer(mutation.stored.projection.state_revision + 1),
        Value::Integer(mutation.stored.projection.attempt_count),
        Value::Integer(mutation.stored.projection.attempt_count + 1),
        Value::Text(claim.receipt.claim_owner_id.clone()),
        Value::Text(claim.receipt.claim_owner_id.clone()),
        Value::Text(claim.receipt.claim_token_digest.clone()),
        Value::Text(claim.receipt.claim_token_digest.clone()),
        Value::Integer(claim.receipt.claim_generation),
        Value::Integer(claim.receipt.claim_generation),
        Value::Text(claim.receipt.claim_expires_at.clone()),
        Value::Text(claim.receipt.claim_expires_at.clone()),
        Value::Text(mutation.envelope.started_at.clone()),
    ]
}

pub(in crate::store) fn insert_prepared_send_started_on(
    connection: &Connection,
    mutation: &PreparedStartSendMutation,
) -> Result<()> {
    let envelope = &mutation.envelope;
    let changed = connection.execute(
        "INSERT INTO compute_attempt_start_send_attempts (
            send_attempt_id, send_attempt_schema, send_attempt_digest,
            send_attempt_json, canonicalization, digest_algorithm,
            outbox_id, outbox_digest, attempt_no, operation_kind,
            command_id, command_digest, route_authorization_id,
            route_authorization_digest, claim_generation, claim_token_digest,
            request_digest, started_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
            ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18
         )",
        params![
            envelope.send_attempt_id,
            envelope.schema,
            envelope.send_attempt_digest,
            mutation.canonical_json,
            envelope.canonicalization,
            envelope.digest_algorithm,
            envelope.outbox_id,
            envelope.outbox_digest,
            envelope.attempt_no,
            envelope.operation_kind,
            envelope.command_id,
            envelope.command_digest,
            envelope.route_authorization_id,
            envelope.route_authorization_digest,
            envelope.claim_generation,
            envelope.claim_token_digest,
            envelope.request_digest,
            envelope.started_at,
        ],
    )?;
    ensure!(
        changed == 1,
        "Start send-attempt insert changed an unexpected row count"
    );
    Ok(())
}

pub(in crate::store) fn finish_prepared_send_started_on(
    connection: &Connection,
    claim: &StartOutboxClaimHandle,
    mutation: PreparedStartSendMutation,
) -> Result<ComputeStartOutboxSendAttemptEnvelope> {
    let stored = &mutation.stored;
    let envelope = &mutation.envelope;
    let changed = connection.execute(
        "UPDATE compute_attempt_start_outbox
            SET state='in_flight_unknown', state_revision=state_revision+1,
                attempt_count=attempt_count+1, updated_at=?1
          WHERE outbox_id=?2 AND state='claimed' AND state_revision=?3
            AND attempt_count=?4 AND claim_owner_id=?5 AND claim_token_digest=?6
            AND claim_generation=?7 AND claim_expires_at=?8",
        params![
            envelope.started_at,
            stored.envelope.outbox_id,
            stored.projection.state_revision,
            stored.projection.attempt_count,
            claim.receipt.claim_owner_id,
            claim.receipt.claim_token_digest,
            claim.receipt.claim_generation,
            claim.receipt.claim_expires_at,
        ],
    )?;
    ensure!(
        changed == 1,
        "Start send state transition lost its exact claim CAS"
    );
    let after = outbox_by_id_on(connection, &stored.envelope.outbox_id)?
        .ok_or_else(|| anyhow::anyhow!("Start outbox disappeared after send-start"))?;
    ensure!(
        after.projection.state == COMPUTE_OUTBOX_STATE_IN_FLIGHT_UNKNOWN
            && after.projection.state_revision == stored.projection.state_revision + 1
            && after.projection.attempt_count == envelope.attempt_no
            && after.projection.claim_owner_id == stored.projection.claim_owner_id
            && after.projection.claim_token_digest == stored.projection.claim_token_digest
            && after.projection.claim_generation == stored.projection.claim_generation
            && after.projection.claim_expires_at == stored.projection.claim_expires_at,
        "Start send durable readback failed exact unknown-delivery audit"
    );
    Ok(mutation.envelope)
}

fn ensure_digest(value: &str, label: &str) -> Result<()> {
    ensure!(
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
        "{label} is not a lowercase SHA-256 digest"
    );
    Ok(())
}

fn ensure_fixed_timestamp(value: &str) -> Result<()> {
    ensure!(
        value.len() == 30 && value.as_bytes().get(19) == Some(&b'.') && value.ends_with('Z'),
        "Start send timestamp must be fixed UTC nanoseconds"
    );
    let _: chrono::DateTime<chrono::FixedOffset> = chrono::DateTime::parse_from_rfc3339(value)?;
    Ok(())
}

fn now_nanos() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
