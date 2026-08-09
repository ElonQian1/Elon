use anyhow::{anyhow, ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    compute_federation::{
        attempt_gateway::{
            canonical_adapter_ack_json_and_digest, canonical_adapter_binding_json_and_digest,
            canonical_dispatch_application_json_and_digest,
            canonical_dispatch_command_json_and_digest, ComputeAttemptAdapterAckEnvelope,
            ComputeAttemptAdapterBinding, ComputeAttemptDispatchApplicationEnvelope,
            ComputeAttemptDispatchCommandEnvelope, COMPUTE_ATTEMPT_ADAPTER_ACK_ACCEPTED,
            COMPUTE_ATTEMPT_DISPATCH_APPLICATION_ACTION_V185_ACTIVATE,
        },
        route_authority::{
            canonical_route_authorization_json_and_digest,
            canonical_service_actor_authorization_json_and_digest,
            ComputeRouteAuthorizationEnvelope, ComputeServiceActorAuthorizationEnvelope,
            COMPUTE_ACTOR_PHASE_APPLICATION,
        },
    },
    store::{
        compute_attempt_activations::compute_attempt_activation_on,
        compute_attempt_dispatches::PreparedApplication,
        compute_attempt_execution_plans::audited_plan_by_id_on,
    },
};

use super::{AcceptedApplicationFact, AcceptedCommitBase, AcceptedCommitSource};

pub(super) fn load_base_on(
    connection: &Connection,
    command_id: &str,
) -> Result<AcceptedCommitBase> {
    let row = connection
        .query_row(
            "SELECT command_json, command_digest, adapter_binding_json,
                    adapter_binding_digest, lease_credential_ref,
                    lease_credential_hint, activated_by_user_id
               FROM compute_attempt_dispatch_commands WHERE command_id=?1",
            params![command_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("accepted closure command is missing"))?;
    let command: ComputeAttemptDispatchCommandEnvelope = serde_json::from_str(&row.0)?;
    let adapter: ComputeAttemptAdapterBinding = serde_json::from_str(&row.2)?;
    let (command_json, command_digest) = canonical_dispatch_command_json_and_digest(&command)?;
    let (adapter_json, adapter_digest) = canonical_adapter_binding_json_and_digest(&adapter)?;
    ensure!(
        command_json == row.0
            && command.command_id == command_id
            && command.command_digest == row.1
            && command_digest == row.1
            && adapter_json == row.2
            && adapter_digest == row.3,
        "accepted closure command failed exact canonical audit"
    );
    let prepare = super::super::read::prepare_by_command_on(connection, command_id)?
        .ok_or_else(|| anyhow!("accepted closure prepare intent is missing"))?;
    let audited_plan = audited_plan_by_id_on(connection, &command.command.execution_plan.plan_id)?;
    ensure!(
        command.issued_at >= audited_plan.seal.sealed_at,
        "accepted closure command predates its sealed execution plan"
    );
    let plan = audited_plan.plan;
    let (route, actor_authority) = load_route_on(connection, &prepare)?;
    ensure_base_bindings(
        &command,
        &adapter,
        &row.6,
        &prepare,
        &plan,
        &route,
        &actor_authority,
    )?;
    Ok(AcceptedCommitBase {
        command,
        adapter,
        lease_credential_ref: row.4,
        lease_credential_hint: row.5,
        activated_by_user_id: row.6,
        prepare,
        plan,
        route,
        actor_authority,
    })
}

pub(super) fn load_source_for_persist_on(
    connection: &Connection,
    command_id: &str,
    application: &PreparedApplication,
) -> Result<AcceptedCommitSource> {
    let envelope = application.envelope().clone();
    let (json, digest) = canonical_dispatch_application_json_and_digest(&envelope)?;
    ensure!(
        json == application.application_json
            && digest == application.application_digest
            && envelope.application_digest == application.application_digest
            && envelope.application_id == application.application_id,
        "prepared accepted application failed exact canonical audit"
    );
    load_source_on(connection, command_id, AcceptedApplicationFact { envelope })
}

pub(super) fn load_source_for_replay_on(
    connection: &Connection,
    command_id: &str,
) -> Result<AcceptedCommitSource> {
    let row = connection
        .query_row(
            "SELECT application_json, application_digest, application_id, command_id,
                    ack_id, action, lease_id, activation_request_digest, lease_digest, applied_at
               FROM compute_attempt_dispatch_applications WHERE command_id=?1",
            params![command_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("accepted closure application is missing"))?;
    let envelope: ComputeAttemptDispatchApplicationEnvelope = serde_json::from_str(&row.0)?;
    let (json, digest) = canonical_dispatch_application_json_and_digest(&envelope)?;
    ensure!(
        json == row.0
            && envelope.application_digest == row.1
            && digest == row.1
            && envelope.application_id == row.2
            && envelope.command_id == row.3
            && envelope.ack_id == row.4
            && envelope.action == row.5
            && envelope.lease_id == row.6
            && envelope.activation_request_digest == row.7
            && envelope.lease_digest == row.8
            && envelope.applied_at == row.9,
        "accepted closure application failed exact projection audit"
    );
    load_source_on(connection, command_id, AcceptedApplicationFact { envelope })
}

fn load_source_on(
    connection: &Connection,
    command_id: &str,
    application: AcceptedApplicationFact,
) -> Result<AcceptedCommitSource> {
    let base = load_base_on(connection, command_id)?;
    let row = connection
        .query_row(
            "SELECT ack_json, ack_digest, disposition, activation_lease_id,
                    application_id, provider_id, adapter_id
               FROM compute_attempt_dispatch_acks WHERE command_id=?1",
            params![command_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("accepted closure ACK is missing"))?;
    let ack: ComputeAttemptAdapterAckEnvelope = serde_json::from_str(&row.0)?;
    let (ack_json, ack_digest) = canonical_adapter_ack_json_and_digest(&ack)?;
    ensure!(
        ack_json == row.0
            && ack.ack_digest == row.1
            && ack_digest == row.1
            && ack.outcome == COMPUTE_ATTEMPT_ADAPTER_ACK_ACCEPTED
            && row.2 == "accepted_applied"
            && row.3.as_deref() == Some(base.command.command.identity.attempt_lease_id.as_str())
            && row.4.as_deref() == Some(application.envelope.application_id.as_str())
            && row.5 == base.adapter.provider_id
            && row.6 == base.adapter.adapter_id,
        "accepted closure ACK failed exact applied audit"
    );
    let activation =
        compute_attempt_activation_on(connection, &base.command.command.identity.attempt_lease_id)?;
    ensure_source_bindings(&base, &ack, &activation, &application.envelope)?;
    Ok(AcceptedCommitSource {
        base,
        ack,
        activation,
        application,
    })
}

fn load_route_on(
    connection: &Connection,
    prepare: &super::super::types::StoredStartOutboxOperation,
) -> Result<(
    ComputeRouteAuthorizationEnvelope,
    ComputeServiceActorAuthorizationEnvelope,
)> {
    let row = connection
        .query_row(
            "SELECT route.route_authorization_json, route.route_authorization_digest,
                    actor.actor_authorization_json, actor.actor_authorization_digest
               FROM compute_route_authorization_receipts route
               JOIN compute_service_actor_authorizations actor
                 ON actor.actor_authorization_id=route.actor_authorization_id
                AND actor.actor_authorization_digest=route.actor_authorization_digest
              WHERE route.route_authorization_id=?1
                AND route.route_authorization_digest=?2",
            params![
                prepare.envelope.route_authorization_id,
                prepare.envelope.route_authorization_digest
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("accepted closure route authority is missing"))?;
    let route: ComputeRouteAuthorizationEnvelope = serde_json::from_str(&row.0)?;
    let actor: ComputeServiceActorAuthorizationEnvelope = serde_json::from_str(&row.2)?;
    let (route_json, route_digest) = canonical_route_authorization_json_and_digest(&route)?;
    let (actor_json, actor_digest) = canonical_service_actor_authorization_json_and_digest(&actor)?;
    ensure!(
        route_json == row.0
            && route.route_authorization_digest == row.1
            && route_digest == row.1
            && actor_json == row.2
            && actor.actor_authorization_digest == row.3
            && actor_digest == row.3
            && super::super::read::route_capabilities_exact_on(
                connection,
                &route.route_authorization_id,
                &row.0,
            )?,
        "accepted closure route authority failed exact canonical audit"
    );
    super::route_audit::audit_immutable_on(connection, &route)?;
    Ok((route, actor))
}

#[allow(clippy::too_many_arguments)]
fn ensure_base_bindings(
    command: &ComputeAttemptDispatchCommandEnvelope,
    adapter: &ComputeAttemptAdapterBinding,
    activated_by_user_id: &str,
    prepare: &super::super::types::StoredStartOutboxOperation,
    plan: &crate::compute_federation::execution_plan::ComputeAttemptExecutionPlanEnvelope,
    route: &ComputeRouteAuthorizationEnvelope,
    actor: &ComputeServiceActorAuthorizationEnvelope,
) -> Result<()> {
    let start = &command.command;
    let plan_body = &plan.plan;
    let route_body = &route.authorization;
    let actor_body = &actor.authorization;
    let (_, adapter_digest) = canonical_adapter_binding_json_and_digest(adapter)?;
    ensure!(
        prepare.envelope.operation_kind == "prepare"
            && prepare.envelope.operation_generation == 1
            && prepare.envelope.subject_outbox_id.is_none()
            && prepare.projection.state == "delivery_observed"
            && prepare.projection.attempt_count >= 1
            && prepare.envelope.command_id == command.command_id
            && prepare.envelope.command_digest == command.command_digest
            && prepare.provider_id == adapter.provider_id
            && prepare.adapter_id == adapter.adapter_id
            && prepare.envelope.adapter_binding_digest == adapter_digest
            && prepare.envelope.adapter_binding_digest == route_body.route.adapter_binding_digest
            && prepare.envelope.plan_id == plan.plan_id
            && prepare.envelope.plan_digest == plan.plan_digest
            && prepare.envelope.lease_id == start.identity.attempt_lease_id
            && prepare.envelope.fencing_generation == start.identity.fencing_generation
            && prepare.envelope.route_authorization_id == route.route_authorization_id
            && prepare.envelope.route_authorization_digest == route.route_authorization_digest
            && plan.plan_id == start.execution_plan.plan_id
            && plan.schema == start.execution_plan.plan_schema
            && plan.plan_digest == start.execution_plan.plan_digest
            && command.not_after == plan_body.not_after
            && command.issued_at >= plan_body.planned_at
            && plan_body.sources.provider.provider_owner_account_id == activated_by_user_id
            && plan_body.sources.provider.provider_id == start.provider.provider_id
            && plan_body.sources.provider.policy_revision == start.provider.policy_revision
            && plan_body.sources.provider.provider_digest == start.provider.provider_digest
            && plan_body.sources.offer == start.offer
            && plan_body.sources.job == start.job
            && plan_body.sources.reservation.reservation_id == start.reservation.reservation_id
            && plan_body.sources.reservation.reservation_revision
                == start.reservation.reservation_revision
            && plan_body.sources.reservation.reservation_digest
                == start.reservation.reservation_digest
            && plan_body.sources.capacity_claim == start.capacity_claim
            && plan_body.attempt.job_id == start.identity.job_id
            && plan_body.attempt.reservation_id == start.identity.reservation_id
            && plan_body.attempt.attempt_lease_id == start.identity.attempt_lease_id
            && plan_body.attempt.attempt_no == start.identity.attempt_no
            && plan_body.attempt.shard_id == start.identity.shard_id
            && plan_body.attempt.fencing_generation == start.identity.fencing_generation
            && plan_body.start.executor_id == start.executor_id
            && plan_body.start.lease_expires_at == start.lease_expires_at
            && plan_body.start.hard_deadline_at == start.hard_deadline_at
            && plan_body.lease_authority.attempt_lease_id == start.identity.attempt_lease_id
            && plan_body.lease_authority.fencing_generation == start.identity.fencing_generation
            && plan_body.route_binding_digest == route_body.route.route_binding_digest
            && route_body.provider.provider_id == start.provider.provider_id
            && route_body.provider.provider_id == adapter.provider_id
            && route_body.provider.provider_kind == adapter.provider_kind
            && route_body.provider.provider_owner_account_id == activated_by_user_id
            && route_body.executor_id == start.executor_id
            && route_body.route.route_kind == adapter.route_kind
            && route_body.route.endpoint_id == adapter.endpoint_id
            && route_body.route.endpoint_transport == adapter.endpoint_transport
            && route_body.route.adapter.adapter_id == adapter.adapter_id
            && route_body.route.adapter.adapter_release_version == adapter.adapter_version
            && route_body.route.adapter.config_revision == adapter.config_revision
            && route_body.route.adapter.config_digest == adapter.config_digest
            && route_body.verified_by_service_actor_id == actor_body.service_actor_id
            && route_body.actor_authorization_id == actor.actor_authorization_id
            && route_body.actor_authorization_digest == actor.actor_authorization_digest
            && actor_body.provider_id == route_body.provider.provider_id
            && actor_body.provider_owner_account_id == activated_by_user_id
            && actor_body.issued_by_user_id == activated_by_user_id
            && actor_body.service_actor_id != activated_by_user_id
            && actor_body
                .allowed_route_kinds
                .iter()
                .any(|kind| kind == &route_body.route.route_kind)
            && actor_body
                .allowed_actor_phases
                .iter()
                .any(|phase| phase == COMPUTE_ACTOR_PHASE_APPLICATION),
        "accepted closure command, plan, and route binding failed exact audit"
    );
    Ok(())
}

fn ensure_source_bindings(
    base: &AcceptedCommitBase,
    ack: &ComputeAttemptAdapterAckEnvelope,
    activation: &crate::store::ComputeAttemptActivationReceipt,
    application: &ComputeAttemptDispatchApplicationEnvelope,
) -> Result<()> {
    let start = &base.command.command;
    let plan_authority = &base.plan.plan.lease_authority;
    ensure!(
        ack.command_id == base.command.command_id
            && ack.command_digest == base.command.command_digest
            && ack.adapter_binding_digest == base.prepare.envelope.adapter_binding_digest
            && application.application_id
                == format!("attempt_dispatch_application_{}", ack.ack_digest)
            && application.command_id == base.command.command_id
            && application.ack_id == ack.ack_id
            && application.action == COMPUTE_ATTEMPT_DISPATCH_APPLICATION_ACTION_V185_ACTIVATE
            && application.lease_id == activation.lease.lease_id
            && application.activation_request_digest == activation.request_digest
            && application.lease_digest == activation.lease_digest
            && application.applied_at == activation.activated_at
            && activation.executor_acceptance_ref
                == ack.remote_execution_ref.as_deref().unwrap_or_default()
            && activation.lease.lease_id == start.identity.attempt_lease_id
            && activation.lease.provider_id == start.provider.provider_id
            && activation.lease.executor_id == start.executor_id
            && activation.lease.fencing_generation == start.identity.fencing_generation
            && activation.lease.lease_credential_ref == base.lease_credential_ref
            && activation.lease.lease_credential_hint == base.lease_credential_hint
            && activation.activated_by_user_id == base.activated_by_user_id
            && plan_authority.attempt_lease_id == activation.lease.lease_id
            && plan_authority.fencing_generation == activation.lease.fencing_generation
            && start.hard_deadline_at <= plan_authority.valid_until,
        "accepted closure ACK, activation, application, or plan authority mismatch"
    );
    Ok(())
}
