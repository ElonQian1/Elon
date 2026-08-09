use crate::compute_federation::{
    attempt::{
        ComputeAttemptArtifactRef, ComputeAttemptCheckpointPolicy, ComputeAttemptModelBinding,
        ComputeAttemptOfferBinding, ComputeAttemptOutputContract, ComputeAttemptResourceLimits,
        ComputeAttemptShardSpec, ComputeAttemptStart, ComputeAttemptUsageLimit,
        ComputeAttemptWorkload,
    },
    attempt_gateway::{
        canonical_adapter_binding_json_and_digest, ComputeAttemptAdapterBinding,
        COMPUTE_ATTEMPT_ADAPTER_BINDING_SCHEMA,
    },
    execution_plan::{
        canonical_execution_plan_json_and_digest, canonical_execution_plan_seal_json_and_digest,
        canonical_input_digest, canonical_plan_access_set_digest,
        canonical_resource_grant_json_and_digest, canonical_workload_spec_digest,
        ComputeArtifactAccessBinding, ComputeArtifactAccessTarget, ComputeAttemptExecutionPlan,
        ComputeAttemptExecutionPlanEnvelope, ComputeAttemptExecutionPlanSealEnvelope,
        ComputeExecutionResourceGrant, COMPUTE_ATTEMPT_EXECUTION_PLAN_SEAL_SCHEMA,
        COMPUTE_EXECUTION_CANONICALIZATION, COMPUTE_EXECUTION_DIGEST_ALGORITHM,
        COMPUTE_EXECUTION_RESOURCE_GRANT_SCHEMA, EXECUTION_CAPABILITY_ADAPTER_EXECUTION,
        EXECUTION_CAPABILITY_NODE_READY, EXECUTION_CAPABILITY_PROVIDER_ENDPOINT,
        RESOURCE_GRANT_NODE_HOST, RESOURCE_GRANT_PROVIDER_RUNTIME, RESOURCE_GRANT_SERVER_ADAPTER,
    },
    workload::ComputeModelRef,
};
use anyhow::{anyhow, bail, Result};

use super::{
    read::StoredPlan,
    replay_validation::validate_plan_time_and_authority,
    source::{derive_capability_binding, derive_source_bindings},
    types::{
        ComputeAttemptExecutionPlanReceipt, CurrentExecutionSources, PreparedInputs, PreparedPlan,
    },
    validation::{access_kind, access_target_identity},
};

pub(super) fn prepare_plan(
    candidate: &ComputeAttemptExecutionPlanEnvelope,
    inputs: &PreparedInputs,
    sources: &CurrentExecutionSources,
    recorded_at: &str,
) -> Result<PreparedPlan> {
    let plan = &candidate.plan;
    ensure_route_and_runtime(plan, inputs, sources)?;
    let resource_grant = derive_resource_grant(candidate, inputs, sources)?;
    let (resource_grant_json, resource_grant_digest) =
        canonical_resource_grant_json_and_digest(&resource_grant)?;
    let (artifact_accesses, input_artifacts) =
        derive_artifact_accesses(plan, inputs, sources, &resource_grant)?;
    let expected = ComputeAttemptExecutionPlan {
        sources: derive_source_bindings(sources),
        attempt: plan.attempt.clone(),
        route_binding_digest: inputs
            .capability
            .envelope
            .capability
            .route
            .route_binding_digest
            .clone(),
        capability: derive_capability_binding(inputs),
        start: derive_start(plan, sources, inputs, &resource_grant, input_artifacts)?,
        artifact_accesses,
        resource_grant,
        lease_authority: plan.lease_authority.clone(),
        required_route_capabilities: plan.required_route_capabilities.clone(),
        planned_at: plan.planned_at.clone(),
        not_after: plan.not_after.clone(),
    };
    validate_plan_time_and_authority(&expected, inputs, sources, recorded_at)?;
    if &expected != plan {
        bail!("Execution plan does not equal the server-derived source, Start and grant");
    }
    let (plan_json, plan_digest) = canonical_execution_plan_json_and_digest(candidate)?;
    if plan_digest != candidate.plan_digest {
        bail!("Execution plan canonical digest changed during derivation");
    }
    let access_set_digest = canonical_plan_access_set_digest(&expected.artifact_accesses)?;
    let mut seal = ComputeAttemptExecutionPlanSealEnvelope {
        schema: COMPUTE_ATTEMPT_EXECUTION_PLAN_SEAL_SCHEMA.to_string(),
        seal_id: format!("seal:{}", candidate.plan_id),
        seal_digest: String::new(),
        canonicalization: COMPUTE_EXECUTION_CANONICALIZATION.to_string(),
        digest_algorithm: COMPUTE_EXECUTION_DIGEST_ALGORITHM.to_string(),
        plan_id: candidate.plan_id.clone(),
        plan_digest: candidate.plan_digest.clone(),
        capability_digest: inputs.capability.digest.clone(),
        artifact_access_count: i64::try_from(expected.artifact_accesses.len())?,
        artifact_access_set_digest: access_set_digest.clone(),
        resource_grant_digest: resource_grant_digest.clone(),
        sealed_at: recorded_at.to_string(),
    };
    let (_, seal_digest) = canonical_execution_plan_seal_json_and_digest(&seal)?;
    seal.seal_digest = seal_digest.clone();
    let (seal_json, checked_seal_digest) = canonical_execution_plan_seal_json_and_digest(&seal)?;
    if checked_seal_digest != seal_digest {
        bail!("Execution plan seal digest is not deterministic");
    }
    Ok(PreparedPlan {
        plan: candidate.clone(),
        plan_json,
        plan_digest,
        access_set_digest,
        resource_grant_json,
        resource_grant_digest,
        seal,
        seal_json,
        seal_digest,
    })
}

fn ensure_route_and_runtime(
    plan: &ComputeAttemptExecutionPlan,
    inputs: &PreparedInputs,
    sources: &CurrentExecutionSources,
) -> Result<()> {
    let capability = &inputs.capability.envelope.capability;
    let route = &capability.route;
    let adapter = ComputeAttemptAdapterBinding {
        schema: COMPUTE_ATTEMPT_ADAPTER_BINDING_SCHEMA.to_string(),
        provider_id: capability.provider_id.clone(),
        provider_kind: capability.provider_kind.clone(),
        route_kind: route.route_kind.clone(),
        endpoint_id: route.endpoint_id.clone(),
        endpoint_transport: route.endpoint_transport.clone(),
        adapter_id: route.adapter_id.clone(),
        adapter_version: route.adapter_version.clone(),
        config_revision: route.adapter_config_revision,
        config_digest: route.adapter_config_digest.clone(),
    };
    let (_, route_digest) = canonical_adapter_binding_json_and_digest(&adapter)?;
    let offer = &sources.historical_offer.offer;
    let runtime = &capability.runtime;
    if route_digest != route.route_binding_digest
        || route_digest != plan.route_binding_digest
        || runtime.runtime_family != offer.runtime.runtime_family
        || runtime.runtime_version != offer.runtime.runtime_version
        || runtime.precision != offer.runtime.precision
        || runtime.runner_digest != offer.runtime.runner_digest
        || !runtime_plugin_matches_offer(runtime, &offer.runtime)
        || !model_matches_offer(capability.model.as_ref(), offer.model.as_ref())
        || sources.job.job.workload.model != offer.model
        || sources
            .job
            .job
            .workload
            .runtime
            .as_ref()
            .is_some_and(|value| value != &offer.runtime)
    {
        bail!("Execution capability route, runtime, Plugin or model is not exact");
    }
    Ok(())
}

fn runtime_plugin_matches_offer(
    runtime: &crate::compute_federation::attempt::ComputeAttemptRuntimeBinding,
    offer: &crate::compute_federation::workload::ComputeRuntimeRef,
) -> bool {
    match (
        offer.plugin_id.as_deref(),
        offer.plugin_version.as_deref(),
        offer.plugin_digest.as_deref(),
        runtime.plugin_release.as_ref(),
    ) {
        (None, None, None, None) => true,
        (Some(id), Some(version), Some(digest), Some(release)) => {
            release.plugin_id == id
                && release.plugin_version == version
                && release.manifest_digest == digest
        }
        _ => false,
    }
}

fn model_matches_offer(
    selected: Option<&ComputeAttemptModelBinding>,
    offer: Option<&ComputeModelRef>,
) -> bool {
    match (selected, offer) {
        (None, None) => true,
        (Some(selected), Some(offer)) => {
            selected.model_id == offer.model_id
                && selected.model_family == offer.model_family
                && selected.model_digest == offer.model_digest
                && selected.tokenizer_digest == offer.tokenizer_digest
                && selected.adapter_digests == offer.adapter_digests
        }
        _ => false,
    }
}

fn derive_resource_grant(
    candidate: &ComputeAttemptExecutionPlanEnvelope,
    inputs: &PreparedInputs,
    sources: &CurrentExecutionSources,
) -> Result<ComputeExecutionResourceGrant> {
    let workload = &sources.job.job.workload;
    let offer = &sources.historical_offer.offer;
    let provider = &sources.historical_provider.provider;
    let ceiling = &inputs.capability.envelope.capability.resource_ceiling;
    if !provider
        .capabilities
        .task_kinds
        .contains(&workload.task_kind)
        || !provider
            .capabilities
            .allowed_data_classes
            .contains(&workload.data_class)
        || offer.sku.task_kind != workload.task_kind
        || (!offer.authorization.allowed_data_classes.is_empty()
            && !offer
                .authorization
                .allowed_data_classes
                .contains(&workload.data_class))
        || !workload
            .resources
            .accelerator_kinds
            .contains(&offer.resource_profile.accelerator_kind)
        || workload.resources.min_accelerator_count > offer.resource_profile.accelerator_count
        || workload.resources.min_accelerator_count > ceiling.accelerator_count
        || workload.resources.min_vram_bytes > offer.resource_profile.vram_bytes
        || workload.resources.min_vram_bytes > ceiling.max_vram_bytes
        || workload.resources.min_ram_bytes > offer.resource_profile.ram_bytes
        || workload.resources.min_ram_bytes > ceiling.max_memory_bytes
        || workload.resources.min_disk_bytes > ceiling.max_disk_bytes
        || workload.output.streaming && !provider.capabilities.supports_streaming
    {
        bail!("Job minimum resources or policy exceed Offer, Provider or capability");
    }
    let enforcement_kind = match inputs
        .capability
        .envelope
        .capability
        .capability_kind
        .as_str()
    {
        EXECUTION_CAPABILITY_NODE_READY => RESOURCE_GRANT_NODE_HOST,
        EXECUTION_CAPABILITY_PROVIDER_ENDPOINT => RESOURCE_GRANT_PROVIDER_RUNTIME,
        EXECUTION_CAPABILITY_ADAPTER_EXECUTION => RESOURCE_GRANT_SERVER_ADAPTER,
        _ => bail!("Execution capability cannot derive a resource enforcement kind"),
    };
    let mut grant = ComputeExecutionResourceGrant {
        schema: COMPUTE_EXECUTION_RESOURCE_GRANT_SCHEMA.to_string(),
        grant_id: format!("grant:{}", candidate.plan_id),
        grant_digest: String::new(),
        enforcement_kind: enforcement_kind.to_string(),
        accelerator_count: workload.resources.min_accelerator_count,
        cpu_millicores: ceiling.max_cpu_millicores,
        memory_bytes: workload.resources.min_ram_bytes,
        vram_bytes: workload.resources.min_vram_bytes,
        disk_bytes: workload.resources.min_disk_bytes,
        max_processes: ceiling.max_processes,
        max_runtime_seconds: workload
            .resources
            .max_runtime_seconds
            .min(offer.execution_limits.max_attempt_runtime_seconds)
            .min(ceiling.max_runtime_seconds),
        max_output_bytes: workload
            .output
            .max_output_bytes
            .min(ceiling.max_output_bytes),
        concurrency_units: 1,
        allow_network_egress: workload.resources.allow_network_egress
            && ceiling.allow_network_egress,
        usage_limits: workload
            .usage_limits
            .iter()
            .map(|item| ComputeAttemptUsageLimit {
                meter: item.meter.clone(),
                max_quantity: item.max_quantity,
            })
            .collect(),
    };
    if grant.max_runtime_seconds <= 0 || grant.max_output_bytes <= 0 {
        bail!("Server-derived execution grant is empty");
    }
    let (_, digest) = canonical_resource_grant_json_and_digest(&grant)?;
    grant.grant_digest = digest;
    Ok(grant)
}

fn derive_artifact_accesses(
    plan: &ComputeAttemptExecutionPlan,
    inputs: &PreparedInputs,
    sources: &CurrentExecutionSources,
    grant: &ComputeExecutionResourceGrant,
) -> Result<(
    Vec<ComputeArtifactAccessBinding>,
    Vec<ComputeAttemptArtifactRef>,
)> {
    let workload = &sources.job.job.workload;
    let expected_count = workload.input_artifacts.len()
        + if workload.output.result_artifact_required {
            1
        } else {
            0
        };
    if inputs.accesses.len() != expected_count {
        bail!("Artifact access set does not exactly cover Start inputs and output");
    }
    let mut bindings = Vec::with_capacity(expected_count);
    let mut artifacts = Vec::with_capacity(workload.input_artifacts.len());
    for (ordinal, (artifact, prepared)) in workload
        .input_artifacts
        .iter()
        .zip(inputs.accesses.iter())
        .enumerate()
    {
        let ComputeArtifactAccessTarget::Read(target) = &prepared.envelope.access.target else {
            bail!("Input artifact requires a read authorization");
        };
        if target.artifact_id != artifact.artifact_id
            || target.digest_algorithm != artifact.digest_algorithm
            || target.artifact_digest != artifact.digest
            || target.media_type != artifact.media_type
            || target.size_bytes != artifact.size_bytes
        {
            bail!("Read authorization does not bind the exact input artifact");
        }
        artifacts.push(ComputeAttemptArtifactRef {
            artifact_id: target.artifact_id.clone(),
            digest_algorithm: target.digest_algorithm.clone(),
            digest: target.artifact_digest.clone(),
            media_type: target.media_type.clone(),
            size_bytes: target.size_bytes,
            access_ref: prepared.envelope.access.non_bearer_access_ref.clone(),
        });
        bindings.push(access_binding(ordinal, prepared)?);
    }
    if workload.output.result_artifact_required {
        let prepared = inputs
            .accesses
            .last()
            .ok_or_else(|| anyhow!("Output access is missing"))?;
        let ComputeArtifactAccessTarget::Write(target) = &prepared.envelope.access.target else {
            bail!("Result artifact requires a write authorization");
        };
        if target.purpose != "result_write"
            || target.media_type != workload.output.media_type
            || target.max_bytes != grant.max_output_bytes
        {
            bail!("Write authorization does not bind the exact output contract");
        }
        bindings.push(access_binding(bindings.len(), prepared)?);
    }
    for prepared in &inputs.accesses {
        ensure_access_audience(plan, &prepared.envelope)?;
    }
    Ok((bindings, artifacts))
}

fn access_binding(
    ordinal: usize,
    prepared: &super::types::PreparedArtifactAccess,
) -> Result<ComputeArtifactAccessBinding> {
    let (target_id, target_digest) = access_target_identity(&prepared.envelope);
    Ok(ComputeArtifactAccessBinding {
        ordinal: i64::try_from(ordinal)?,
        access_id: prepared.envelope.access_id.clone(),
        access_digest: prepared.digest.clone(),
        access_kind: access_kind(&prepared.envelope).to_string(),
        target_id: target_id.to_string(),
        target_digest: target_digest.to_string(),
        expires_at: prepared.envelope.access.expires_at.clone(),
    })
}

fn ensure_access_audience(
    plan: &ComputeAttemptExecutionPlan,
    envelope: &crate::compute_federation::execution_plan::ComputeArtifactAccessEnvelope,
) -> Result<()> {
    let audience = &envelope.access.audience;
    if audience.job_id != plan.attempt.job_id
        || audience.reservation_id != plan.attempt.reservation_id
        || audience.attempt_lease_id != plan.attempt.attempt_lease_id
        || audience.provider_id != plan.sources.provider.provider_id
        || audience.executor_id != plan.capability.executor_id
        || audience.fencing_generation != plan.attempt.fencing_generation
        || audience.route_binding_digest != plan.route_binding_digest
    {
        bail!("Artifact access audience does not bind the exact Attempt and route");
    }
    Ok(())
}

fn derive_start(
    plan: &ComputeAttemptExecutionPlan,
    sources: &CurrentExecutionSources,
    inputs: &PreparedInputs,
    grant: &ComputeExecutionResourceGrant,
    artifacts: Vec<ComputeAttemptArtifactRef>,
) -> Result<ComputeAttemptStart> {
    let workload = &sources.job.job.workload;
    if plan.attempt.job_id != sources.job.job.job_id
        || plan.attempt.reservation_id != sources.reservation.reservation.reservation_id
        || plan.attempt.shard_id != workload.shard.as_ref().map(|item| item.shard_id.clone())
    {
        bail!("Execution plan Attempt identity does not bind the exact workload");
    }
    Ok(ComputeAttemptStart {
        identity: plan.attempt.clone(),
        provider_id: sources.historical_provider.provider.provider_id.clone(),
        executor_id: inputs.capability.envelope.capability.executor_id.clone(),
        offer: ComputeAttemptOfferBinding {
            offer_id: sources.historical_offer.offer.offer_id.clone(),
            offer_version: sources.historical_offer.offer.offer_version,
            offer_digest: sources.historical_offer.offer.offer_digest.clone(),
        },
        selected_runtime: inputs.capability.envelope.capability.runtime.clone(),
        selected_model: inputs.capability.envelope.capability.model.clone(),
        workload: ComputeAttemptWorkload {
            workload_schema: workload.schema.clone(),
            workload_spec_digest: canonical_workload_spec_digest(workload)?,
            canonical_input_digest: canonical_input_digest(&workload.input_artifacts)?,
            task_kind: workload.task_kind.clone(),
            data_class: workload.data_class.clone(),
            shard: workload.shard.as_ref().map(|item| ComputeAttemptShardSpec {
                shard_id: item.shard_id.clone(),
                shard_index: item.shard_index,
                shard_count: item.shard_count,
                merge_strategy: item.merge_strategy.clone(),
            }),
            input_artifacts: artifacts,
            output: ComputeAttemptOutputContract {
                media_type: workload.output.media_type.clone(),
                max_output_bytes: grant.max_output_bytes,
                streaming: workload.output.streaming,
                result_artifact_required: workload.output.result_artifact_required,
                deterministic_digest_expected: workload.output.deterministic_digest_expected,
            },
            resources: ComputeAttemptResourceLimits {
                accelerator_count: grant.accelerator_count,
                max_cpu_millicores: grant.cpu_millicores,
                max_memory_bytes: grant.memory_bytes,
                max_vram_bytes: grant.vram_bytes,
                max_disk_bytes: grant.disk_bytes,
                max_runtime_seconds: grant.max_runtime_seconds,
                allow_network_egress: grant.allow_network_egress,
            },
            usage_limits: grant.usage_limits.clone(),
            checkpoint_policy: ComputeAttemptCheckpointPolicy {
                mode: workload.checkpoint_policy.mode.clone(),
                interval_seconds: workload.checkpoint_policy.interval_seconds,
                maximum_checkpoints: workload.checkpoint_policy.max_checkpoints,
                checkpoint_media_type: workload.checkpoint_policy.checkpoint_media_type.clone(),
            },
            workload_deadline_at: workload.deadline_at.clone(),
        },
        latest_checkpoint: None,
        lease_expires_at: plan.start.lease_expires_at.clone(),
        hard_deadline_at: plan.start.hard_deadline_at.clone(),
    })
}

pub(super) fn exact_replay_receipt(
    stored: StoredPlan,
    candidate: &ComputeAttemptExecutionPlanEnvelope,
) -> Result<ComputeAttemptExecutionPlanReceipt> {
    if &stored.plan != candidate {
        bail!("Execution plan replay conflicts with the immutable stored plan");
    }
    Ok(ComputeAttemptExecutionPlanReceipt::new(
        stored.plan,
        stored.seal,
        true,
    ))
}
