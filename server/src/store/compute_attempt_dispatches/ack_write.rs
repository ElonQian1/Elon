use anyhow::{anyhow, bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection};

use crate::{
    compute_federation::attempt_gateway::{
        ComputeAttemptAdapterAckEnvelope, VerifiedComputeAttemptAdapterAck,
        COMPUTE_ATTEMPT_ADAPTER_ACK_ACCEPTED, COMPUTE_ATTEMPT_ADAPTER_ACK_REJECTED,
    },
    store::{
        compute_attempt_activations::activate_compute_attempt_on,
        compute_attempt_start_outbox::record_verified_observation_on,
        ActivateComputeAttemptRequest, ComputeAttemptActivationReceipt,
    },
};

use super::{
    read::{
        ack_by_adapter_ack_on, ack_by_command_on, ack_receipt, application_by_command_on,
        command_by_id_on, ensure_ack_replay_matches, ensure_application_matches,
        ensure_remote_ack_replay_matches, StoredDispatchCommand,
    },
    replay::{ensure_activation_matches_command, replay_ack_commit},
    source::{ack_received_after_deadline, current_budget_blocker_on, current_source_blocker_on},
    types::ComputeAttemptDispatchAckCommit,
    validation::{
        application_id_for_ack, prepare_application, PreparedApplication, PreparedVerifiedAck,
    },
};

pub(super) fn ingest_verified_ack_on(
    connection: &Connection,
    verified: &VerifiedComputeAttemptAdapterAck,
    prepared: &PreparedVerifiedAck,
) -> Result<ComputeAttemptDispatchAckCommit> {
    let ack = verified.ack();
    ensure_v213_accepted_apply_available(ack)?;
    if let Some(stored) = ack_by_adapter_ack_on(
        connection,
        &verified.adapter().provider_id,
        &verified.adapter().adapter_id,
        &ack.adapter_ack_id,
    )? {
        ensure_remote_ack_replay_matches(&stored, verified, prepared)?;
        let command = command_by_id_on(connection, &stored.ack.command_id)?
            .ok_or_else(|| anyhow!("Stored Adapter ACK lost its immutable command"))?;
        ensure_ack_binds_command(&command, verified, prepared)?;
        record_verified_observation_on(connection, verified.prepare_observation())?;
        return replay_ack_commit(connection, &command, stored);
    }
    if ack_by_command_on(connection, &ack.command_id)?.is_some() {
        bail!("Attempt command already has a different immutable Adapter ACK");
    }
    let command = command_by_id_on(connection, &ack.command_id)?
        .ok_or_else(|| anyhow!("Adapter ACK references an unknown Attempt command"))?;
    ensure_ack_binds_command(&command, verified, prepared)?;
    let ingested_at = now_dispatch();
    if chrono::DateTime::parse_from_rfc3339(&ack.received_at)?
        > chrono::DateTime::parse_from_rfc3339(&ingested_at)?
    {
        bail!("Adapter ACK received_at cannot be later than durable ingestion");
    }
    record_verified_observation_on(connection, verified.prepare_observation())?;
    match ack.outcome.as_str() {
        COMPUTE_ATTEMPT_ADAPTER_ACK_REJECTED => {
            insert_ack_on(
                connection,
                &command,
                verified,
                prepared,
                "rejected",
                None,
                None,
                &ingested_at,
            )?;
            let stored = ack_by_command_on(connection, &ack.command_id)?
                .ok_or_else(|| anyhow!("Rejected Adapter ACK is not visible after insert"))?;
            ensure_ack_replay_matches(&stored, verified, prepared)?;
            Ok(ComputeAttemptDispatchAckCommit::Rejected {
                ack: ack_receipt(stored, false),
            })
        }
        COMPUTE_ATTEMPT_ADAPTER_ACK_ACCEPTED => {
            let remote_execution_ref = ack
                .remote_execution_ref
                .as_deref()
                .ok_or_else(|| anyhow!("Accepted Adapter ACK is missing its execution ref"))?;
            if ack_received_after_deadline(ack, &command.command.not_after, &ingested_at)? {
                return quarantine_ack_on(
                    connection,
                    &command,
                    verified,
                    prepared,
                    "COMMAND_EXPIRED",
                    &ingested_at,
                );
            }
            if let Some(reason) = current_source_blocker_on(
                connection,
                &command.command,
                &command.adapter,
                &command.activated_by_user_id,
                &command.activation_idempotency_key,
                true,
            )? {
                return quarantine_ack_on(
                    connection,
                    &command,
                    verified,
                    prepared,
                    reason,
                    &ingested_at,
                );
            }
            if let Some(reason) = current_budget_blocker_on(connection, &command, &ingested_at)? {
                return quarantine_ack_on(
                    connection,
                    &command,
                    verified,
                    prepared,
                    reason,
                    &ingested_at,
                );
            }
            insert_ack_on(
                connection,
                &command,
                verified,
                prepared,
                "accepted_applied",
                None,
                Some(&command.command.command.identity.attempt_lease_id),
                &ingested_at,
            )?;
            let activation = activate_compute_attempt_on(
                connection,
                &activation_request(&command, remote_execution_ref),
            )?;
            ensure_activation_matches_command(&command, ack, &activation)?;
            let prepared_application = prepare_application(ack, &activation)?;
            insert_application_on(connection, ack, &activation, &prepared_application)?;
            let stored_ack = ack_by_command_on(connection, &ack.command_id)?
                .ok_or_else(|| anyhow!("Accepted Adapter ACK is not visible after activation"))?;
            ensure_ack_replay_matches(&stored_ack, verified, prepared)?;
            if stored_ack.activation_lease_id.as_deref()
                != Some(command.command.command.identity.attempt_lease_id.as_str())
                || stored_ack.application_id.as_deref()
                    != Some(prepared_application.application_id.as_str())
            {
                bail!("Accepted Adapter ACK does not bind the activated lease");
            }
            let stored_application = application_by_command_on(connection, &ack.command_id)?
                .ok_or_else(|| anyhow!("Accepted Adapter ACK is missing its application"))?;
            ensure_application_matches(&stored_application, ack, &activation)?;
            Ok(ComputeAttemptDispatchAckCommit::Activated {
                ack: ack_receipt(stored_ack, false),
                application: stored_application.into_receipt(false),
                activation,
            })
        }
        _ => bail!("Unsupported Adapter ACK outcome"),
    }
}

fn ensure_v213_accepted_apply_available(ack: &ComputeAttemptAdapterAckEnvelope) -> Result<()> {
    if ack.outcome == COMPUTE_ATTEMPT_ADAPTER_ACK_ACCEPTED {
        bail!("COMPUTE_ATTEMPT_ACCEPTED_ACK_V213_ISSUER_UNAVAILABLE");
    }
    Ok(())
}

fn quarantine_ack_on(
    connection: &Connection,
    command: &StoredDispatchCommand,
    verified: &VerifiedComputeAttemptAdapterAck,
    prepared: &PreparedVerifiedAck,
    reason: &str,
    created_at: &str,
) -> Result<ComputeAttemptDispatchAckCommit> {
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

fn insert_ack_on(
    connection: &Connection,
    command: &StoredDispatchCommand,
    verified: &VerifiedComputeAttemptAdapterAck,
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

fn insert_application_on(
    connection: &Connection,
    ack: &ComputeAttemptAdapterAckEnvelope,
    activation: &ComputeAttemptActivationReceipt,
    prepared: &PreparedApplication,
) -> Result<()> {
    let created_at = application_created_at(&activation.activated_at)?;
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

fn application_created_at(activated_at: &str) -> Result<String> {
    let activated_at = chrono::DateTime::parse_from_rfc3339(activated_at)?.with_timezone(&Utc);
    Ok(std::cmp::max(activated_at, Utc::now()).to_rfc3339_opts(SecondsFormat::Nanos, true))
}

fn now_dispatch() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}

fn activation_request(
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

fn ensure_ack_binds_command(
    command: &StoredDispatchCommand,
    verified: &VerifiedComputeAttemptAdapterAck,
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
