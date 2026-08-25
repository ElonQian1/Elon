use anyhow::{ensure, Result};
use chrono::{DateTime, Duration, SecondsFormat};

use super::{
    canonical::{
        canonical_compute_user_node_ready_source_lineage_json_and_digest,
        canonical_untrusted_host_runtime_observation_digest,
    },
    types::*,
};

const IJSON_MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
const MAX_READY_SOURCE_LIFETIME_SECONDS: i64 = 5 * 60;

pub(crate) fn validate_compute_user_node_ready_source_lineage(
    envelope: &UntrustedComputeUserNodeReadySourceLineageEnvelopeV1,
) -> Result<()> {
    ensure!(
        envelope.schema == COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_SCHEMA
            && envelope.lineage_kind == COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_KIND
            && envelope.canonicalization == COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_CANONICALIZATION
            && envelope.digest_algorithm == COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_DIGEST_ALGORITHM,
        "user-node Ready source lineage metadata is unsupported"
    );
    digest(&envelope.lineage_digest, "lineage digest")?;
    validate_lineage(&envelope.lineage)?;
    let (_, computed_digest) =
        canonical_compute_user_node_ready_source_lineage_json_and_digest(envelope)?;
    ensure!(
        envelope.lineage_digest == computed_digest,
        "user-node Ready source lineage digest does not match its canonical projection"
    );
    Ok(())
}

pub(crate) fn validate_untrusted_compute_user_node_host_runtime_observation(
    observation: &UntrustedComputeUserNodeHostRuntimeObservationV1,
) -> Result<()> {
    ensure!(
        observation.schema == UNTRUSTED_COMPUTE_USER_NODE_HOST_RUNTIME_OBSERVATION_SCHEMA,
        "untrusted Host runtime observation schema is unsupported"
    );
    for (value, label) in [
        (&observation.runner_digest, "Host runner digest"),
        (&observation.runtime_digest, "Host runtime digest"),
        (
            &observation.host_enforcement_digest,
            "Host enforcement digest",
        ),
        (
            &observation.resource_profile_digest,
            "Host resource-profile digest",
        ),
        (&observation.observation_digest, "Host observation digest"),
    ] {
        digest(value, label)?;
    }
    for (value, label) in [
        (&observation.executor_id, "Host executor ID"),
        (&observation.runner_id, "Host runner ID"),
        (
            &observation.host_enforcement_ref,
            "Host enforcement reference",
        ),
    ] {
        identifier(value, label)?;
    }
    sorted_identifiers(&observation.task_kinds, "Host task kinds", false)?;
    sorted_identifiers(
        &observation.supported_precisions,
        "Host supported precisions",
        observation.model_bindings.is_empty(),
    )?;
    validate_model_bindings(&observation.model_bindings)?;
    ensure!(
        observation.model_bindings.is_empty() == observation.supported_precisions.is_empty(),
        "Host model and precision observations must be jointly empty or jointly present"
    );
    validate_host_resources(observation)?;
    let observed_at = canonical_utc_millis(&observation.observed_at, "Host observed_at")?;
    let expires_at = canonical_utc_millis(&observation.expires_at, "Host expires_at")?;
    ensure!(
        observed_at < expires_at
            && expires_at - observed_at <= Duration::seconds(MAX_READY_SOURCE_LIFETIME_SECONDS),
        "untrusted Host runtime observation lifetime is invalid"
    );
    ensure!(
        canonical_untrusted_host_runtime_observation_digest(observation)?
            == observation.observation_digest,
        "untrusted Host runtime observation digest mismatch"
    );
    Ok(())
}

fn validate_lineage(lineage: &ComputeUserNodeReadySourceLineageV1) -> Result<()> {
    ensure!(
        lineage.projection_status == COMPUTE_USER_NODE_READY_SOURCE_PROJECTION_STATUS,
        "user-node Ready source projection status is not failure-closed"
    );
    validate_work_admission(&lineage.work_admission)?;
    validate_ready_health(&lineage.ready_health)?;
    validate_untrusted_compute_user_node_host_runtime_observation(
        &lineage.host_runtime_observation,
    )?;
    validate_cross_source_equations(lineage)?;
    ensure!(
        lineage.authority_gaps.node_local_authority_currentness
            == COMPUTE_USER_NODE_READY_SOURCE_AUTHORITY_MISSING
            && lineage.authority_gaps.runtime_transition_authority
                == COMPUTE_USER_NODE_READY_SOURCE_AUTHORITY_MISSING
            && lineage.authority_gaps.host_runtime_authority
                == COMPUTE_USER_NODE_READY_SOURCE_AUTHORITY_MISSING
            && lineage.authority_gaps.v15_authenticated_session
                == COMPUTE_USER_NODE_READY_SOURCE_AUTHORITY_MISSING,
        "user-node Ready source authority gaps must remain explicit"
    );
    let effects = &lineage.effects;
    ensure!(
        effects.projection_effect == COMPUTE_USER_NODE_READY_SOURCE_PROJECTION_EFFECT
            && [
                effects.readiness_effect.as_str(),
                effects.provider_effect.as_str(),
                effects.route_effect.as_str(),
                effects.offer_effect.as_str(),
                effects.capacity_effect.as_str(),
                effects.execution_effect.as_str(),
                effects.lease_effect.as_str(),
                effects.settlement_effect.as_str(),
                effects.money_effect.as_str(),
            ]
            .into_iter()
            .all(|effect| effect == COMPUTE_USER_NODE_READY_SOURCE_NO_EFFECT),
        "user-node Ready source projection effects are not inert"
    );
    Ok(())
}

fn validate_work_admission(value: &ComputeUserNodeReadyWorkAdmissionSourceRefV1) -> Result<()> {
    ensure!(
        value.source_schema == COMPUTE_USER_NODE_READY_WORK_ADMISSION_SOURCE_SCHEMA
            && value.receipt_schema == COMPUTE_USER_NODE_READY_WORK_ADMISSION_RECEIPT_SCHEMA,
        "user-node Ready work-admission source schemas are unsupported"
    );
    for (candidate, label) in [
        (&value.plugin_id, "work-admission plugin ID"),
        (&value.slot_ref, "work-admission slot"),
        (&value.work_admission_id, "work-admission ID"),
        (&value.install_receipt_id, "install receipt ID"),
        (&value.promotion_receipt_id, "promotion receipt ID"),
        (&value.plan_id, "work-admission plan ID"),
        (&value.grant_ref, "work-admission grant reference"),
    ] {
        identifier(candidate, label)?;
    }
    for (candidate, label) in [
        (&value.source_digest, "work-admission source digest"),
        (&value.receipt_digest, "work-admission receipt digest"),
        (
            &value.clock_epoch_digest,
            "work-admission clock-epoch digest",
        ),
        (
            &value.installation_identity_digest,
            "work-admission installation digest",
        ),
        (&value.install_receipt_digest, "install receipt digest"),
        (&value.promotion_receipt_digest, "promotion receipt digest"),
        (&value.plan_digest, "work-admission plan digest"),
        (
            &value.application_receipt_digest,
            "PlanApply receipt digest",
        ),
        (&value.grant_digest, "work-admission grant digest"),
        (&value.inventory_digest, "work-admission inventory digest"),
        (&value.runner_digest, "work-admission runner digest"),
    ] {
        digest(candidate, label)?;
    }
    validate_release(&value.release)?;
    ensure!(
        value.release.plugin_id == value.plugin_id,
        "work-admission release does not name the same plugin"
    );
    for (candidate, label) in [
        (value.install_generation, "install generation"),
        (value.activation_generation, "activation generation"),
        (value.admitted_at_ms, "work-admission admitted time"),
        (value.plan_policy_revision, "work-admission policy revision"),
        (value.work_admission_generation, "work-admission generation"),
        (
            value.inventory_revision,
            "work-admission inventory revision",
        ),
        (
            value.authority_state_revision,
            "work-admission authority state revision",
        ),
        (value.authority_epoch, "work-admission authority epoch"),
        (
            value.process_owner_epoch,
            "work-admission process-owner epoch",
        ),
    ] {
        positive(candidate, label)?;
    }
    nonnegative(
        value.runtime_generation_before_ready,
        "pre-Ready runtime generation",
    )?;
    if let Some(kind) = &value.target_accelerator_kind {
        identifier(kind, "target accelerator kind")?;
    }
    sorted_identifiers(&value.task_kinds, "work-admission task kinds", false)?;
    validate_granted_resources(&value.granted_resources)
}

fn validate_ready_health(value: &ComputeUserNodeReadyHealthSourceRefV1) -> Result<()> {
    for (candidate, label) in [
        (&value.plugin_id, "ready-health plugin ID"),
        (&value.last_plan_id, "ready-health Plan ID"),
        (&value.slot_ref, "ready-health slot"),
        (
            &value.trusted_time.time_authority_id,
            "trusted-time authority ID",
        ),
    ] {
        identifier(candidate, label)?;
    }
    for (candidate, label) in [
        (
            &value.installation_identity_digest,
            "ready-health installation digest",
        ),
        (
            &value.permission_grant_digest,
            "ready-health permission-grant digest",
        ),
        (&value.runner_digest, "ready-health runner digest"),
        (
            &value.health_observation_digest,
            "ready-health observation digest",
        ),
        (
            &value.trusted_time.clock_epoch_digest,
            "trusted-time clock-epoch digest",
        ),
        (
            &value.trusted_time.attestation_digest,
            "trusted-time attestation digest",
        ),
        (
            &value.trusted_time.signing_key_fingerprint,
            "trusted-time signing-key fingerprint",
        ),
    ] {
        digest(candidate, label)?;
    }
    validate_release(&value.release)?;
    ensure!(
        value.release.plugin_id == value.plugin_id,
        "ready-health release does not name the same plugin"
    );
    for (candidate, label) in [
        (value.inventory_revision, "ready-health inventory revision"),
        (
            value.desired_policy_revision,
            "ready-health policy revision",
        ),
        (value.install_generation, "ready-health install generation"),
        (
            value.activation_generation,
            "ready-health activation generation",
        ),
        (value.runtime_generation, "ready-health runtime generation"),
        (
            value.trusted_time.attestation_sequence,
            "trusted-time attestation sequence",
        ),
    ] {
        positive(candidate, label)?;
    }
    sorted_identifiers(
        &value.health_reason_codes,
        "ready-health reason codes",
        true,
    )?;
    let observed_at = canonical_utc_millis(&value.health_observed_at, "health observed_at")?;
    let expires_at = canonical_utc_millis(&value.health_expires_at, "health expires_at")?;
    let trusted_now = canonical_utc_millis(&value.trusted_time.trusted_now, "trusted now")?;
    ensure!(
        observed_at <= trusted_now
            && trusted_now < expires_at
            && expires_at - observed_at <= Duration::seconds(MAX_READY_SOURCE_LIFETIME_SECONDS),
        "ready-health trusted-time interval is invalid"
    );
    Ok(())
}

fn validate_cross_source_equations(lineage: &ComputeUserNodeReadySourceLineageV1) -> Result<()> {
    let work = &lineage.work_admission;
    let ready = &lineage.ready_health;
    let host = &lineage.host_runtime_observation;
    ensure!(
        work.installation_identity_digest == ready.installation_identity_digest
            && work.plugin_id == ready.plugin_id
            && work.plan_id == ready.last_plan_id
            && work.plan_policy_revision == ready.desired_policy_revision
            && work.clock_epoch_digest == ready.trusted_time.clock_epoch_digest
            && work.slot_ref == ready.slot_ref
            && work.release == ready.release
            && work.install_generation == ready.install_generation
            && work.activation_generation == ready.activation_generation
            && work.grant_digest == ready.permission_grant_digest
            && work.runner_digest == ready.runner_digest
            && ready.runtime_generation > work.runtime_generation_before_ready
            && ready.inventory_revision > work.inventory_revision,
        "user-node Ready work-admission and ready-health sources diverge"
    );
    ensure!(
        host.runner_digest == work.runner_digest && host.task_kinds == work.task_kinds,
        "untrusted Host runtime observation diverges from admitted runner or task kinds"
    );
    let observed_at = canonical_utc_millis(&host.observed_at, "Host observed_at")?;
    let expires_at = canonical_utc_millis(&host.expires_at, "Host expires_at")?;
    let health_observed_at = canonical_utc_millis(&ready.health_observed_at, "health observed_at")?;
    let health_expires_at = canonical_utc_millis(&ready.health_expires_at, "health expires_at")?;
    ensure!(
        health_observed_at.timestamp_millis() > work.admitted_at_ms
            && observed_at <= health_observed_at
            && expires_at >= health_expires_at,
        "untrusted Host observation does not cover the ready-health interval"
    );
    validate_host_within_grant(host, work)
}

fn validate_release(value: &ComputeUserNodeReadyPluginReleaseRefV1) -> Result<()> {
    for (candidate, label) in [
        (&value.plugin_id, "release plugin ID"),
        (&value.plugin_version, "release plugin version"),
        (&value.target_id, "release target ID"),
    ] {
        identifier(candidate, label)?;
    }
    digest(&value.manifest_digest, "release manifest digest")?;
    digest(&value.package_digest, "release package digest")
}

fn validate_granted_resources(value: &ComputeUserNodeReadyGrantedResourceCeilingV1) -> Result<()> {
    for (candidate, label) in [
        (value.max_cpu_millicores, "granted CPU"),
        (value.max_memory_bytes, "granted memory"),
        (value.max_disk_bytes, "granted disk"),
        (value.max_processes, "granted processes"),
        (value.max_sidecar_uptime_seconds, "granted Sidecar uptime"),
    ] {
        positive(candidate, label)?;
    }
    nonnegative(value.max_vram_bytes, "granted VRAM")
}

fn validate_host_resources(
    observation: &UntrustedComputeUserNodeHostRuntimeObservationV1,
) -> Result<()> {
    let resources = &observation.resources;
    nonnegative(resources.accelerator_count, "observed accelerator count")?;
    nonnegative(resources.vram_bytes, "observed VRAM")?;
    for (candidate, label) in [
        (resources.cpu_millicores, "observed CPU"),
        (resources.memory_bytes, "observed memory"),
        (resources.disk_bytes, "observed disk"),
        (resources.process_count, "observed process count"),
        (
            observation.technical_concurrency_limit,
            "technical concurrency limit",
        ),
    ] {
        positive(candidate, label)?;
    }
    Ok(())
}

fn validate_host_within_grant(
    host: &UntrustedComputeUserNodeHostRuntimeObservationV1,
    work: &ComputeUserNodeReadyWorkAdmissionSourceRefV1,
) -> Result<()> {
    let observed = &host.resources;
    let granted = &work.granted_resources;
    ensure!(
        observed.cpu_millicores <= granted.max_cpu_millicores
            && observed.memory_bytes <= granted.max_memory_bytes
            && observed.vram_bytes <= granted.max_vram_bytes
            && observed.disk_bytes <= granted.max_disk_bytes
            && observed.process_count <= granted.max_processes
            && host.technical_concurrency_limit <= granted.max_processes,
        "untrusted Host runtime observation exceeds the signed work-admission grant"
    );
    match &work.target_accelerator_kind {
        None => ensure!(
            observed.accelerator_count == 0 && observed.vram_bytes == 0,
            "CPU-only target cannot invent accelerator capacity"
        ),
        Some(_) => ensure!(
            observed.accelerator_count > 0,
            "accelerator target must explicitly observe a positive accelerator count"
        ),
    };
    Ok(())
}

fn validate_model_bindings(values: &[ComputeUserNodeReadyModelBindingV1]) -> Result<()> {
    let mut previous = None;
    for value in values {
        identifier(&value.model_id, "Host model ID")?;
        digest(&value.model_digest, "Host model digest")?;
        if let Some(tokenizer_digest) = &value.tokenizer_digest {
            digest(tokenizer_digest, "Host tokenizer digest")?;
        }
        ensure!(
            previous.is_none_or(|prior| prior < value.model_id.as_str()),
            "Host model bindings must be strictly sorted and unique"
        );
        previous = Some(value.model_id.as_str());
    }
    Ok(())
}

fn sorted_identifiers(values: &[String], label: &str, allow_empty: bool) -> Result<()> {
    ensure!(
        allow_empty || !values.is_empty(),
        "user-node Ready source {label} cannot be empty"
    );
    let mut previous = None;
    for value in values {
        identifier(value, label)?;
        ensure!(
            previous.is_none_or(|prior| prior < value.as_str()),
            "user-node Ready source {label} must be strictly sorted and unique"
        );
        previous = Some(value.as_str());
    }
    Ok(())
}

fn identifier(value: &str, label: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value.trim() == value
            && value.chars().count() <= 200
            && !value.chars().any(char::is_control),
        "user-node Ready source {label} is invalid"
    );
    Ok(())
}

fn digest(value: &str, label: &str) -> Result<()> {
    ensure!(
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "user-node Ready source {label} is invalid"
    );
    Ok(())
}

fn positive(value: i64, label: &str) -> Result<()> {
    ensure!(
        (1..=IJSON_MAX_SAFE_INTEGER).contains(&value),
        "user-node Ready source {label} must be an I-JSON safe positive integer"
    );
    Ok(())
}

fn nonnegative(value: i64, label: &str) -> Result<()> {
    ensure!(
        (0..=IJSON_MAX_SAFE_INTEGER).contains(&value),
        "user-node Ready source {label} must be an I-JSON safe non-negative integer"
    );
    Ok(())
}

fn canonical_utc_millis(value: &str, label: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    ensure!(
        parsed.to_rfc3339_opts(SecondsFormat::Millis, true) == value,
        "user-node Ready source {label} is not canonical UTC milliseconds"
    );
    Ok(parsed)
}
