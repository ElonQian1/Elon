use anyhow::{anyhow, bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{named_params, Connection};

use crate::compute_federation::execution_plan::{
    ComputeArtifactAccessEnvelope, ComputeArtifactAccessTarget, ComputeAttemptExecutionPlanEnvelope,
};

use super::{
    read::{access_collision_on, capability_collision_on, plan_by_id_on, plan_collision_on},
    replay::{exact_replay_receipt, prepare_plan},
    source::current_execution_sources_on,
    types::{
        ComputeAttemptExecutionPlanReceipt, PreparedArtifactAccess, PreparedCapability,
        PreparedInputs, PreparedPlan,
    },
    validation::{access_kind, access_target_identity},
};

pub(super) fn produce_on(
    connection: &Connection,
    candidate: &ComputeAttemptExecutionPlanEnvelope,
    inputs: &PreparedInputs,
) -> Result<ComputeAttemptExecutionPlanReceipt> {
    let recorded_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
    let authority_recorded_at = candidate.plan.planned_at.as_str();
    record_capability_on(connection, &inputs.capability, authority_recorded_at)?;
    for access in &inputs.accesses {
        record_access_on(connection, access, authority_recorded_at)?;
    }
    let sources = current_execution_sources_on(connection, candidate, &inputs.capability.envelope)?;
    let prepared = prepare_plan(candidate, inputs, &sources, &recorded_at)?;
    if let Some(stored) = plan_collision_on(connection, candidate)? {
        return exact_replay_receipt(stored, candidate);
    }
    insert_plan_on(connection, &prepared, &recorded_at)?;
    insert_plan_accesses_on(connection, &prepared)?;
    insert_seal_on(connection, &prepared, &recorded_at)?;
    let stored = plan_by_id_on(connection, &candidate.plan_id)?;
    if stored.plan != prepared.plan || stored.seal != prepared.seal {
        bail!("Execution plan exact readback differs from the committed candidate");
    }
    Ok(ComputeAttemptExecutionPlanReceipt::new(
        stored.plan,
        stored.seal,
        false,
    ))
}

fn record_capability_on(
    connection: &Connection,
    prepared: &PreparedCapability,
    recorded_at: &str,
) -> Result<()> {
    let envelope = &prepared.envelope;
    if let Some(stored) =
        capability_collision_on(connection, &envelope.capability_id, &prepared.digest)?
    {
        if stored.envelope != *envelope {
            bail!("Execution capability replay conflicts with its immutable receipt");
        }
        return Ok(());
    }
    let capability = &envelope.capability;
    let route = &capability.route;
    let provenance = &capability.provenance;
    connection.execute(
        "INSERT INTO compute_execution_capability_receipts (
            capability_id, capability_schema, capability_digest, capability_json,
            canonicalization, digest_algorithm, capability_kind, provider_id, provider_kind,
            executor_id, route_kind, route_binding_digest, endpoint_id, endpoint_transport,
            adapter_id, adapter_version, adapter_config_revision, adapter_config_digest,
            source_schema, source_id, source_digest, verification_kind, verifier_id,
            verification_digest, authenticated_at, observed_at, expires_at, recorded_at
         ) VALUES (
            :id, :schema, :digest, :json, :canonicalization, :algorithm, :kind, :provider,
            :provider_kind, :executor, :route_kind, :route_digest, :endpoint_id,
            :endpoint_transport, :adapter_id, :adapter_version, :config_revision,
            :config_digest, :source_schema, :source_id, :source_digest, :verification_kind,
            :verifier_id, :verification_digest, :authenticated_at, :observed_at,
            :expires_at, :recorded_at)",
        named_params! {
            ":id": envelope.capability_id,
            ":schema": envelope.schema,
            ":digest": prepared.digest,
            ":json": prepared.canonical_json,
            ":canonicalization": envelope.canonicalization,
            ":algorithm": envelope.digest_algorithm,
            ":kind": capability.capability_kind,
            ":provider": capability.provider_id,
            ":provider_kind": capability.provider_kind,
            ":executor": capability.executor_id,
            ":route_kind": route.route_kind,
            ":route_digest": route.route_binding_digest,
            ":endpoint_id": route.endpoint_id,
            ":endpoint_transport": route.endpoint_transport,
            ":adapter_id": route.adapter_id,
            ":adapter_version": route.adapter_version,
            ":config_revision": route.adapter_config_revision,
            ":config_digest": route.adapter_config_digest,
            ":source_schema": provenance.source_schema,
            ":source_id": provenance.source_id,
            ":source_digest": provenance.source_digest,
            ":verification_kind": provenance.verification_kind,
            ":verifier_id": provenance.verifier_id,
            ":verification_digest": provenance.verification_digest,
            ":authenticated_at": provenance.authenticated_at,
            ":observed_at": capability.observed_at,
            ":expires_at": capability.expires_at,
            ":recorded_at": recorded_at,
        },
    )?;
    let stored = capability_collision_on(connection, &envelope.capability_id, &prepared.digest)?
        .ok_or_else(|| anyhow!("Execution capability is not visible after insert"))?;
    if stored.envelope != *envelope {
        bail!("Execution capability exact readback failed");
    }
    Ok(())
}

fn record_access_on(
    connection: &Connection,
    prepared: &PreparedArtifactAccess,
    recorded_at: &str,
) -> Result<()> {
    let envelope = &prepared.envelope;
    if let Some(stored) = access_collision_on(connection, &envelope.access_id, &prepared.digest)? {
        if stored.envelope != *envelope {
            bail!("Artifact access replay conflicts with its immutable receipt");
        }
        return Ok(());
    }
    let access = &envelope.access;
    let (target_id, target_digest) = access_target_identity(envelope);
    let (media_type, size_limit) = access_media_and_size(envelope);
    connection.execute(
        "INSERT INTO compute_artifact_access_receipts (
            access_id, access_schema, access_digest, access_json, canonicalization,
            digest_algorithm, non_bearer_access_ref, authorization_digest, job_id,
            reservation_id, attempt_lease_id, provider_id, executor_id, fencing_generation,
            route_binding_digest, access_kind, target_id, target_digest, media_type,
            size_limit_bytes, issued_at, expires_at, recorded_at
         ) VALUES (
            :id, :schema, :digest, :json, :canonicalization, :algorithm, :access_ref,
            :authorization_digest, :job, :reservation, :lease, :provider, :executor,
            :fence, :route_digest, :kind, :target_id, :target_digest, :media_type,
            :size_limit, :issued_at, :expires_at, :recorded_at)",
        named_params! {
            ":id": envelope.access_id,
            ":schema": envelope.schema,
            ":digest": prepared.digest,
            ":json": prepared.canonical_json,
            ":canonicalization": envelope.canonicalization,
            ":algorithm": envelope.digest_algorithm,
            ":access_ref": access.non_bearer_access_ref,
            ":authorization_digest": access.authorization_digest,
            ":job": access.audience.job_id,
            ":reservation": access.audience.reservation_id,
            ":lease": access.audience.attempt_lease_id,
            ":provider": access.audience.provider_id,
            ":executor": access.audience.executor_id,
            ":fence": access.audience.fencing_generation,
            ":route_digest": access.audience.route_binding_digest,
            ":kind": access_kind(envelope),
            ":target_id": target_id,
            ":target_digest": target_digest,
            ":media_type": media_type,
            ":size_limit": size_limit,
            ":issued_at": access.issued_at,
            ":expires_at": access.expires_at,
            ":recorded_at": recorded_at,
        },
    )?;
    let stored = access_collision_on(connection, &envelope.access_id, &prepared.digest)?
        .ok_or_else(|| anyhow!("Artifact access is not visible after insert"))?;
    if stored.envelope != *envelope {
        bail!("Artifact access exact readback failed");
    }
    Ok(())
}

fn access_media_and_size(envelope: &ComputeArtifactAccessEnvelope) -> (&str, i64) {
    match &envelope.access.target {
        ComputeArtifactAccessTarget::Read(target) => (&target.media_type, target.size_bytes),
        ComputeArtifactAccessTarget::Write(target) => (&target.media_type, target.max_bytes),
    }
}

fn insert_plan_on(
    connection: &Connection,
    prepared: &PreparedPlan,
    recorded_at: &str,
) -> Result<()> {
    let envelope = &prepared.plan;
    let plan = &envelope.plan;
    let source = &plan.sources;
    connection.execute(
        "INSERT INTO compute_attempt_execution_plans (
            plan_id, plan_schema, plan_digest, plan_json, canonicalization, digest_algorithm,
            consumer_account_id, provider_id, provider_kind, provider_owner_account_id,
            provider_policy_revision, provider_digest, offer_id, offer_version, offer_digest,
            job_id, job_revision, job_digest, reservation_id, reservation_revision,
            reservation_digest, capacity_claim_id, claim_revision, claim_digest,
            price_snapshot_id, price_snapshot_digest, budget_reservation_id, budget_reserved_fen,
            broker_request_digest, attempt_lease_id, attempt_no, shard_id, fencing_generation,
            executor_id, route_binding_digest, capability_id, capability_digest, capability_kind,
            capability_expires_at, resource_grant_schema, resource_grant_json,
            resource_grant_id, resource_grant_digest, resource_grant_enforcement_kind,
            artifact_access_count, artifact_access_set_digest, lease_authority_kind,
            lease_delivery_mode, lease_audience, lease_authority_valid_until, planned_at,
            not_after, lease_expires_at, hard_deadline_at, recorded_at
         ) VALUES (
            :plan_id, :schema, :digest, :json, :canonicalization, :algorithm, :consumer,
            :provider, :provider_kind, :provider_owner, :provider_revision, :provider_digest,
            :offer, :offer_version, :offer_digest, :job, :job_revision, :job_digest,
            :reservation, :reservation_revision, :reservation_digest, :claim, :claim_revision,
            :claim_digest, :snapshot, :snapshot_digest, :budget, :budget_fen, :broker_digest,
            :lease, :attempt_no, :shard, :fence, :executor, :route_digest, :capability,
            :capability_digest, :capability_kind, :capability_expires, :grant_schema, :grant_json,
            :grant_id, :grant_digest, :grant_enforcement, :access_count, :access_set_digest,
            :authority_kind, :delivery_mode, :audience, :authority_until, :planned_at,
            :not_after, :lease_expires, :hard_deadline, :recorded_at)",
        named_params! {
            ":plan_id": envelope.plan_id, ":schema": envelope.schema,
            ":digest": prepared.plan_digest, ":json": prepared.plan_json,
            ":canonicalization": envelope.canonicalization, ":algorithm": envelope.digest_algorithm,
            ":consumer": source.consumer_account_id, ":provider": source.provider.provider_id,
            ":provider_kind": source.provider.provider_kind,
            ":provider_owner": source.provider.provider_owner_account_id,
            ":provider_revision": source.provider.policy_revision,
            ":provider_digest": source.provider.provider_digest, ":offer": source.offer.offer_id,
            ":offer_version": source.offer.offer_version, ":offer_digest": source.offer.offer_digest,
            ":job": source.job.job_id, ":job_revision": source.job.job_revision,
            ":job_digest": source.job.job_digest, ":reservation": source.reservation.reservation_id,
            ":reservation_revision": source.reservation.reservation_revision,
            ":reservation_digest": source.reservation.reservation_digest,
            ":claim": source.capacity_claim.claim_id,
            ":claim_revision": source.capacity_claim.claim_revision,
            ":claim_digest": source.capacity_claim.claim_digest,
            ":snapshot": source.price_snapshot.price_snapshot_id,
            ":snapshot_digest": source.price_snapshot.price_snapshot_digest,
            ":budget": source.budget.budget_reservation_id,
            ":budget_fen": source.budget.budget_reserved_fen,
            ":broker_digest": source.broker_request_digest,
            ":lease": plan.attempt.attempt_lease_id, ":attempt_no": plan.attempt.attempt_no,
            ":shard": plan.attempt.shard_id, ":fence": plan.attempt.fencing_generation,
            ":executor": plan.start.executor_id, ":route_digest": plan.route_binding_digest,
            ":capability": plan.capability.capability_id,
            ":capability_digest": plan.capability.capability_digest,
            ":capability_kind": plan.capability.capability_kind,
            ":capability_expires": plan.capability.expires_at,
            ":grant_schema": plan.resource_grant.schema,
            ":grant_json": prepared.resource_grant_json,
            ":grant_id": plan.resource_grant.grant_id,
            ":grant_digest": prepared.resource_grant_digest,
            ":grant_enforcement": plan.resource_grant.enforcement_kind,
            ":access_count": i64::try_from(plan.artifact_accesses.len())?,
            ":access_set_digest": prepared.access_set_digest,
            ":authority_kind": plan.lease_authority.authority_kind,
            ":delivery_mode": plan.lease_authority.delivery_mode,
            ":audience": plan.lease_authority.audience,
            ":authority_until": plan.lease_authority.valid_until,
            ":planned_at": plan.planned_at, ":not_after": plan.not_after,
            ":lease_expires": plan.start.lease_expires_at,
            ":hard_deadline": plan.start.hard_deadline_at, ":recorded_at": recorded_at,
        },
    )?;
    Ok(())
}

fn insert_plan_accesses_on(connection: &Connection, prepared: &PreparedPlan) -> Result<()> {
    for access in &prepared.plan.plan.artifact_accesses {
        connection.execute(
            "INSERT INTO compute_attempt_execution_plan_accesses (
                plan_id, plan_digest, ordinal, access_id, access_digest, access_kind,
                target_id, target_digest, expires_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                prepared.plan.plan_id,
                prepared.plan_digest,
                access.ordinal,
                access.access_id,
                access.access_digest,
                access.access_kind,
                access.target_id,
                access.target_digest,
                access.expires_at,
            ],
        )?;
    }
    Ok(())
}

fn insert_seal_on(
    connection: &Connection,
    prepared: &PreparedPlan,
    recorded_at: &str,
) -> Result<()> {
    let seal = &prepared.seal;
    connection.execute(
        "INSERT INTO compute_attempt_execution_plan_seals (
            seal_id, seal_schema, seal_digest, seal_json, canonicalization, digest_algorithm,
            plan_id, plan_digest, capability_digest, artifact_access_count,
            artifact_access_set_digest, resource_grant_digest, sealed_at, recorded_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        rusqlite::params![
            seal.seal_id,
            seal.schema,
            prepared.seal_digest,
            prepared.seal_json,
            seal.canonicalization,
            seal.digest_algorithm,
            seal.plan_id,
            seal.plan_digest,
            seal.capability_digest,
            seal.artifact_access_count,
            seal.artifact_access_set_digest,
            seal.resource_grant_digest,
            seal.sealed_at,
            recorded_at,
        ],
    )?;
    Ok(())
}
