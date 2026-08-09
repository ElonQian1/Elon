use anyhow::{bail, Result};
use chrono::Duration;

use crate::compute_federation::execution_plan::{
    ComputeArtifactAccessEnvelope, ComputeArtifactAccessTarget, ComputeAttemptExecutionPlan,
    ComputeExecutionCapabilityEnvelope, COMPUTE_ARTIFACT_ACCESS_SCHEMA,
    COMPUTE_EXECUTION_CANONICALIZATION, COMPUTE_EXECUTION_DIGEST_ALGORITHM,
};

use super::{
    types::{CurrentExecutionSources, PreparedInputs},
    validation::{validate_canonical_timestamp, validate_digest, validate_identifier},
};

const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

pub(super) fn validate_required_route_capabilities(
    plan: &ComputeAttemptExecutionPlan,
) -> Result<()> {
    let mut previous: Option<&str> = None;
    for item in &plan.required_route_capabilities {
        validate_identifier(&item.capability_id, "required route capability", 160)?;
        if !(1..=MAX_SAFE_INTEGER).contains(&item.minimum_revision)
            || previous.is_some_and(|value| value >= item.capability_id.as_str())
        {
            bail!("Required route capabilities must be positive, unique and sorted");
        }
        previous = Some(&item.capability_id);
    }
    for required in [
        "authenticated_ack",
        "authenticated_events",
        "cancel_no_start",
        "idempotent_commit",
        "prepare",
        "reconcile",
    ] {
        if !plan
            .required_route_capabilities
            .iter()
            .any(|item| item.capability_id == required && item.minimum_revision >= 1)
        {
            bail!("Required route capability set is incomplete");
        }
    }
    Ok(())
}

pub(super) fn validate_node_ready_facts(
    envelope: &ComputeExecutionCapabilityEnvelope,
) -> Result<()> {
    let Some(node) = &envelope.capability.node_ready else {
        return Ok(());
    };
    validate_digest(
        &node.installation_identity_digest,
        "node installation identity digest",
    )?;
    validate_identifier(&node.slot_ref, "node slot ref", 160)?;
    validate_identifier(&node.evidence_ref, "node evidence ref", 512)?;
    if [
        node.inventory_revision,
        node.install_generation,
        node.activation_generation,
        node.runtime_generation,
    ]
    .iter()
    .any(|value| !(1..=MAX_SAFE_INTEGER).contains(value))
    {
        bail!("Node-ready revisions and generations are invalid");
    }
    Ok(())
}

pub(super) fn validate_artifact_access(envelope: &ComputeArtifactAccessEnvelope) -> Result<()> {
    validate_identifier(&envelope.access_id, "artifact access ID", 160)?;
    validate_digest(&envelope.access_digest, "artifact access digest")?;
    if envelope.schema != COMPUTE_ARTIFACT_ACCESS_SCHEMA
        || envelope.canonicalization != COMPUTE_EXECUTION_CANONICALIZATION
        || envelope.digest_algorithm != COMPUTE_EXECUTION_DIGEST_ALGORITHM
    {
        bail!("Artifact access envelope metadata is not supported");
    }
    let access = &envelope.access;
    validate_identifier(&access.non_bearer_access_ref, "non-bearer access ref", 512)?;
    validate_digest(
        &access.authorization_digest,
        "artifact authorization digest",
    )?;
    for (label, value) in [
        ("artifact access Job", access.audience.job_id.as_str()),
        (
            "artifact access Reservation",
            access.audience.reservation_id.as_str(),
        ),
        (
            "artifact access Lease",
            access.audience.attempt_lease_id.as_str(),
        ),
        (
            "artifact access Provider",
            access.audience.provider_id.as_str(),
        ),
        (
            "artifact access executor",
            access.audience.executor_id.as_str(),
        ),
    ] {
        validate_identifier(value, label, 160)?;
    }
    validate_digest(
        &access.audience.route_binding_digest,
        "artifact access route binding digest",
    )?;
    if access.audience.fencing_generation <= 0 {
        bail!("Artifact access fencing generation is invalid");
    }
    match &access.target {
        ComputeArtifactAccessTarget::Read(target) => {
            validate_identifier(&target.artifact_id, "read artifact ID", 160)?;
            validate_identifier(
                &target.digest_algorithm,
                "read artifact digest algorithm",
                32,
            )?;
            validate_digest(&target.artifact_digest, "read artifact digest")?;
            validate_identifier(&target.media_type, "read artifact media type", 160)?;
            if target.size_bytes < 0 {
                bail!("Read artifact size is invalid");
            }
        }
        ComputeArtifactAccessTarget::Write(target) => {
            validate_identifier(&target.namespace_id, "output namespace ID", 160)?;
            validate_digest(&target.namespace_digest, "output namespace digest")?;
            validate_identifier(&target.purpose, "output namespace purpose", 160)?;
            validate_identifier(&target.media_type, "output media type", 160)?;
            if target.purpose != "result_write" || target.max_bytes <= 0 {
                bail!("Output artifact purpose or maximum size is invalid");
            }
        }
    }
    let issued = validate_canonical_timestamp(&access.issued_at, "artifact access issued_at")?;
    let expires = validate_canonical_timestamp(&access.expires_at, "artifact access expires_at")?;
    if expires <= issued {
        bail!("Artifact access lifetime is invalid");
    }
    Ok(())
}

pub(super) fn validate_plan_time_and_authority(
    plan: &ComputeAttemptExecutionPlan,
    inputs: &PreparedInputs,
    sources: &CurrentExecutionSources,
    recorded_at: &str,
) -> Result<()> {
    let checkpoint = &sources.job.job.workload.checkpoint_policy;
    if checkpoint.mode != "disabled"
        || checkpoint.interval_seconds.is_some()
        || checkpoint.max_checkpoints != 0
        || checkpoint.checkpoint_media_type.is_some()
    {
        bail!(
            "Execution plans require the disabled checkpoint policy until checkpoint access exists"
        );
    }
    let planned = validate_canonical_timestamp(&plan.planned_at, "plan planned_at")?;
    let recorded = validate_canonical_timestamp(recorded_at, "plan recorded_at")?;
    let not_after = validate_canonical_timestamp(&plan.not_after, "plan not_after")?;
    let lease = validate_canonical_timestamp(&plan.start.lease_expires_at, "lease expiry")?;
    let hard = validate_canonical_timestamp(&plan.start.hard_deadline_at, "hard deadline")?;
    if planned > recorded
        || recorded >= not_after
        || not_after >= lease
        || lease >= hard
        || lease - not_after < Duration::seconds(60)
        || hard - planned > Duration::seconds(plan.resource_grant.max_runtime_seconds)
        || hard > source_time(&sources.job.job.workload.deadline_at)?
        || hard > source_time(&sources.reservation.reservation.expires_at)?
        || planned < source_time(&sources.historical_offer.offer.valid_from)?
        || hard > source_time(&sources.historical_offer.offer.valid_until)?
        || sources
            .claim
            .expires_at
            .as_deref()
            .is_some_and(|value| source_time(value).map_or(true, |expiry| hard > expiry))
        || sources
            .budget_expires_at
            .as_deref()
            .is_some_and(|value| source_time(value).map_or(true, |expiry| hard > expiry))
    {
        bail!("Execution plan, lease, source or budget lifetime is invalid");
    }
    let capability = &inputs.capability.envelope.capability;
    if validate_canonical_timestamp(&capability.observed_at, "capability observed_at")? > planned
        || validate_canonical_timestamp(&capability.expires_at, "capability expires_at")? < hard
    {
        bail!("Execution capability does not cover the complete plan lifetime");
    }
    for access in &inputs.accesses {
        if validate_canonical_timestamp(&access.envelope.access.issued_at, "access issued_at")?
            > planned
            || validate_canonical_timestamp(
                &access.envelope.access.expires_at,
                "access expires_at",
            )? < hard
        {
            bail!("Artifact access does not cover the complete plan lifetime");
        }
    }
    let authority = &plan.lease_authority;
    validate_identifier(&authority.authority_kind, "lease authority kind", 80)?;
    validate_identifier(&authority.delivery_mode, "lease delivery mode", 80)?;
    for scope in &authority.required_scopes {
        validate_identifier(scope, "lease authority scope", 160)?;
    }
    if authority.audience != capability.executor_id
        || authority.attempt_lease_id != plan.attempt.attempt_lease_id
        || authority.fencing_generation != plan.attempt.fencing_generation
        || validate_canonical_timestamp(&authority.valid_until, "lease authority valid_until")?
            < hard
        || authority.required_scopes.is_empty()
        || !authority
            .required_scopes
            .windows(2)
            .all(|pair| pair[0] < pair[1])
    {
        bail!("Lease authority does not bind the complete Attempt lifetime and scope set");
    }
    Ok(())
}

pub(super) fn source_time(value: &str) -> Result<chrono::DateTime<chrono::FixedOffset>> {
    chrono::DateTime::parse_from_rfc3339(value).map_err(Into::into)
}
