use anyhow::{bail, Result};
use chrono::{DateTime, Duration, SecondsFormat};

use super::{canonical::*, types::*};

pub(crate) fn validate_sandbox_conformance_draft(
    draft: &ExternalPoolAdapterSandboxConformanceDraft,
) -> Result<()> {
    for value in [&draft.verifier_report_id, &draft.sandbox_runtime_id] {
        identifier(value, 200)?;
    }
    digest(&draft.runtime_image_digest)?;
    if draft.isolation_profile_id != SANDBOX_CONFORMANCE_ISOLATION_PROFILE_ID {
        bail!("sandbox conformance isolation profile is unsupported");
    }
    let started = canonical_nanos(&draft.run_started_at)?;
    let completed = canonical_nanos(&draft.run_completed_at)?;
    let generated = canonical_nanos(&draft.report_generated_at)?;
    let expires = canonical_nanos(&draft.report_expires_at)?;
    if completed < started
        || completed - started > Duration::minutes(MAX_SANDBOX_RUN_MINUTES)
        || generated < completed
        || expires <= generated
        || expires - generated > Duration::hours(MAX_SANDBOX_REPORT_VALIDITY_HOURS)
        || draft.external_network_attempt_count != 0
        || draft.write_outside_ephemeral_count != 0
        || draft.child_process_attempt_count != 0
        || draft.peak_memory_bytes == 0
        || draft.peak_memory_bytes > MAX_SANDBOX_PEAK_MEMORY_BYTES
        || draft.cpu_time_ms == 0
        || draft.cpu_time_ms > MAX_SANDBOX_CPU_TIME_MS
    {
        bail!("sandbox conformance runtime/report window is invalid");
    }
    validate_observations(&draft.observations)
}

pub(crate) fn validate_sandbox_conformance_binding(
    binding: &ExternalPoolAdapterSandboxConformanceBinding,
) -> Result<()> {
    if binding.schema != SANDBOX_CONFORMANCE_BINDING_SCHEMA
        || binding.signature_algorithm != SANDBOX_CONFORMANCE_SIGNATURE_ALGORITHM
        || binding.sandbox_policy_id != SANDBOX_CONFORMANCE_POLICY_ID
        || binding.isolation_profile_id != SANDBOX_CONFORMANCE_ISOLATION_PROFILE_ID
        || binding.supported_capabilities.len() != REQUIRED_SANDBOX_CAPABILITY_COUNT
        || binding.test_plan.len() != REQUIRED_SANDBOX_CAPABILITY_COUNT
        || binding.passed_capability_count != REQUIRED_SANDBOX_CAPABILITY_COUNT as u64
        || binding.policy_violation_count != 0
    {
        bail!("sandbox conformance binding policy is invalid");
    }
    for value in [
        &binding.admission_id,
        &binding.adapter_id,
        &binding.release_version,
        &binding.vulnerability_report_receipt_id,
        &binding.sandbox_verifier_key_record_id,
        &binding.sandbox_verifier_key_id,
        &binding.sandbox_verifier_operator,
        &binding.sandbox_verifier_product,
    ] {
        identifier(value, 200)?;
    }
    for value in [
        &binding.admission_digest,
        &binding.declared_implementation_sha256,
        &binding.capability_set_digest,
        &binding.expected_credential_verifier.verifier_digest,
        &binding.vulnerability_report_receipt_digest,
        &binding.security_receipt_digest,
        &binding.package_receipt_digest,
        &binding.archive_sha256,
        &binding.sbom_digest,
        &binding.sandbox_verifier_key_record_digest,
        &binding.runtime_image_digest,
        &binding.test_plan_digest,
        &binding.observation_inventory_digest,
    ] {
        digest(value)?;
    }
    let expected_plan =
        sandbox_capability_test_plan(&binding.admission_digest, &binding.supported_capabilities)?;
    if binding.test_plan != expected_plan
        || sandbox_test_plan_digest(&binding.test_plan)? != binding.test_plan_digest
        || sandbox_observation_inventory_digest(&binding.observations)?
            != binding.observation_inventory_digest
    {
        bail!("sandbox conformance test plan or observation digest is invalid");
    }
    validate_sandbox_conformance_draft(&ExternalPoolAdapterSandboxConformanceDraft {
        verifier_report_id: binding.verifier_report_id.clone(),
        sandbox_runtime_id: binding.sandbox_runtime_id.clone(),
        runtime_image_digest: binding.runtime_image_digest.clone(),
        isolation_profile_id: binding.isolation_profile_id.clone(),
        run_started_at: binding.run_started_at.clone(),
        run_completed_at: binding.run_completed_at.clone(),
        report_generated_at: binding.report_generated_at.clone(),
        report_expires_at: binding.report_expires_at.clone(),
        external_network_attempt_count: binding.external_network_attempt_count,
        write_outside_ephemeral_count: binding.write_outside_ephemeral_count,
        child_process_attempt_count: binding.child_process_attempt_count,
        peak_memory_bytes: binding.peak_memory_bytes,
        cpu_time_ms: binding.cpu_time_ms,
        observations: binding.observations.clone(),
    })?;
    for ((capability, test), observation) in binding
        .supported_capabilities
        .iter()
        .zip(&binding.test_plan)
        .zip(&binding.observations)
    {
        if capability.capability_id != test.capability_id
            || capability.capability_revision != test.capability_revision
            || observation.capability_id != test.capability_id
            || observation.capability_revision != test.capability_revision
            || observation.test_case_id != test.test_case_id
        {
            bail!("sandbox conformance observation does not match the server-derived test plan");
        }
    }
    let intelligence_expires = canonical_nanos(&binding.vulnerability_intelligence_expires_at)?;
    let vulnerability_verified = canonical_nanos(&binding.vulnerability_report_verified_at)?;
    let run_started = canonical_nanos(&binding.run_started_at)?;
    let report_expires = canonical_nanos(&binding.report_expires_at)?;
    if run_started < vulnerability_verified || report_expires > intelligence_expires {
        bail!("sandbox conformance cannot outlive its vulnerability intelligence");
    }
    Ok(())
}

pub(crate) fn validate_sandbox_conformance_receipt(
    receipt: &ExternalPoolAdapterSandboxConformanceReceipt,
) -> Result<()> {
    if receipt.schema != SANDBOX_CONFORMANCE_RECEIPT_SCHEMA
        || receipt.canonicalization != SANDBOX_CONFORMANCE_CANONICALIZATION
        || receipt.digest_algorithm != SANDBOX_CONFORMANCE_DIGEST_ALGORITHM
    {
        bail!("sandbox conformance receipt metadata is unsupported");
    }
    identifier(&receipt.sandbox_conformance_receipt_id, 200)?;
    digest(&receipt.sandbox_conformance_receipt_digest)?;
    digest(&receipt.conformance_material_digest)?;
    validate_sandbox_conformance_binding(&receipt.conformance.binding)?;
    let item = &receipt.conformance;
    for value in [&item.signature_message_digest, &item.signature_digest] {
        digest(value)?;
    }
    for value in [
        &item.verified_by_admin_user_id,
        &item.idempotency_scope,
        &item.idempotency_key,
    ] {
        identifier(value, 240)?;
    }
    let verified = canonical_nanos(&item.verified_at)?;
    canonical_nanos(&item.recorded_at)?;
    let generated = canonical_nanos(&item.binding.report_generated_at)?;
    let expires = canonical_nanos(&item.binding.report_expires_at)?;
    if item.confirmation != SANDBOX_CONFORMANCE_CONFIRMATION
        || item.recorded_at != item.verified_at
        || item.evidence_scope != SANDBOX_CONFORMANCE_EVIDENCE_SCOPE
        || item.conformance_effect != SANDBOX_CONFORMANCE_EFFECT
        || item.credential_effect != SANDBOX_CONFORMANCE_NO_EFFECT
        || item.adapter_effect != SANDBOX_CONFORMANCE_NO_EFFECT
        || item.route_effect != SANDBOX_CONFORMANCE_NO_EFFECT
        || verified < generated
        || verified > expires
        || sandbox_conformance_material_digest(item)? != receipt.conformance_material_digest
        || canonical_sandbox_conformance_receipt_json_and_digest(receipt)?.1
            != receipt.sandbox_conformance_receipt_digest
    {
        bail!("sandbox conformance receipt material is not exact");
    }
    Ok(())
}

fn validate_observations(
    observations: &[ExternalPoolAdapterSandboxCapabilityObservation],
) -> Result<()> {
    if observations.len() != REQUIRED_SANDBOX_CAPABILITY_COUNT {
        bail!("sandbox conformance requires exactly six capability observations");
    }
    for observation in observations {
        identifier(&observation.capability_id, 80)?;
        identifier(&observation.test_case_id, 160)?;
        digest(&observation.output_transcript_digest)?;
        if observation.capability_revision < 1
            || observation.outcome != "passed"
            || !(1..=300_000).contains(&observation.duration_ms)
            || observation.policy_violation_count != 0
        {
            bail!("sandbox capability observation did not satisfy the pass policy");
        }
    }
    Ok(())
}

fn identifier(value: &str, max: usize) -> Result<()> {
    if value.trim() != value
        || value.is_empty()
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("sandbox conformance identifier is invalid");
    }
    Ok(())
}

fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("sandbox conformance digest is invalid");
    }
    Ok(())
}

fn canonical_nanos(value: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("sandbox conformance timestamp is not canonical UTC nanoseconds");
    }
    Ok(parsed)
}
