use std::collections::BTreeSet;

use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use crate::compute_federation::{
    attempt_gateway::{
        COMPUTE_ATTEMPT_ROUTE_PROVIDER_ENDPOINT, COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER,
    },
    execution_plan::{
        canonical_artifact_access_json_and_digest, canonical_execution_capability_json_and_digest,
        canonical_execution_plan_json_and_digest, ComputeArtifactAccessEnvelope,
        ComputeArtifactAccessTarget, ComputeAttemptExecutionPlanEnvelope,
        ComputeExecutionCapabilityEnvelope, ValidatedComputeAttemptExecutionPlanInputs,
        ARTIFACT_ACCESS_READ, ARTIFACT_ACCESS_WRITE, COMPUTE_ATTEMPT_EXECUTION_PLAN_SCHEMA,
        COMPUTE_EXECUTION_CANONICALIZATION, COMPUTE_EXECUTION_CAPABILITY_SCHEMA,
        COMPUTE_EXECUTION_DIGEST_ALGORITHM, EXECUTION_CAPABILITY_ADAPTER_EXECUTION,
        EXECUTION_CAPABILITY_NODE_READY, EXECUTION_CAPABILITY_PROVIDER_ENDPOINT,
    },
    provider::{
        PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_KIND_MANAGED_CLUSTER, PROVIDER_KIND_USER_NODE,
    },
};

use super::replay_validation::{
    validate_artifact_access, validate_node_ready_facts, validate_required_route_capabilities,
};
use super::types::{PreparedArtifactAccess, PreparedCapability, PreparedInputs};

const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

pub(super) fn prepare_inputs(
    input: &ValidatedComputeAttemptExecutionPlanInputs,
) -> Result<PreparedInputs> {
    validate_plan_envelope(input.plan())?;
    let capability = input.capability().envelope();
    validate_capability(capability)?;
    let (capability_json, capability_digest) =
        canonical_execution_capability_json_and_digest(capability)?;
    if capability_digest != capability.capability_digest {
        bail!("Execution capability digest does not match its canonical payload");
    }

    let mut access_ids = BTreeSet::new();
    let mut accesses = Vec::with_capacity(input.artifact_accesses().len());
    for verified in input.artifact_accesses() {
        let envelope = verified.envelope();
        validate_artifact_access(envelope)?;
        if !access_ids.insert(envelope.access_id.as_str()) {
            bail!("Execution plan contains a duplicate artifact access ID");
        }
        let (canonical_json, digest) = canonical_artifact_access_json_and_digest(envelope)?;
        if digest != envelope.access_digest {
            bail!("Artifact access digest does not match its canonical payload");
        }
        accesses.push(PreparedArtifactAccess {
            envelope: envelope.clone(),
            canonical_json,
            digest,
        });
    }

    Ok(PreparedInputs {
        capability: PreparedCapability {
            envelope: capability.clone(),
            canonical_json: capability_json,
            digest: capability_digest,
        },
        accesses,
    })
}

pub(super) fn validate_plan_envelope(plan: &ComputeAttemptExecutionPlanEnvelope) -> Result<()> {
    validate_identifier(&plan.plan_id, "execution plan ID", 120)?;
    validate_digest(&plan.plan_digest, "execution plan digest")?;
    if plan.schema != COMPUTE_ATTEMPT_EXECUTION_PLAN_SCHEMA
        || plan.canonicalization != COMPUTE_EXECUTION_CANONICALIZATION
        || plan.digest_algorithm != COMPUTE_EXECUTION_DIGEST_ALGORITHM
    {
        bail!("Execution plan envelope metadata is not supported");
    }
    let candidate = &plan.plan;
    validate_identifier(
        &candidate.sources.consumer_account_id,
        "execution plan consumer",
        160,
    )?;
    for (label, value) in [
        (
            "execution plan Provider",
            candidate.sources.provider.provider_id.as_str(),
        ),
        (
            "execution plan Provider owner",
            candidate
                .sources
                .provider
                .provider_owner_account_id
                .as_str(),
        ),
        (
            "execution plan Offer",
            candidate.sources.offer.offer_id.as_str(),
        ),
        ("execution plan Job", candidate.sources.job.job_id.as_str()),
        (
            "execution plan Reservation",
            candidate.sources.reservation.reservation_id.as_str(),
        ),
        (
            "execution plan Capacity Claim",
            candidate.sources.capacity_claim.claim_id.as_str(),
        ),
        (
            "execution plan price snapshot",
            candidate.sources.price_snapshot.price_snapshot_id.as_str(),
        ),
        (
            "execution plan budget reservation",
            candidate.sources.budget.budget_reservation_id.as_str(),
        ),
        (
            "execution plan lease",
            candidate.attempt.attempt_lease_id.as_str(),
        ),
        (
            "execution plan executor",
            candidate.start.executor_id.as_str(),
        ),
    ] {
        validate_identifier(value, label, 160)?;
    }
    for (label, digest) in [
        (
            "Provider digest",
            candidate.sources.provider.provider_digest.as_str(),
        ),
        (
            "Offer digest",
            candidate.sources.offer.offer_digest.as_str(),
        ),
        ("Job digest", candidate.sources.job.job_digest.as_str()),
        (
            "Reservation digest",
            candidate.sources.reservation.reservation_digest.as_str(),
        ),
        (
            "Capacity Claim digest",
            candidate.sources.capacity_claim.claim_digest.as_str(),
        ),
        (
            "price snapshot digest",
            candidate
                .sources
                .price_snapshot
                .price_snapshot_digest
                .as_str(),
        ),
        (
            "Broker request digest",
            candidate.sources.broker_request_digest.as_str(),
        ),
        (
            "route binding digest",
            candidate.route_binding_digest.as_str(),
        ),
        (
            "capability digest",
            candidate.capability.capability_digest.as_str(),
        ),
        (
            "resource grant digest",
            candidate.resource_grant.grant_digest.as_str(),
        ),
    ] {
        validate_digest(digest, label)?;
    }
    if candidate.attempt.attempt_no != 1
        || candidate.attempt.fencing_generation != 1
        || candidate.sources.provider.policy_revision <= 0
        || candidate.sources.offer.offer_version <= 0
        || candidate.sources.job.job_revision <= 0
        || candidate.sources.reservation.reservation_revision <= 0
        || candidate.sources.capacity_claim.claim_revision <= 0
        || candidate.sources.budget.budget_reserved_fen < 0
    {
        bail!("Execution plan source revision, budget or Attempt identity is invalid");
    }
    validate_canonical_timestamp(&candidate.planned_at, "execution plan planned_at")?;
    validate_canonical_timestamp(&candidate.not_after, "execution plan not_after")?;
    validate_required_route_capabilities(candidate)?;
    let (_, digest) = canonical_execution_plan_json_and_digest(plan)?;
    if digest != plan.plan_digest {
        bail!("Execution plan digest does not match its canonical payload");
    }
    Ok(())
}

pub(super) fn validate_capability(envelope: &ComputeExecutionCapabilityEnvelope) -> Result<()> {
    validate_identifier(&envelope.capability_id, "execution capability ID", 160)?;
    validate_digest(&envelope.capability_digest, "execution capability digest")?;
    if envelope.schema != COMPUTE_EXECUTION_CAPABILITY_SCHEMA
        || envelope.canonicalization != COMPUTE_EXECUTION_CANONICALIZATION
        || envelope.digest_algorithm != COMPUTE_EXECUTION_DIGEST_ALGORITHM
    {
        bail!("Execution capability envelope metadata is not supported");
    }
    let capability = &envelope.capability;
    for (label, value) in [
        ("capability Provider", capability.provider_id.as_str()),
        (
            "capability Provider kind",
            capability.provider_kind.as_str(),
        ),
        ("capability executor", capability.executor_id.as_str()),
        (
            "capability route kind",
            capability.route.route_kind.as_str(),
        ),
        (
            "capability Adapter ID",
            capability.route.adapter_id.as_str(),
        ),
        (
            "capability Adapter version",
            capability.route.adapter_version.as_str(),
        ),
        (
            "capability source schema",
            capability.provenance.source_schema.as_str(),
        ),
        (
            "capability source ID",
            capability.provenance.source_id.as_str(),
        ),
        (
            "capability verification kind",
            capability.provenance.verification_kind.as_str(),
        ),
        (
            "capability verifier",
            capability.provenance.verifier_id.as_str(),
        ),
        (
            "capability runner ID",
            capability.runtime.runner_id.as_str(),
        ),
        (
            "capability runtime family",
            capability.runtime.runtime_family.as_str(),
        ),
        (
            "capability runtime version",
            capability.runtime.runtime_version.as_str(),
        ),
        (
            "capability precision",
            capability.runtime.precision.as_str(),
        ),
    ] {
        validate_identifier(value, label, 256)?;
    }
    for (label, digest) in [
        (
            "capability route digest",
            capability.route.route_binding_digest.as_str(),
        ),
        (
            "capability source digest",
            capability.provenance.source_digest.as_str(),
        ),
        (
            "capability verification digest",
            capability.provenance.verification_digest.as_str(),
        ),
        (
            "capability runner digest",
            capability.runtime.runner_digest.as_str(),
        ),
        (
            "capability runtime digest",
            capability.runtime.runtime_digest.as_str(),
        ),
    ] {
        validate_digest(digest, label)?;
    }
    validate_identifier(
        &capability.route.adapter_config_digest,
        "capability Adapter config digest",
        512,
    )?;
    if !matches!(
        capability.provider_kind.as_str(),
        PROVIDER_KIND_USER_NODE | PROVIDER_KIND_MANAGED_CLUSTER | PROVIDER_KIND_EXTERNAL_POOL
    ) || capability.route.adapter_config_revision <= 0
    {
        bail!("Execution capability Provider kind or Adapter revision is invalid");
    }
    validate_capability_route(envelope)?;
    validate_node_ready_facts(envelope)?;
    validate_runtime_and_model(envelope)?;
    validate_resource_ceiling(envelope)?;
    validate_canonical_timestamp(&capability.provenance.authenticated_at, "authenticated_at")?;
    let observed = validate_canonical_timestamp(&capability.observed_at, "capability observed_at")?;
    let expires = validate_canonical_timestamp(&capability.expires_at, "capability expires_at")?;
    if expires <= observed {
        bail!("Execution capability lifetime is invalid");
    }
    Ok(())
}

fn validate_capability_route(envelope: &ComputeExecutionCapabilityEnvelope) -> Result<()> {
    let capability = &envelope.capability;
    let route = &capability.route;
    match capability.capability_kind.as_str() {
        EXECUTION_CAPABILITY_NODE_READY => {
            if capability.provider_kind != PROVIDER_KIND_USER_NODE
                || route.route_kind != COMPUTE_ATTEMPT_ROUTE_PROVIDER_ENDPOINT
                || capability.node_ready.is_none()
                || capability.runtime.plugin_release.is_none()
            {
                bail!("Node-ready capability route or node binding is invalid");
            }
        }
        EXECUTION_CAPABILITY_PROVIDER_ENDPOINT => {
            if capability.provider_kind != PROVIDER_KIND_MANAGED_CLUSTER
                || route.route_kind != COMPUTE_ATTEMPT_ROUTE_PROVIDER_ENDPOINT
                || capability.node_ready.is_some()
            {
                bail!("Provider-endpoint capability route is invalid");
            }
        }
        EXECUTION_CAPABILITY_ADAPTER_EXECUTION => {
            if !matches!(
                capability.provider_kind.as_str(),
                PROVIDER_KIND_MANAGED_CLUSTER | PROVIDER_KIND_EXTERNAL_POOL
            ) || route.route_kind != COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER
                || capability.node_ready.is_some()
            {
                bail!("Server-Adapter capability route is invalid");
            }
        }
        _ => bail!("Execution capability kind is not supported"),
    }
    match route.route_kind.as_str() {
        COMPUTE_ATTEMPT_ROUTE_PROVIDER_ENDPOINT => {
            validate_identifier(
                route.endpoint_id.as_deref().unwrap_or_default(),
                "capability endpoint ID",
                160,
            )?;
            validate_identifier(
                route.endpoint_transport.as_deref().unwrap_or_default(),
                "capability endpoint transport",
                80,
            )?;
        }
        COMPUTE_ATTEMPT_ROUTE_SERVER_ADAPTER => {
            if route.endpoint_id.is_some() || route.endpoint_transport.is_some() {
                bail!("Server-Adapter capability cannot carry endpoint fields");
            }
        }
        _ => bail!("Execution capability route kind is not supported"),
    }
    Ok(())
}

fn validate_runtime_and_model(envelope: &ComputeExecutionCapabilityEnvelope) -> Result<()> {
    let runtime = &envelope.capability.runtime;
    if let Some(release) = &runtime.plugin_release {
        for (label, value) in [
            ("plugin ID", release.plugin_id.as_str()),
            ("plugin version", release.plugin_version.as_str()),
            ("plugin target", release.target_id.as_str()),
        ] {
            validate_identifier(value, label, 160)?;
        }
        validate_digest(&release.manifest_digest, "plugin Manifest digest")?;
        validate_digest(&release.package_digest, "plugin package digest")?;
    }
    if let Some(model) = &envelope.capability.model {
        validate_identifier(&model.model_id, "model ID", 160)?;
        validate_identifier(&model.model_family, "model family", 160)?;
        validate_digest(&model.model_digest, "model digest")?;
        if let Some(tokenizer) = &model.tokenizer_digest {
            validate_digest(tokenizer, "tokenizer digest")?;
        }
        let mut adapter_digests = BTreeSet::new();
        for digest in &model.adapter_digests {
            validate_digest(digest, "model Adapter digest")?;
            if !adapter_digests.insert(digest.as_str()) {
                bail!("Model Adapter digests must be unique");
            }
        }
    }
    Ok(())
}

fn validate_resource_ceiling(envelope: &ComputeExecutionCapabilityEnvelope) -> Result<()> {
    let ceiling = &envelope.capability.resource_ceiling;
    if ceiling.accelerator_count <= 0
        || ceiling.max_cpu_millicores <= 0
        || ceiling.max_memory_bytes <= 0
        || ceiling.max_vram_bytes < 0
        || ceiling.max_disk_bytes < 0
        || ceiling.max_processes <= 0
        || ceiling.max_runtime_seconds <= 0
        || ceiling.max_output_bytes <= 0
        || ceiling.max_concurrent_attempts <= 0
        || [
            ceiling.accelerator_count,
            ceiling.max_cpu_millicores,
            ceiling.max_memory_bytes,
            ceiling.max_vram_bytes,
            ceiling.max_disk_bytes,
            ceiling.max_processes,
            ceiling.max_runtime_seconds,
            ceiling.max_output_bytes,
            ceiling.max_concurrent_attempts,
        ]
        .iter()
        .any(|value| *value > MAX_SAFE_INTEGER)
    {
        bail!("Execution capability numeric resource ceiling is invalid");
    }
    Ok(())
}

pub(super) fn access_kind(envelope: &ComputeArtifactAccessEnvelope) -> &'static str {
    match &envelope.access.target {
        ComputeArtifactAccessTarget::Read(_) => ARTIFACT_ACCESS_READ,
        ComputeArtifactAccessTarget::Write(_) => ARTIFACT_ACCESS_WRITE,
    }
}

pub(super) fn access_target_identity(envelope: &ComputeArtifactAccessEnvelope) -> (&str, &str) {
    match &envelope.access.target {
        ComputeArtifactAccessTarget::Read(target) => (&target.artifact_id, &target.artifact_digest),
        ComputeArtifactAccessTarget::Write(target) => {
            (&target.namespace_id, &target.namespace_digest)
        }
    }
}

pub(super) fn validate_identifier(value: &str, label: &str, limit: usize) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.len() > limit
        || value.chars().any(char::is_control)
    {
        bail!("{label} is invalid");
    }
    Ok(())
}

pub(super) fn validate_digest(value: &str, label: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("{label} must be a lowercase SHA-256 digest");
    }
    Ok(())
}

pub(super) fn validate_canonical_timestamp(
    value: &str,
    label: &str,
) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| anyhow::anyhow!("{label} is not RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("{label} must use canonical UTC nanoseconds");
    }
    Ok(parsed)
}
