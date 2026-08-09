use anyhow::{anyhow, bail, ensure, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    compute_federation::{
        attempt_gateway::{VerifiedComputeAttemptAdapterAck, COMPUTE_ATTEMPT_ADAPTER_ACK_REJECTED},
        start_outbox::{
            canonical_start_no_start_proof_json_and_digest, ComputeStartNoStartProofEnvelope,
            VerifiedComputeStartOutboxRemoteObservation, COMPUTE_NO_START_PROOF_LOCAL_NEVER_SENT,
            COMPUTE_NO_START_PROOF_PREPARE_REJECTED, COMPUTE_NO_START_PROOF_REMOTE_NEVER_COMMITTED,
            COMPUTE_OBSERVATION_PREPARE_RESPONSE, COMPUTE_OBSERVATION_RECONCILE_ATTESTATION,
            COMPUTE_REMOTE_EXECUTION_REJECTED, COMPUTE_REMOTE_EXECUTION_TERMINAL_NO_START,
            COMPUTE_REMOTE_TERMINALITY_FINAL, COMPUTE_START_NO_START_PROOF_SCHEMA,
            COMPUTE_START_OUTBOX_CANONICALIZATION, COMPUTE_START_OUTBOX_DIGEST_ALGORITHM,
        },
    },
    store::new_id,
};

use super::super::{
    cleanup::ensure_unknown_prepare_cleanup_on,
    read::no_start_source_on,
    types::{NoStartProofSource, StartNoStartRecoveryReceipt, StartOutboxNoStartProofReceipt},
};
use super::{persist_proof_on, proof_by_command_on, proof_receipt};

#[derive(Clone, Copy)]
enum DurableNoStartCause<'a> {
    LocalNeverSent {
        command_id: &'a str,
    },
    PrepareRejected {
        command_id: &'a str,
        observation_id: &'a str,
    },
    RemoteNeverCommitted {
        command_id: &'a str,
        observation_id: &'a str,
    },
}

impl<'a> DurableNoStartCause<'a> {
    fn command_id(self) -> &'a str {
        match self {
            Self::LocalNeverSent { command_id }
            | Self::PrepareRejected { command_id, .. }
            | Self::RemoteNeverCommitted { command_id, .. } => command_id,
        }
    }

    fn proof_kind(self) -> &'static str {
        match self {
            Self::LocalNeverSent { .. } => COMPUTE_NO_START_PROOF_LOCAL_NEVER_SENT,
            Self::PrepareRejected { .. } => COMPUTE_NO_START_PROOF_PREPARE_REJECTED,
            Self::RemoteNeverCommitted { .. } => COMPUTE_NO_START_PROOF_REMOTE_NEVER_COMMITTED,
        }
    }

    fn observation_id(self) -> Option<&'a str> {
        match self {
            Self::LocalNeverSent { .. } => None,
            Self::PrepareRejected { observation_id, .. }
            | Self::RemoteNeverCommitted { observation_id, .. } => Some(observation_id),
        }
    }
}

struct DurableObservationEvidence {
    observation_id: String,
    observation_digest: String,
    no_commit_tombstone_id: Option<String>,
    no_commit_tombstone_digest: Option<String>,
    proven_at: String,
}

pub(in crate::store) fn record_prepare_rejected_no_start_on(
    connection: &Connection,
    verified: &VerifiedComputeAttemptAdapterAck,
) -> Result<StartOutboxNoStartProofReceipt> {
    let ack = verified.ack();
    let observation = verified.prepare_observation().envelope();
    ensure!(
        ack.outcome == COMPUTE_ATTEMPT_ADAPTER_ACK_REJECTED
            && observation.command_id == ack.command_id
            && observation.observation_kind == COMPUTE_OBSERVATION_PREPARE_RESPONSE
            && observation.response_outcome == COMPUTE_ATTEMPT_ADAPTER_ACK_REJECTED
            && observation.remote_execution_state == COMPUTE_REMOTE_EXECUTION_REJECTED
            && observation.terminality == COMPUTE_REMOTE_TERMINALITY_FINAL,
        "prepare-rejected proof requires the exact final authenticated rejection"
    );
    derive_and_record_on(
        connection,
        DurableNoStartCause::PrepareRejected {
            command_id: &ack.command_id,
            observation_id: &observation.observation_id,
        },
    )
}

pub(in crate::store::compute_attempt_start_outbox) fn record_remote_never_committed_no_start_on(
    connection: &Connection,
    verified: &VerifiedComputeStartOutboxRemoteObservation,
) -> Result<Option<StartOutboxNoStartProofReceipt>> {
    let observation = verified.envelope();
    if observation.observation_kind != COMPUTE_OBSERVATION_RECONCILE_ATTESTATION
        || observation.remote_execution_state != COMPUTE_REMOTE_EXECUTION_TERMINAL_NO_START
    {
        return Ok(None);
    }
    ensure!(
        observation.response_outcome == "observed"
            && observation.terminality == COMPUTE_REMOTE_TERMINALITY_FINAL
            && observation.no_commit_tombstone_id.is_some()
            && observation.no_commit_tombstone_digest.is_some(),
        "remote-never-committed proof requires a final observed tombstone"
    );
    derive_and_record_on(
        connection,
        DurableNoStartCause::RemoteNeverCommitted {
            command_id: &observation.command_id,
            observation_id: &observation.observation_id,
        },
    )
    .map(Some)
}

pub(in crate::store::compute_attempt_start_outbox) fn recover_no_start_on(
    connection: &Connection,
    command_id: &str,
) -> Result<StartNoStartRecoveryReceipt> {
    if let Some(proof) = proof_by_command_on(connection, command_id)? {
        return Ok(StartNoStartRecoveryReceipt::ProofRecorded(proof_receipt(
            &proof, true,
        )));
    }
    if let Some(observation_id) = rejected_observation_id_on(connection, command_id)? {
        return derive_and_record_on(
            connection,
            DurableNoStartCause::PrepareRejected {
                command_id,
                observation_id: &observation_id,
            },
        )
        .map(StartNoStartRecoveryReceipt::ProofRecorded);
    }
    if let Some(observation_id) = final_reconcile_observation_id_on(connection, command_id)? {
        return derive_and_record_on(
            connection,
            DurableNoStartCause::RemoteNeverCommitted {
                command_id,
                observation_id: &observation_id,
            },
        )
        .map(StartNoStartRecoveryReceipt::ProofRecorded);
    }
    if let Some(proof) = derive_local_never_sent_if_due_on(connection, command_id)? {
        return Ok(StartNoStartRecoveryReceipt::ProofRecorded(proof));
    }
    if let Some(cleanup) = ensure_unknown_prepare_cleanup_on(connection, command_id)? {
        return Ok(StartNoStartRecoveryReceipt::CleanupEnqueued(cleanup));
    }
    bail!("COMPUTE_ATTEMPT_START_NO_START_RECOVERY_NOT_READY")
}

pub(super) fn derive_local_never_sent_if_due_on(
    connection: &Connection,
    command_id: &str,
) -> Result<Option<StartOutboxNoStartProofReceipt>> {
    if let Some(stored) = proof_by_command_on(connection, command_id)? {
        return Ok(Some(proof_receipt(&stored, true)));
    }
    let source = no_start_source_on(connection, command_id)?
        .ok_or_else(|| anyhow!("no-start recovery lacks an exact command and prepare closure"))?;
    let now = now_nanos();
    let safely_due = source.prepare_not_after.as_str() <= now.as_str()
        && (source.prepare_state == "pending"
            || source.prepare_state == "abandoned_no_send"
            || (source.prepare_state == "claimed"
                && source
                    .prepare_claim_expires_at
                    .as_deref()
                    .is_some_and(|expiry| expiry <= now.as_str())));
    if !safely_due {
        return Ok(None);
    }
    derive_and_record_at_on(
        connection,
        DurableNoStartCause::LocalNeverSent { command_id },
        &now,
    )
    .map(Some)
}

fn derive_and_record_on(
    connection: &Connection,
    cause: DurableNoStartCause<'_>,
) -> Result<StartOutboxNoStartProofReceipt> {
    let recorded_at = now_nanos();
    derive_and_record_at_on(connection, cause, &recorded_at)
}

fn derive_and_record_at_on(
    connection: &Connection,
    cause: DurableNoStartCause<'_>,
    recorded_at: &str,
) -> Result<StartOutboxNoStartProofReceipt> {
    if let Some(stored) = proof_by_command_on(connection, cause.command_id())? {
        ensure_cause_matches(&stored, cause)?;
        return Ok(proof_receipt(&stored, true));
    }
    let source = no_start_source_on(connection, cause.command_id())?
        .ok_or_else(|| anyhow!("no-start derivation lacks an exact command and prepare closure"))?;
    let evidence =
        match cause {
            DurableNoStartCause::LocalNeverSent { .. } => {
                abandon_local_never_sent_on(connection, &source, recorded_at)?;
                None
            }
            DurableNoStartCause::PrepareRejected { observation_id, .. } => Some(
                exact_observation_on(connection, &source, observation_id, cause.proof_kind())?,
            ),
            DurableNoStartCause::RemoteNeverCommitted { observation_id, .. } => Some(
                exact_observation_on(connection, &source, observation_id, cause.proof_kind())?,
            ),
        };
    let proven_at = evidence
        .as_ref()
        .map_or_else(|| recorded_at.to_string(), |value| value.proven_at.clone());
    ensure!(
        proven_at.as_str() <= recorded_at,
        "no-start proof cannot be recorded before it is proven"
    );
    let mut envelope = proof_envelope(&source, cause, evidence.as_ref(), proven_at, recorded_at);
    let (_, digest) = canonical_start_no_start_proof_json_and_digest(&envelope)?;
    envelope.proof_digest = digest;
    persist_proof_on(connection, &envelope)?;
    let stored = proof_by_command_on(connection, &envelope.command_id)?
        .ok_or_else(|| anyhow!("no-start proof is not visible after insert"))?;
    ensure!(
        stored == envelope,
        "no-start proof failed exact durable readback"
    );
    Ok(proof_receipt(&stored, false))
}

fn proof_envelope(
    source: &NoStartProofSource,
    cause: DurableNoStartCause<'_>,
    evidence: Option<&DurableObservationEvidence>,
    proven_at: String,
    recorded_at: &str,
) -> ComputeStartNoStartProofEnvelope {
    ComputeStartNoStartProofEnvelope {
        schema: COMPUTE_START_NO_START_PROOF_SCHEMA.to_string(),
        proof_id: new_id("start_no_start"),
        proof_digest: String::new(),
        canonicalization: COMPUTE_START_OUTBOX_CANONICALIZATION.to_string(),
        digest_algorithm: COMPUTE_START_OUTBOX_DIGEST_ALGORITHM.to_string(),
        proof_kind: cause.proof_kind().to_string(),
        outbox_id: source.outbox_id.clone(),
        outbox_digest: source.outbox_digest.clone(),
        command_id: source.command_id.clone(),
        command_digest: source.command_digest.clone(),
        plan_id: source.plan_id.clone(),
        plan_digest: source.plan_digest.clone(),
        provider_id: source.provider_id.clone(),
        reservation_id: source.reservation_id.clone(),
        reservation_revision: source.reservation_revision,
        reservation_digest: source.reservation_digest.clone(),
        job_id: source.job_id.clone(),
        job_revision: source.job_revision,
        job_digest: source.job_digest.clone(),
        capacity_claim_id: source.capacity_claim_id.clone(),
        capacity_claim_revision: source.capacity_claim_revision,
        capacity_claim_digest: source.capacity_claim_digest.clone(),
        budget_reservation_id: source.budget_reservation_id.clone(),
        budget_reserved_fen: source.budget_reserved_fen,
        broker_request_digest: source.broker_request_digest.clone(),
        lease_id: source.lease_id.clone(),
        lease_digest: None,
        fencing_generation: source.fencing_generation,
        adapter_id: source.adapter_id.clone(),
        adapter_revision: source.adapter_revision,
        adapter_registry_digest: source.adapter_registry_digest.clone(),
        adapter_binding_digest: source.adapter_binding_digest.clone(),
        route_authorization_id: source.route_authorization_id.clone(),
        route_authorization_digest: source.route_authorization_digest.clone(),
        observation_id: evidence.map(|value| value.observation_id.clone()),
        observation_digest: evidence.map(|value| value.observation_digest.clone()),
        no_commit_tombstone_id: evidence.and_then(|value| value.no_commit_tombstone_id.clone()),
        no_commit_tombstone_digest: evidence
            .and_then(|value| value.no_commit_tombstone_digest.clone()),
        proven_at,
        recorded_at: recorded_at.to_string(),
    }
}

fn abandon_local_never_sent_on(
    connection: &Connection,
    source: &NoStartProofSource,
    proven_at: &str,
) -> Result<()> {
    ensure!(
        source.prepare_not_after.as_str() <= proven_at,
        "local-never-sent cannot be proven before the delivery window closes"
    );
    if source.prepare_state == "abandoned_no_send" {
        return Ok(());
    }
    ensure!(
        source.prepare_state == "pending"
            || (source.prepare_state == "claimed"
                && source
                    .prepare_claim_expires_at
                    .as_deref()
                    .is_some_and(|expiry| expiry <= proven_at)),
        "local-never-sent source is not safely abandonable"
    );
    let changed = connection.execute(
        "UPDATE compute_attempt_start_outbox
            SET state='abandoned_no_send', state_revision=state_revision+1,
                claim_owner_id=NULL, claim_token_digest=NULL, claim_expires_at=NULL,
                last_failure_code='DELIVERY_WINDOW_CLOSED_BEFORE_SEND', updated_at=?1
          WHERE outbox_id=?2 AND state=?3 AND state_revision=?4
            AND attempt_count=?5 AND claim_generation=?6
            AND NOT EXISTS (
                SELECT 1 FROM compute_attempt_start_send_attempts send
                 WHERE send.outbox_id=compute_attempt_start_outbox.outbox_id
            )",
        params![
            proven_at,
            source.outbox_id,
            source.prepare_state,
            source.prepare_state_revision,
            source.prepare_attempt_count,
            source.prepare_claim_generation,
        ],
    )?;
    ensure!(
        changed == 1,
        "local-never-sent abandonment lost its exact CAS"
    );
    Ok(())
}

fn exact_observation_on(
    connection: &Connection,
    source: &NoStartProofSource,
    observation_id: &str,
    proof_kind: &str,
) -> Result<DurableObservationEvidence> {
    let row = match proof_kind {
        COMPUTE_NO_START_PROOF_PREPARE_REJECTED => connection
            .query_row(
                "SELECT observation.observation_id, observation.observation_digest,
                        observation.no_commit_tombstone_id,
                        observation.no_commit_tombstone_digest, ack.created_at
                   FROM compute_attempt_start_remote_observations observation
                   JOIN compute_attempt_dispatch_acks ack
                     ON ack.command_id=observation.command_id
                  WHERE observation.observation_id=?1 AND observation.command_id=?2
                    AND observation.outbox_id=?3
                    AND observation.observation_kind='prepare_response'
                    AND observation.response_outcome='rejected'
                    AND observation.remote_execution_state='rejected'
                    AND observation.terminality='final'
                    AND ack.outcome='rejected' AND ack.disposition='rejected'
                    AND ack.adapter_ack_id=observation.adapter_observation_id",
                params![observation_id, source.command_id, source.outbox_id],
                evidence_row,
            )
            .optional()?,
        COMPUTE_NO_START_PROOF_REMOTE_NEVER_COMMITTED => connection
            .query_row(
                "SELECT observation.observation_id, observation.observation_digest,
                        observation.no_commit_tombstone_id,
                        observation.no_commit_tombstone_digest, observation.recorded_at
                   FROM compute_attempt_start_remote_observations observation
                   JOIN compute_attempt_start_outbox reconcile
                     ON reconcile.outbox_id=observation.outbox_id
                   JOIN compute_attempt_start_outbox cancel
                     ON cancel.outbox_id=reconcile.subject_outbox_id
                  WHERE observation.observation_id=?1 AND observation.command_id=?2
                    AND observation.observation_kind='reconcile_attestation'
                    AND observation.response_outcome='observed'
                    AND observation.remote_execution_state='terminal_no_start'
                    AND observation.terminality='final'
                    AND observation.no_commit_tombstone_id IS NOT NULL
                    AND observation.no_commit_tombstone_digest IS NOT NULL
                    AND reconcile.operation_kind='reconcile'
                    AND reconcile.state='delivery_observed'
                    AND reconcile.outbox_digest=observation.outbox_digest
                    AND reconcile.command_digest=observation.command_digest
                    AND cancel.operation_kind='cancel'
                    AND cancel.state='delivery_observed'
                    AND cancel.subject_outbox_id=?3
                    AND cancel.ack_id IS reconcile.ack_id
                    AND cancel.ack_digest IS reconcile.ack_digest",
                params![observation_id, source.command_id, source.outbox_id],
                evidence_row,
            )
            .optional()?,
        _ => bail!("unsupported observation-backed no-start proof"),
    };
    row.ok_or_else(|| anyhow!("no-start derivation lacks exact authenticated observation"))
}

fn evidence_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DurableObservationEvidence> {
    Ok(DurableObservationEvidence {
        observation_id: row.get(0)?,
        observation_digest: row.get(1)?,
        no_commit_tombstone_id: row.get(2)?,
        no_commit_tombstone_digest: row.get(3)?,
        proven_at: row.get(4)?,
    })
}

fn rejected_observation_id_on(connection: &Connection, command_id: &str) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT observation.observation_id
               FROM compute_attempt_start_remote_observations observation
               JOIN compute_attempt_dispatch_acks ack
                 ON ack.command_id=observation.command_id
              WHERE observation.command_id=?1
                AND observation.observation_kind='prepare_response'
                AND observation.response_outcome='rejected'
                AND observation.remote_execution_state='rejected'
                AND observation.terminality='final'
                AND ack.outcome='rejected' AND ack.disposition='rejected'
                AND ack.adapter_ack_id=observation.adapter_observation_id",
            params![command_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

fn final_reconcile_observation_id_on(
    connection: &Connection,
    command_id: &str,
) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT observation.observation_id
               FROM compute_attempt_start_remote_observations observation
              WHERE observation.command_id=?1
                AND observation.observation_kind='reconcile_attestation'
                AND observation.response_outcome='observed'
                AND observation.remote_execution_state='terminal_no_start'
                AND observation.terminality='final'
                AND observation.no_commit_tombstone_id IS NOT NULL
                AND observation.no_commit_tombstone_digest IS NOT NULL",
            params![command_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

fn ensure_cause_matches(
    stored: &ComputeStartNoStartProofEnvelope,
    cause: DurableNoStartCause<'_>,
) -> Result<()> {
    ensure!(
        stored.command_id == cause.command_id()
            && stored.proof_kind == cause.proof_kind()
            && stored.observation_id.as_deref() == cause.observation_id(),
        "no-start proof replay conflicts with the durable cause"
    );
    Ok(())
}

fn now_nanos() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
