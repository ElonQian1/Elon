use anyhow::{anyhow, bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::{
    attempt_gateway::{
        canonical_adapter_binding_json_and_digest, ComputeAttemptAdapterBinding,
        ComputeAttemptDispatchCommandEnvelope,
    },
    execution_plan::{
        canonical_artifact_access_json_and_digest, canonical_execution_capability_json_and_digest,
        canonical_execution_plan_json_and_digest, canonical_execution_plan_seal_json_and_digest,
        canonical_plan_access_set_digest, canonical_resource_grant_json_and_digest,
        ComputeArtifactAccessEnvelope, ComputeAttemptExecutionPlanEnvelope,
        ComputeAttemptExecutionPlanSealEnvelope, ComputeExecutionCapabilityEnvelope,
    },
};

use super::{
    replay_validation::source_time,
    source::current_execution_sources_on,
    validation::{access_kind, access_target_identity, validate_canonical_timestamp},
};

pub(super) struct StoredCapability {
    pub envelope: ComputeExecutionCapabilityEnvelope,
}

pub(super) struct StoredAccess {
    pub envelope: ComputeArtifactAccessEnvelope,
}

pub(super) struct StoredPlan {
    pub plan: ComputeAttemptExecutionPlanEnvelope,
    pub seal: ComputeAttemptExecutionPlanSealEnvelope,
    pub capability: ComputeExecutionCapabilityEnvelope,
}

pub(super) fn capability_collision_on(
    connection: &Connection,
    id: &str,
    digest: &str,
) -> Result<Option<StoredCapability>> {
    let mut statement = connection.prepare(
        "SELECT capability_json, capability_digest FROM compute_execution_capability_receipts
          WHERE capability_id=?1 OR capability_digest=?2 ORDER BY capability_id",
    )?;
    let rows = statement
        .query_map(params![id, digest], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if rows.len() > 1 {
        bail!("Execution capability identity and digest collide with different receipts");
    }
    rows.into_iter()
        .next()
        .map(|(json, stored_digest)| audit_capability(&json, &stored_digest))
        .transpose()
}

pub(super) fn access_collision_on(
    connection: &Connection,
    id: &str,
    digest: &str,
) -> Result<Option<StoredAccess>> {
    let mut statement = connection.prepare(
        "SELECT access_json, access_digest FROM compute_artifact_access_receipts
          WHERE access_id=?1 OR access_digest=?2 ORDER BY access_id",
    )?;
    let rows = statement
        .query_map(params![id, digest], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if rows.len() > 1 {
        bail!("Artifact access identity and digest collide with different receipts");
    }
    rows.into_iter()
        .next()
        .map(|(json, stored_digest)| audit_access(&json, &stored_digest))
        .transpose()
}

pub(super) fn plan_collision_on(
    connection: &Connection,
    candidate: &ComputeAttemptExecutionPlanEnvelope,
) -> Result<Option<StoredPlan>> {
    let plan = &candidate.plan;
    let mut statement = connection.prepare(
        "SELECT plan_id FROM compute_attempt_execution_plans
          WHERE plan_id=?1 OR plan_digest=?2 OR attempt_lease_id=?3
             OR (job_id=?4 AND attempt_no=?5) ORDER BY plan_id",
    )?;
    let ids = statement
        .query_map(
            params![
                candidate.plan_id,
                candidate.plan_digest,
                plan.attempt.attempt_lease_id,
                plan.attempt.job_id,
                plan.attempt.attempt_no,
            ],
            |row| row.get::<_, String>(0),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if ids.len() > 1 {
        bail!("Execution plan identity collides with different immutable plans");
    }
    ids.first()
        .map(|id| plan_by_id_on(connection, id))
        .transpose()
}

pub(super) fn plan_by_id_on(connection: &Connection, plan_id: &str) -> Result<StoredPlan> {
    let row = connection
        .query_row(
            "SELECT plan_json, plan_digest, resource_grant_json, resource_grant_digest,
                    artifact_access_count, artifact_access_set_digest
               FROM compute_attempt_execution_plans WHERE plan_id=?1",
            params![plan_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("Execution plan is missing"))?;
    let plan: ComputeAttemptExecutionPlanEnvelope = serde_json::from_str(&row.0)?;
    let (canonical_plan, plan_digest) = canonical_execution_plan_json_and_digest(&plan)?;
    let (grant_json, grant_digest) =
        canonical_resource_grant_json_and_digest(&plan.plan.resource_grant)?;
    let access_set_digest = canonical_plan_access_set_digest(&plan.plan.artifact_accesses)?;
    if canonical_plan != row.0
        || plan.plan_id != plan_id
        || plan.plan_digest != row.1
        || plan_digest != row.1
        || grant_json != row.2
        || plan.plan.resource_grant.grant_digest != row.3
        || grant_digest != row.3
        || i64::try_from(plan.plan.artifact_accesses.len())? != row.4
        || access_set_digest != row.5
    {
        bail!("Stored execution plan failed canonical or resource audit");
    }
    audit_plan_accesses(connection, &plan)?;
    let capability = capability_collision_on(
        connection,
        &plan.plan.capability.capability_id,
        &plan.plan.capability.capability_digest,
    )?
    .ok_or_else(|| anyhow!("Execution plan capability receipt is missing"))?
    .envelope;
    if capability.capability_id != plan.plan.capability.capability_id
        || capability.capability_digest != plan.plan.capability.capability_digest
        || capability.capability.capability_kind != plan.plan.capability.capability_kind
        || capability.capability.provider_id != plan.plan.capability.provider_id
        || capability.capability.executor_id != plan.plan.capability.executor_id
        || capability.capability.expires_at != plan.plan.capability.expires_at
        || capability.capability.route.route_binding_digest != plan.plan.route_binding_digest
    {
        bail!("Execution plan capability binding failed exact audit");
    }
    let seal = audit_seal_on(connection, &plan, &row.5, row.4, &row.3)?;
    Ok(StoredPlan {
        plan,
        seal,
        capability,
    })
}

fn audit_plan_accesses(
    connection: &Connection,
    plan: &ComputeAttemptExecutionPlanEnvelope,
) -> Result<()> {
    let mut statement = connection.prepare(
        "SELECT ordinal, access_id, access_digest, access_kind, target_id, target_digest, expires_at
           FROM compute_attempt_execution_plan_accesses
          WHERE plan_id=?1 ORDER BY ordinal",
    )?;
    let rows = statement
        .query_map(params![plan.plan_id], |row| {
            Ok(
                crate::compute_federation::execution_plan::ComputeArtifactAccessBinding {
                    ordinal: row.get(0)?,
                    access_id: row.get(1)?,
                    access_digest: row.get(2)?,
                    access_kind: row.get(3)?,
                    target_id: row.get(4)?,
                    target_digest: row.get(5)?,
                    expires_at: row.get(6)?,
                },
            )
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if rows != plan.plan.artifact_accesses {
        bail!("Stored execution plan access ordering failed exact audit");
    }
    for binding in &rows {
        let access = access_collision_on(connection, &binding.access_id, &binding.access_digest)?
            .ok_or_else(|| anyhow!("Execution plan artifact access receipt is missing"))?
            .envelope;
        let (target_id, target_digest) = access_target_identity(&access);
        if access.access_id != binding.access_id
            || access.access_digest != binding.access_digest
            || access_kind(&access) != binding.access_kind
            || target_id != binding.target_id
            || target_digest != binding.target_digest
            || access.access.expires_at != binding.expires_at
        {
            bail!("Stored artifact access binding failed exact audit");
        }
    }
    Ok(())
}

fn audit_seal_on(
    connection: &Connection,
    plan: &ComputeAttemptExecutionPlanEnvelope,
    access_set_digest: &str,
    access_count: i64,
    grant_digest: &str,
) -> Result<ComputeAttemptExecutionPlanSealEnvelope> {
    let (json, stored_digest) = connection
        .query_row(
            "SELECT seal_json, seal_digest FROM compute_attempt_execution_plan_seals
              WHERE plan_id=?1 AND plan_digest=?2",
            params![plan.plan_id, plan.plan_digest],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("Execution plan seal is missing"))?;
    let seal: ComputeAttemptExecutionPlanSealEnvelope = serde_json::from_str(&json)?;
    let (canonical, digest) = canonical_execution_plan_seal_json_and_digest(&seal)?;
    if canonical != json
        || seal.seal_digest != stored_digest
        || digest != stored_digest
        || seal.plan_id != plan.plan_id
        || seal.plan_digest != plan.plan_digest
        || seal.capability_digest != plan.plan.capability.capability_digest
        || seal.artifact_access_count != access_count
        || seal.artifact_access_set_digest != access_set_digest
        || seal.resource_grant_digest != grant_digest
    {
        bail!("Stored execution plan seal failed exact audit");
    }
    Ok(seal)
}

fn audit_capability(json: &str, stored_digest: &str) -> Result<StoredCapability> {
    let envelope: ComputeExecutionCapabilityEnvelope = serde_json::from_str(json)?;
    let (canonical, digest) = canonical_execution_capability_json_and_digest(&envelope)?;
    if canonical != json || envelope.capability_digest != stored_digest || digest != stored_digest {
        bail!("Stored execution capability failed canonical audit");
    }
    Ok(StoredCapability { envelope })
}

fn audit_access(json: &str, stored_digest: &str) -> Result<StoredAccess> {
    let envelope: ComputeArtifactAccessEnvelope = serde_json::from_str(json)?;
    let (canonical, digest) = canonical_artifact_access_json_and_digest(&envelope)?;
    if canonical != json || envelope.access_digest != stored_digest || digest != stored_digest {
        bail!("Stored artifact access failed canonical audit");
    }
    Ok(StoredAccess { envelope })
}

/// Revalidates a sealed plan for v211. This proves freshness only; it neither claims an outbox
/// item nor grants network-send authority.
pub(in crate::store) fn ensure_current_plan_for_dispatch_on(
    connection: &Connection,
    command: &ComputeAttemptDispatchCommandEnvelope,
    adapter: &ComputeAttemptAdapterBinding,
    provider_owner_account_id: &str,
) -> Result<()> {
    let stored = plan_by_id_on(connection, &command.command.execution_plan.plan_id)?;
    let plan = &stored.plan.plan;
    let (_, adapter_digest) = canonical_adapter_binding_json_and_digest(adapter)?;
    let start = &command.command;
    if stored.plan.schema != start.execution_plan.plan_schema
        || stored.plan.plan_digest != start.execution_plan.plan_digest
        || plan.sources.provider.provider_owner_account_id != provider_owner_account_id
        || plan.sources.provider.provider_id != start.provider.provider_id
        || plan.sources.provider.policy_revision != start.provider.policy_revision
        || plan.sources.provider.provider_digest != start.provider.provider_digest
        || plan.sources.offer != start.offer
        || plan.sources.job != start.job
        || plan.sources.reservation.reservation_id != start.reservation.reservation_id
        || plan.sources.reservation.reservation_revision != start.reservation.reservation_revision
        || plan.sources.reservation.reservation_digest != start.reservation.reservation_digest
        || plan.sources.capacity_claim != start.capacity_claim
        || plan.attempt.job_id != start.identity.job_id
        || plan.attempt.reservation_id != start.identity.reservation_id
        || plan.attempt.attempt_lease_id != start.identity.attempt_lease_id
        || plan.attempt.attempt_no != start.identity.attempt_no
        || plan.attempt.shard_id != start.identity.shard_id
        || plan.attempt.fencing_generation != start.identity.fencing_generation
        || plan.start.executor_id != start.executor_id
        || plan.start.lease_expires_at != start.lease_expires_at
        || plan.start.hard_deadline_at != start.hard_deadline_at
        || plan.route_binding_digest != adapter_digest
        || adapter.provider_id != start.provider.provider_id
        || adapter.provider_kind != plan.sources.provider.provider_kind
    {
        bail!("Attempt dispatch does not match its exact sealed execution plan");
    }
    let sources = current_execution_sources_on(connection, &stored.plan, &stored.capability)?;
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
    let now_time = validate_canonical_timestamp(&now, "dispatch check time")?;
    let not_after = validate_canonical_timestamp(&plan.not_after, "plan not_after")?;
    let hard = validate_canonical_timestamp(&plan.start.hard_deadline_at, "hard deadline")?;
    if now_time >= not_after
        || command.not_after != plan.not_after
        || command.issued_at < plan.planned_at
        || command.issued_at < stored.seal.sealed_at
        || validate_canonical_timestamp(&plan.capability.expires_at, "capability expiry")? < hard
        || validate_canonical_timestamp(
            &plan.lease_authority.valid_until,
            "lease authority expiry",
        )? < hard
        || plan.artifact_accesses.iter().any(|access| {
            validate_canonical_timestamp(&access.expires_at, "artifact access expiry")
                .map_or(true, |expiry| expiry < hard)
        })
        || sources
            .budget_expires_at
            .as_deref()
            .is_some_and(|expiry| source_time(expiry).map_or(true, |expiry| expiry < hard))
    {
        bail!("Execution plan is not live for dispatch");
    }
    Ok(())
}
