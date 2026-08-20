use anyhow::{anyhow, ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::start_outbox::{
    canonical_lease_authority_binding_json_and_digest, canonical_lease_authority_scopes_digest,
    ComputeLeaseAuthorityBindingEnvelope,
};

use super::super::types::StoredStartOutboxOperation;

pub(super) fn ensure_commit_source_current_on(
    connection: &Connection,
    stored: &StoredStartOutboxOperation,
    checked_at: &str,
) -> Result<()> {
    let authority_row = connection
        .query_row(
            "SELECT authority.authority_json, plan.lease_authority_kind,
                    plan.lease_delivery_mode, plan.lease_audience,
                    json_extract(plan.plan_json,
                        '$.plan.lease_authority.required_scopes'),
                    plan.lease_authority_valid_until
               FROM compute_attempt_lease_authority_bindings authority
               JOIN compute_attempt_execution_plans plan ON plan.plan_id=authority.plan_id
               JOIN compute_attempt_dispatch_commands command
                 ON command.command_id=authority.command_id
                AND command.command_digest=authority.command_digest
               JOIN compute_attempt_dispatch_acks ack ON ack.ack_id=authority.ack_id
               JOIN compute_attempt_dispatch_applications application
                 ON application.application_id=authority.application_id
               JOIN compute_attempt_dispatch_actor_receipts actor
                 ON actor.actor_receipt_id=authority.application_actor_receipt_id
                AND actor.actor_receipt_digest=authority.application_actor_receipt_digest
               JOIN compute_attempt_activations activation ON activation.lease_id=authority.lease_id
               JOIN compute_attempt_lease_states lease ON lease.lease_id=authority.lease_id
               JOIN compute_jobs job ON job.job_id=activation.job_id
               JOIN compute_reservations reservation
                 ON reservation.reservation_id=activation.reservation_id
               JOIN compute_capacity_claims claim ON claim.claim_id=activation.capacity_claim_id
               JOIN billing_reservations budget ON budget.id=activation.budget_reservation_id
              WHERE authority.lease_authority_id=?1 AND authority.authority_revision=?2
                AND authority.lease_authority_digest=?3
                AND authority.command_id=?4 AND authority.command_digest=?5
                AND authority.plan_id=?6 AND authority.plan_digest=?7
                AND authority.application_id=?8 AND authority.application_digest=?9
                AND authority.lease_id=?10 AND authority.fencing_generation=?11
                AND authority.route_authorization_id=?12
                AND authority.route_authorization_digest=?13
                AND command.execution_plan_id=authority.plan_id
                AND command.execution_plan_digest=authority.plan_digest
                AND command.provider_id=authority.provider_id
                AND command.executor_id=authority.executor_id
                AND command.lease_id=authority.lease_id
                AND command.fencing_generation=authority.fencing_generation
                AND command.lease_credential_ref=authority.non_bearer_authority_ref
                AND command.lease_credential_hint=authority.authority_hint
                AND ack.command_id=authority.command_id
                AND ack.ack_digest=authority.ack_digest
                AND ack.outcome='accepted' AND ack.disposition='accepted_applied'
                AND ack.activation_lease_id=authority.lease_id
                AND ack.application_id=authority.application_id
                AND application.command_id=authority.command_id
                AND application.ack_id=authority.ack_id
                AND application.application_digest=authority.application_digest
                AND application.action='v185_activate'
                AND application.lease_id=authority.lease_id
                AND application.lease_digest=authority.lease_digest
                AND actor.actor_phase='application'
                AND actor.command_id=authority.command_id
                AND actor.command_digest=authority.command_digest
                AND actor.provider_id=authority.provider_id
                AND actor.route_authorization_id=authority.route_authorization_id
                AND actor.route_authorization_digest=authority.route_authorization_digest
                AND actor.ack_id=authority.ack_id
                AND actor.ack_digest=authority.ack_digest
                AND actor.application_id=authority.application_id
                AND actor.application_digest=authority.application_digest
                AND authority.authority_kind=plan.lease_authority_kind
                AND authority.delivery_mode=plan.lease_delivery_mode
                AND authority.audience=plan.lease_audience
                AND authority.scopes_json=json_extract(plan.plan_json,
                    '$.plan.lease_authority.required_scopes')
                AND authority.expires_at=plan.lease_authority_valid_until
                AND activation.lease_digest=authority.lease_digest
                AND activation.fencing_generation=authority.fencing_generation
                AND lease.lease_digest=authority.lease_digest
                AND lease.fencing_generation=authority.fencing_generation
                AND lease.status='staging'
                AND job.current_revision=activation.running_job_revision
                AND job.current_job_digest=activation.running_job_digest
                AND job.status='running'
                AND reservation.current_revision=activation.active_reservation_revision
                AND reservation.current_reservation_digest=activation.active_reservation_digest
                AND reservation.status='active'
                AND claim.revision=activation.active_claim_revision
                AND claim.claim_digest=activation.active_claim_digest
                AND claim.status='active'
                AND budget.reserved_fen=activation.budget_reserved_fen
                AND budget.status='reserved'
                AND ack.received_at<=authority.issued_at
                AND command.hard_deadline_at<=authority.expires_at
                AND ?14<authority.expires_at",
            params![
                stored.envelope.lease_authority_id,
                stored.envelope.lease_authority_revision,
                stored.envelope.lease_authority_digest,
                stored.envelope.command_id,
                stored.envelope.command_digest,
                stored.envelope.plan_id,
                stored.envelope.plan_digest,
                stored.envelope.application_id,
                stored.envelope.application_digest,
                stored.envelope.lease_id,
                stored.envelope.fencing_generation,
                stored.envelope.route_authorization_id,
                stored.envelope.route_authorization_digest,
                checked_at,
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("commit send lacks exact live application lease authority"))?;
    let (authority_json, plan_kind, plan_mode, plan_audience, plan_scopes_json, plan_valid_until) =
        authority_row;
    let authority: ComputeLeaseAuthorityBindingEnvelope = serde_json::from_str(&authority_json)?;
    let (canonical, digest) = canonical_lease_authority_binding_json_and_digest(&authority)?;
    ensure!(
        canonical == authority_json
            && digest == authority.lease_authority_digest
            && canonical_lease_authority_scopes_digest(&authority.scopes)?
                == authority.scopes_digest
            && authority.lease_authority_id
                == stored
                    .envelope
                    .lease_authority_id
                    .as_deref()
                    .unwrap_or_default()
            && Some(authority.authority_revision) == stored.envelope.lease_authority_revision
            && authority.command_id == stored.envelope.command_id
            && authority.command_digest == stored.envelope.command_digest
            && authority.plan_id == stored.envelope.plan_id
            && authority.plan_digest == stored.envelope.plan_digest
            && Some(authority.ack_id.as_str()) == stored.envelope.ack_id.as_deref()
            && Some(authority.ack_digest.as_str()) == stored.envelope.ack_digest.as_deref()
            && Some(authority.application_id.as_str()) == stored.envelope.application_id.as_deref()
            && Some(authority.application_digest.as_str())
                == stored.envelope.application_digest.as_deref()
            && authority.application_actor_receipt_id == stored.envelope.actor_receipt_id
            && authority.application_actor_receipt_digest == stored.envelope.actor_receipt_digest
            && authority.lease_id == stored.envelope.lease_id
            && authority.fencing_generation == stored.envelope.fencing_generation
            && authority.route_authorization_id == stored.envelope.route_authorization_id
            && authority.route_authorization_digest == stored.envelope.route_authorization_digest
            && authority.authority_kind == plan_kind
            && authority.delivery_mode == plan_mode
            && authority.audience == plan_audience
            && serde_json::to_string(&authority.scopes)? == plan_scopes_json
            && authority.expires_at == plan_valid_until
            && !authority.non_bearer_authority_ref.trim().is_empty(),
        "commit lease authority failed canonical audit"
    );
    Ok(())
}
