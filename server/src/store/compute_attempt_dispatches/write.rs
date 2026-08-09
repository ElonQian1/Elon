use anyhow::{anyhow, bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{named_params, params, Connection};

use crate::{
    compute_federation::attempt_gateway::{
        ValidatedComputeAttemptStartDispatch, VerifiedComputeAttemptAdapterAck,
        COMPUTE_ATTEMPT_ADAPTER_ACK_ACCEPTED, COMPUTE_ATTEMPT_ADAPTER_ACK_REJECTED,
    },
    store::ActivateComputeAttemptRequest,
};

use super::{
    read::{
        ack_by_adapter_ack_on, ack_by_command_on, ack_receipt, application_by_command_on,
        command_by_id_on, command_by_idempotency_on, command_receipt, ensure_ack_replay_matches,
        ensure_application_matches, ensure_command_replay_matches,
        ensure_remote_ack_replay_matches, StoredDispatchCommand,
    },
    replay::{ensure_activation_matches_command, replay_ack_commit},
    source::{
        ack_received_after_deadline, current_budget_blocker_on, current_source_blocker_on,
        ensure_broker_matches_command, ensure_command_live_at,
    },
    types::{ComputeAttemptDispatchAckCommit, ComputeAttemptDispatchCommandReceipt},
    validation::{
        application_id_for_ack, prepare_application, PreparedApplication, PreparedStartDispatch,
        PreparedVerifiedAck,
    },
};
use crate::store::{
    compute_attempt_activations::activate_compute_attempt_on,
    compute_attempt_execution_plans::ensure_current_plan_for_dispatch_on,
    compute_broker_reservation::broker_reserve_binding_on,
    compute_job_registry::current_registered_job_on,
};

pub(super) fn prepare_start_dispatch_on(
    connection: &Connection,
    plan: &ValidatedComputeAttemptStartDispatch,
    prepared: &PreparedStartDispatch,
) -> Result<ComputeAttemptDispatchCommandReceipt> {
    if let Some(stored) = command_by_id_on(connection, &plan.command().command_id)? {
        ensure_command_replay_matches(&stored, plan, prepared)?;
        return Ok(command_receipt(stored, true));
    }
    if let Some(stored) = command_by_idempotency_on(
        connection,
        &plan.command().command.provider.provider_id,
        plan.activation().idempotency_key(),
    )? {
        ensure_command_replay_matches(&stored, plan, prepared)?;
        return Ok(command_receipt(stored, true));
    }
    ensure_current_plan_for_dispatch_on(
        connection,
        plan.command(),
        plan.adapter(),
        plan.activation().activated_by_user_id(),
    )?;
    let source = plan.command();
    if let Some(reason) = current_source_blocker_on(
        connection,
        source,
        plan.adapter(),
        plan.activation().activated_by_user_id(),
        plan.activation().idempotency_key(),
        true,
    )? {
        bail!("Attempt Start dispatch source is not current: {reason}");
    }
    let broker = broker_reserve_binding_on(
        connection,
        &source.command.identity.reservation_id,
        &current_registered_job_on(connection, &source.command.identity.job_id)?
            .ok_or_else(|| anyhow!("Attempt dispatch Job disappeared during preparation"))?
            .job
            .consumer_account_id,
    )?;
    ensure_broker_matches_command(&broker, source)?;
    let broker_request_digest: String = connection.query_row(
        "SELECT request_digest FROM compute_broker_reserve_receipts
          WHERE reservation_id=?1 AND budget_reservation_id=?2",
        params![
            source.command.identity.reservation_id,
            broker.budget_reservation_id
        ],
        |row| row.get(0),
    )?;
    let created_at = now_dispatch();
    ensure_command_live_at(source, &created_at)?;
    connection.execute(
        "INSERT INTO compute_attempt_dispatch_commands (
            command_id, command_schema, command_type, command_digest, command_json,
            adapter_binding_digest, adapter_binding_json,
            provider_id, provider_kind, route_kind, provider_policy_revision, provider_digest,
            endpoint_id, endpoint_transport, adapter_id, adapter_version,
            adapter_config_revision, adapter_config_digest,
            job_id, job_revision, job_digest,
            reservation_id, reservation_revision, reservation_digest,
            capacity_claim_id, claim_revision, claim_digest,
            budget_reservation_id, budget_reserved_fen, broker_request_digest,
            offer_id, offer_version, offer_digest,
            lease_id, executor_id, attempt_no, shard_id, fencing_generation,
            execution_plan_id, execution_plan_schema, execution_plan_digest,
            lease_credential_ref, lease_credential_hint,
            activation_idempotency_key, activated_by_user_id,
            lease_expires_at, hard_deadline_at, issued_at, not_after, created_at
         ) VALUES (
            :command_id, :command_schema, :command_type, :command_digest, :command_json,
            :adapter_binding_digest, :adapter_binding_json,
            :provider_id, :provider_kind, :route_kind, :provider_policy_revision, :provider_digest,
            :endpoint_id, :endpoint_transport, :adapter_id, :adapter_version,
            :adapter_config_revision, :adapter_config_digest,
            :job_id, :job_revision, :job_digest,
            :reservation_id, :reservation_revision, :reservation_digest,
            :capacity_claim_id, :claim_revision, :claim_digest,
            :budget_reservation_id, :budget_reserved_fen, :broker_request_digest,
            :offer_id, :offer_version, :offer_digest,
            :lease_id, :executor_id, :attempt_no, :shard_id, :fencing_generation,
            :execution_plan_id, :execution_plan_schema, :execution_plan_digest,
            :lease_credential_ref, :lease_credential_hint,
            :activation_idempotency_key, :activated_by_user_id,
            :lease_expires_at, :hard_deadline_at, :issued_at, :not_after, :created_at
         )",
        named_params! {
            ":command_id": source.command_id,
            ":command_schema": source.schema,
            ":command_type": source.command.command_type,
            ":command_digest": prepared.command_digest,
            ":command_json": prepared.command_json,
            ":adapter_binding_digest": prepared.adapter_digest,
            ":adapter_binding_json": prepared.adapter_json,
            ":provider_id": source.command.provider.provider_id,
            ":provider_kind": plan.adapter().provider_kind,
            ":route_kind": plan.adapter().route_kind,
            ":provider_policy_revision": source.command.provider.policy_revision,
            ":provider_digest": source.command.provider.provider_digest,
            ":endpoint_id": plan.adapter().endpoint_id,
            ":endpoint_transport": plan.adapter().endpoint_transport,
            ":adapter_id": plan.adapter().adapter_id,
            ":adapter_version": plan.adapter().adapter_version,
            ":adapter_config_revision": plan.adapter().config_revision,
            ":adapter_config_digest": plan.adapter().config_digest,
            ":job_id": source.command.job.job_id,
            ":job_revision": source.command.job.job_revision,
            ":job_digest": source.command.job.job_digest,
            ":reservation_id": source.command.reservation.reservation_id,
            ":reservation_revision": source.command.reservation.reservation_revision,
            ":reservation_digest": source.command.reservation.reservation_digest,
            ":capacity_claim_id": source.command.capacity_claim.claim_id,
            ":claim_revision": source.command.capacity_claim.claim_revision,
            ":claim_digest": source.command.capacity_claim.claim_digest,
            ":budget_reservation_id": broker.budget_reservation_id,
            ":budget_reserved_fen": broker.budget_reserved_fen,
            ":broker_request_digest": broker_request_digest,
            ":offer_id": source.command.offer.offer_id,
            ":offer_version": source.command.offer.offer_version,
            ":offer_digest": source.command.offer.offer_digest,
            ":lease_id": source.command.identity.attempt_lease_id,
            ":executor_id": source.command.executor_id,
            ":attempt_no": source.command.identity.attempt_no,
            ":shard_id": source.command.identity.shard_id,
            ":fencing_generation": source.command.identity.fencing_generation,
            ":execution_plan_id": source.command.execution_plan.plan_id,
            ":execution_plan_schema": source.command.execution_plan.plan_schema,
            ":execution_plan_digest": source.command.execution_plan.plan_digest,
            ":lease_credential_ref": plan.activation().lease_credential_ref(),
            ":lease_credential_hint": plan.activation().lease_credential_hint(),
            ":activation_idempotency_key": plan.activation().idempotency_key(),
            ":activated_by_user_id": plan.activation().activated_by_user_id(),
            ":lease_expires_at": source.command.lease_expires_at,
            ":hard_deadline_at": source.command.hard_deadline_at,
            ":issued_at": source.issued_at,
            ":not_after": source.not_after,
            ":created_at": created_at,
        },
    )?;
    let stored = command_by_id_on(connection, &source.command_id)?
        .ok_or_else(|| anyhow!("Attempt dispatch command is not visible after insert"))?;
    ensure_command_replay_matches(&stored, plan, prepared)?;
    Ok(command_receipt(stored, false))
}

pub(super) fn ingest_verified_ack_on(
    connection: &Connection,
    verified: &VerifiedComputeAttemptAdapterAck,
    prepared: &PreparedVerifiedAck,
) -> Result<ComputeAttemptDispatchAckCommit> {
    let ack = verified.ack();
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
    ack: &crate::compute_federation::attempt_gateway::ComputeAttemptAdapterAckEnvelope,
    activation: &crate::store::ComputeAttemptActivationReceipt,
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
