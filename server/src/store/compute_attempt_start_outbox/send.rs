use anyhow::{anyhow, ensure, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    compute_federation::start_outbox::{
        canonical_lease_authority_binding_json_and_digest, canonical_lease_authority_scopes_digest,
        canonical_start_outbox_send_attempt_json_and_digest, ComputeLeaseAuthorityBindingEnvelope,
        ComputeStartOutboxSendAttemptEnvelope, COMPUTE_OUTBOX_STATE_CLAIMED,
        COMPUTE_OUTBOX_STATE_IN_FLIGHT_UNKNOWN, COMPUTE_START_OUTBOX_CANONICALIZATION,
        COMPUTE_START_OUTBOX_DIGEST_ALGORITHM, COMPUTE_START_OUTBOX_SEND_ATTEMPT_SCHEMA,
    },
    store::{hash_token, new_id},
};

use super::{
    currentness::ensure_send_current_on,
    read::outbox_by_id_on,
    types::{PreparedStartSendRequest, StartOutboxClaimHandle, StoredStartOutboxOperation},
};

pub(super) fn record_send_started_on(
    connection: &Connection,
    claim: &StartOutboxClaimHandle,
    request: &PreparedStartSendRequest,
) -> Result<ComputeStartOutboxSendAttemptEnvelope> {
    ensure_digest(&request.request_digest, "sealed Start request digest")?;
    let started_at = now_nanos();
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
            && started_at.as_str() > stored.projection.updated_at.as_str()
            && started_at.as_str() < claim.receipt.claim_expires_at.as_str(),
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
    ensure_send_current_on(connection, &stored, &started_at)?;
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
        started_at: started_at.clone(),
    };
    let (_, digest) = canonical_start_outbox_send_attempt_json_and_digest(&envelope)?;
    envelope.send_attempt_digest = digest;
    let (json, recomputed) = canonical_start_outbox_send_attempt_json_and_digest(&envelope)?;
    ensure!(
        recomputed == envelope.send_attempt_digest,
        "Start send-attempt failed canonical audit"
    );
    connection.execute(
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
            json,
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
    let changed = connection.execute(
        "UPDATE compute_attempt_start_outbox
            SET state='in_flight_unknown', state_revision=state_revision+1,
                attempt_count=attempt_count+1, updated_at=?1
          WHERE outbox_id=?2 AND state='claimed' AND state_revision=?3
            AND attempt_count=?4 AND claim_owner_id=?5 AND claim_token_digest=?6
            AND claim_generation=?7 AND claim_expires_at=?8",
        params![
            started_at,
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
    Ok(envelope)
}

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

fn now_nanos() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
