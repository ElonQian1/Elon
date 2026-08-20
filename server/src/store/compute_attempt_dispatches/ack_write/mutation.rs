use anyhow::{anyhow, bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection};

use crate::{
    compute_federation::attempt_gateway::{
        ComputeAttemptAdapterAckEnvelope, VerifiedComputeAttemptAdapterAckView,
    },
    store::{
        compute_attempt_start_outbox::enqueue_quarantined_cleanup_on,
        ActivateComputeAttemptRequest, ComputeAttemptActivationReceipt,
    },
};

use super::super::{
    read::{ack_by_command_on, ack_receipt, ensure_ack_replay_matches, StoredDispatchCommand},
    types::ComputeAttemptDispatchAckCommit,
    validation::{application_id_for_ack, PreparedApplication, PreparedVerifiedAck},
};

pub(super) fn quarantine_ack_on(
    connection: &Connection,
    command: &StoredDispatchCommand,
    verified: &dyn VerifiedComputeAttemptAdapterAckView,
    prepared: &PreparedVerifiedAck,
    reason: &str,
    created_at: &str,
) -> Result<ComputeAttemptDispatchAckCommit> {
    enqueue_quarantined_cleanup_on(connection, verified, created_at)?;
    insert_ack_on(
        connection,
        command,
        verified,
        prepared,
        "quarantined",
        Some(reason),
        None,
        created_at,
    )?;
    let stored = ack_by_command_on(connection, &verified.ack().command_id)?
        .ok_or_else(|| anyhow!("Quarantined Adapter ACK is not visible after insert"))?;
    ensure_ack_replay_matches(&stored, verified, prepared)?;
    Ok(ComputeAttemptDispatchAckCommit::Quarantined {
        ack: ack_receipt(stored, false),
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn insert_ack_on(
    connection: &Connection,
    command: &StoredDispatchCommand,
    verified: &dyn VerifiedComputeAttemptAdapterAckView,
    prepared: &PreparedVerifiedAck,
    disposition: &str,
    disposition_reason_code: Option<&str>,
    activation_lease_id: Option<&str>,
    created_at: &str,
) -> Result<()> {
    let ack = verified.ack();
    let application_id = (disposition == "accepted_applied").then(|| application_id_for_ack(ack));
    connection.execute(
        "INSERT INTO compute_attempt_dispatch_acks (
            ack_id, command_id, provider_id, adapter_id, adapter_ack_id,
            command_digest, adapter_binding_digest, outcome, disposition,
            disposition_reason_code, activation_lease_id, application_id,
            remote_execution_ref, reason_code,
            ack_json, ack_digest, observed_at, received_at, created_at
         ) VALUES (
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19
         )",
        params![
            ack.ack_id,
            ack.command_id,
            command.adapter.provider_id,
            command.adapter.adapter_id,
            ack.adapter_ack_id,
            ack.command_digest,
            prepared.adapter_digest,
            ack.outcome,
            disposition,
            disposition_reason_code,
            activation_lease_id,
            application_id,
            ack.remote_execution_ref,
            ack.reason_code,
            prepared.ack_json,
            prepared.ack_digest,
            ack.observed_at,
            ack.received_at,
            created_at,
        ],
    )?;
    Ok(())
}

pub(super) fn insert_application_on(
    connection: &Connection,
    ack: &ComputeAttemptAdapterAckEnvelope,
    activation: &ComputeAttemptActivationReceipt,
    prepared: &PreparedApplication,
    created_at: &str,
) -> Result<()> {
    connection.execute(
        "INSERT INTO compute_attempt_dispatch_applications (
            application_id, command_id, ack_id, action, lease_id,
            activation_request_digest, lease_digest, application_json,
            application_digest, applied_at, created_at
         ) VALUES (?1,?2,?3,'v185_activate',?4,?5,?6,?7,?8,?9,?10)",
        params![
            prepared.application_id,
            ack.command_id,
            ack.ack_id,
            activation.lease.lease_id,
            activation.request_digest,
            activation.lease_digest,
            prepared.application_json,
            prepared.application_digest,
            activation.activated_at,
            created_at,
        ],
    )?;
    Ok(())
}

pub(super) fn application_created_at(activated_at: &str) -> Result<String> {
    let activated_at = chrono::DateTime::parse_from_rfc3339(activated_at)?.with_timezone(&Utc);
    Ok(std::cmp::max(activated_at, Utc::now()).to_rfc3339_opts(SecondsFormat::Nanos, true))
}

pub(super) fn now_dispatch() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}

pub(super) fn activation_request(
    command: &StoredDispatchCommand,
    remote_execution_ref: &str,
) -> ActivateComputeAttemptRequest {
    let start = &command.command.command;
    ActivateComputeAttemptRequest {
        lease_id: start.identity.attempt_lease_id.clone(),
        reservation_id: start.identity.reservation_id.clone(),
        provider_id: start.provider.provider_id.clone(),
        executor_id: start.executor_id.clone(),
        shard_id: start.identity.shard_id.clone(),
        attempt_no: start.identity.attempt_no,
        fencing_generation: start.identity.fencing_generation,
        executor_acceptance_ref: remote_execution_ref.to_string(),
        lease_credential_ref: command.lease_credential_ref.clone(),
        lease_credential_hint: command.lease_credential_hint.clone(),
        expected_job_revision: start.job.job_revision,
        expected_job_digest: start.job.job_digest.clone(),
        expected_reservation_revision: start.reservation.reservation_revision,
        expected_reservation_digest: start.reservation.reservation_digest.clone(),
        expected_claim_revision: start.capacity_claim.claim_revision,
        expected_claim_digest: start.capacity_claim.claim_digest.clone(),
        expires_at: start.lease_expires_at.clone(),
        hard_deadline_at: start.hard_deadline_at.clone(),
        idempotency_key: command.activation_idempotency_key.clone(),
        activated_by_user_id: command.activated_by_user_id.clone(),
    }
}

pub(super) fn ensure_ack_binds_command(
    command: &StoredDispatchCommand,
    verified: &dyn VerifiedComputeAttemptAdapterAckView,
    prepared: &PreparedVerifiedAck,
) -> Result<()> {
    let ack = verified.ack();
    if ack.command_digest != command.command.command_digest
        || ack.adapter_binding_digest != command.adapter_binding_digest
        || prepared.adapter_digest != command.adapter_binding_digest
        || verified.adapter() != &command.adapter
        || ack.received_at.as_str() < command.created_at.as_str()
    {
        bail!("Adapter ACK does not bind the exact immutable dispatch command and route");
    }
    Ok(())
}
