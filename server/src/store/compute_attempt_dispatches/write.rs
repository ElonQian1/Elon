use anyhow::{anyhow, bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{named_params, params, Connection};

use crate::compute_federation::attempt_gateway::ValidatedComputeAttemptStartDispatch;

use super::{
    read::{
        command_by_id_on, command_by_idempotency_on, command_receipt, ensure_command_replay_matches,
    },
    source::{current_source_blocker_on, ensure_broker_matches_command, ensure_command_live_at},
    types::ComputeAttemptDispatchCommandReceipt,
    validation::PreparedStartDispatch,
};
use crate::store::{
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

fn now_dispatch() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
