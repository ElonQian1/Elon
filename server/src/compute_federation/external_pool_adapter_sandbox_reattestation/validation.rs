use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Duration, SecondsFormat};
use sha2::{Digest, Sha256};

use crate::compute_federation::external_pool_adapter_artifact_sandbox_conformance::{
    sandbox_capability_test_plan, sandbox_observation_inventory_digest, sandbox_test_plan_digest,
    validate_sandbox_conformance_draft, ExternalPoolAdapterSandboxConformanceDraft,
    MAX_SANDBOX_REPORT_VALIDITY_HOURS, REQUIRED_SANDBOX_CAPABILITY_COUNT,
    SANDBOX_CONFORMANCE_ISOLATION_PROFILE_ID,
};

use super::{canonical::*, types::*};

pub(crate) fn validate_sandbox_reattestation_binding(
    binding: &ExternalPoolAdapterSandboxReattestationBinding,
) -> Result<()> {
    if binding.schema != SANDBOX_REATTESTATION_BINDING_SCHEMA
        || binding.signature_algorithm != SANDBOX_REATTESTATION_SIGNATURE_ALGORITHM
        || binding.sandbox_policy_id != SANDBOX_REATTESTATION_POLICY_ID
        || binding.isolation_profile_id != SANDBOX_CONFORMANCE_ISOLATION_PROFILE_ID
        || binding.sequence == 0
        || binding.vulnerability_reattestation_sequence == 0
        || binding.supported_capabilities.len() != REQUIRED_SANDBOX_CAPABILITY_COUNT
        || binding.test_plan.len() != REQUIRED_SANDBOX_CAPABILITY_COUNT
        || binding.passed_capability_count != REQUIRED_SANDBOX_CAPABILITY_COUNT as u64
        || binding.policy_violation_count != 0
        || binding.predecessor_receipt_id.is_some() != binding.predecessor_receipt_digest.is_some()
        || (binding.sequence == 1) != binding.predecessor_receipt_id.is_none()
    {
        bail!("sandbox re-attestation binding policy is invalid");
    }
    for value in identifiers(binding) {
        identifier(value, 240)?;
    }
    for value in digests(binding) {
        digest(value)?;
    }
    if let Some(value) = binding.predecessor_receipt_id.as_deref() {
        identifier(value, 200)?;
    }
    if let Some(value) = binding.predecessor_receipt_digest.as_deref() {
        digest(value)?;
    }
    let nonce = STANDARD.decode(&binding.challenge_nonce_base64)?;
    let issued = canonical_nanos(&binding.challenge_issued_at)?;
    let challenge_expires = canonical_nanos(&binding.challenge_expires_at)?;
    if nonce.len() != 32
        || STANDARD.encode(&nonce) != binding.challenge_nonce_base64
        || hex::encode(Sha256::digest(&nonce)) != binding.challenge_nonce_digest
        || challenge_expires - issued
            != Duration::minutes(SANDBOX_REATTESTATION_CHALLENGE_VALIDITY_MINUTES)
    {
        bail!("sandbox re-attestation challenge material is invalid");
    }
    let draft = draft(binding);
    validate_sandbox_conformance_draft(&draft)?;
    let plan =
        sandbox_capability_test_plan(&binding.admission_digest, &binding.supported_capabilities)?;
    if plan != binding.test_plan
        || sandbox_test_plan_digest(&plan)? != binding.test_plan_digest
        || sandbox_observation_inventory_digest(&binding.observations)?
            != binding.observation_inventory_digest
    {
        bail!("sandbox re-attestation plan or observations are not exact");
    }
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
            bail!("sandbox re-attestation observation does not match the server-derived test plan");
        }
    }
    let vulnerability_verified = canonical_nanos(&binding.vulnerability_reattestation_verified_at)?;
    let vulnerability_expires = canonical_nanos(&binding.vulnerability_intelligence_expires_at)?;
    let run_started = canonical_nanos(&binding.run_started_at)?;
    let report_expires = canonical_nanos(&binding.report_expires_at)?;
    if run_started < vulnerability_verified
        || report_expires > vulnerability_expires
        || report_expires - canonical_nanos(&binding.report_generated_at)?
            > Duration::hours(MAX_SANDBOX_REPORT_VALIDITY_HOURS)
    {
        bail!("sandbox re-attestation cannot outlive its vulnerability authority");
    }
    Ok(())
}

pub(crate) fn validate_sandbox_reattestation_receipt(
    receipt: &ExternalPoolAdapterSandboxReattestationReceipt,
) -> Result<()> {
    if receipt.schema != SANDBOX_REATTESTATION_RECEIPT_SCHEMA
        || receipt.canonicalization != SANDBOX_REATTESTATION_CANONICALIZATION
        || receipt.digest_algorithm != SANDBOX_REATTESTATION_DIGEST_ALGORITHM
    {
        bail!("sandbox re-attestation receipt metadata is unsupported");
    }
    identifier(&receipt.reattestation_receipt_id, 200)?;
    digest(&receipt.reattestation_receipt_digest)?;
    digest(&receipt.reattestation_material_digest)?;
    validate_sandbox_reattestation_binding(&receipt.reattestation.binding)?;
    let item = &receipt.reattestation;
    digest(&item.signature_message_digest)?;
    digest(&item.signature_digest)?;
    identifier(&item.recorded_by_admin_user_id, 200)?;
    identifier(&item.idempotency_scope, 240)?;
    identifier(&item.idempotency_key, 240)?;
    let signature = STANDARD.decode(&item.signature_base64)?;
    let verified = canonical_nanos(&item.verified_at)?;
    let issued = canonical_nanos(&item.binding.challenge_issued_at)?;
    let challenge_expires = canonical_nanos(&item.binding.challenge_expires_at)?;
    let report_generated = canonical_nanos(&item.binding.report_generated_at)?;
    let report_expires = canonical_nanos(&item.binding.report_expires_at)?;
    if signature.is_empty()
        || signature.len() > 1024
        || STANDARD.encode(&signature) != item.signature_base64
        || hex::encode(Sha256::digest(&signature)) != item.signature_digest
        || item.confirmation != SANDBOX_REATTESTATION_CONFIRMATION
        || item.recorded_at != item.verified_at
        || verified < issued
        || verified < report_generated
        || verified >= challenge_expires
        || verified >= report_expires
        || item.evidence_scope != SANDBOX_REATTESTATION_EVIDENCE_SCOPE
        || item.sandbox_reattestation_effect != SANDBOX_REATTESTATION_EFFECT
        || !no_effects([
            &item.adapter_effect,
            &item.provider_effect,
            &item.credential_effect,
            &item.route_effect,
            &item.execution_effect,
            &item.settlement_effect,
        ])
        || sandbox_reattestation_material_digest(item)? != receipt.reattestation_material_digest
        || sandbox_reattestation_receipt_json_and_digest(receipt)?.1
            != receipt.reattestation_receipt_digest
    {
        bail!("sandbox re-attestation receipt is not exact");
    }
    Ok(())
}

pub(crate) fn validate_sandbox_reattestation_revocation_receipt(
    receipt: &ExternalPoolAdapterSandboxReattestationRevocationReceipt,
) -> Result<()> {
    if receipt.schema != SANDBOX_REATTESTATION_REVOCATION_RECEIPT_SCHEMA
        || receipt.canonicalization != SANDBOX_REATTESTATION_CANONICALIZATION
        || receipt.digest_algorithm != SANDBOX_REATTESTATION_DIGEST_ALGORITHM
    {
        bail!("sandbox re-attestation revocation metadata is unsupported");
    }
    identifier(&receipt.revocation_receipt_id, 200)?;
    digest(&receipt.revocation_receipt_digest)?;
    digest(&receipt.revocation_material_digest)?;
    let item = &receipt.revocation;
    identifier(&item.reattestation_receipt_id, 200)?;
    digest(&item.reattestation_receipt_digest)?;
    identifier(&item.registry_release_id, 200)?;
    digest(&item.registry_release_digest)?;
    identifier(&item.revoked_by_admin_user_id, 200)?;
    identifier(&item.idempotency_scope, 240)?;
    identifier(&item.idempotency_key, 240)?;
    if item.reason.trim() != item.reason || !(12..=500).contains(&item.reason.chars().count()) {
        bail!("sandbox re-attestation revocation reason is invalid");
    }
    canonical_nanos(&item.revoked_at)?;
    if item.revoked_at != item.recorded_at
        || item.confirmation != SANDBOX_REATTESTATION_REVOCATION_CONFIRMATION
        || item.revocation_effect != SANDBOX_REATTESTATION_REVOCATION_EFFECT
        || !no_effects([
            &item.adapter_effect,
            &item.provider_effect,
            &item.credential_effect,
            &item.route_effect,
            &item.execution_effect,
            &item.settlement_effect,
        ])
        || sandbox_reattestation_revocation_material_digest(item)?
            != receipt.revocation_material_digest
        || sandbox_reattestation_revocation_receipt_json_and_digest(receipt)?.1
            != receipt.revocation_receipt_digest
    {
        bail!("sandbox re-attestation revocation is not exact");
    }
    Ok(())
}

fn draft(
    binding: &ExternalPoolAdapterSandboxReattestationBinding,
) -> ExternalPoolAdapterSandboxConformanceDraft {
    ExternalPoolAdapterSandboxConformanceDraft {
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
    }
}

fn identifiers(binding: &ExternalPoolAdapterSandboxReattestationBinding) -> [&str; 18] {
    [
        &binding.challenge_id,
        &binding.registry_release_id,
        &binding.admission_id,
        &binding.package_receipt_id,
        &binding.source_receipt_id,
        &binding.adapter_id,
        &binding.release_version,
        &binding.vulnerability_reattestation_receipt_id,
        &binding.security_receipt_id,
        &binding.sandbox_verifier_key_record_id,
        &binding.sandbox_verifier_key_id,
        &binding.sandbox_verifier_operator,
        &binding.sandbox_verifier_product,
        &binding.verifier_report_id,
        &binding.sandbox_runtime_id,
        &binding.route_kind,
        &binding.sandbox_policy_id,
        &binding.isolation_profile_id,
    ]
}

fn digests(binding: &ExternalPoolAdapterSandboxReattestationBinding) -> [&str; 28] {
    [
        &binding.challenge_nonce_digest,
        &binding.registry_release_digest,
        &binding.registry_release_material_digest,
        &binding.admission_digest,
        &binding.package_receipt_digest,
        &binding.source_receipt_digest,
        &binding.implementation_digest,
        &binding.declared_implementation_sha256,
        &binding.capability_set_digest,
        &binding.expected_credential_verifier.verifier_digest,
        &binding.credential_verifier_digest,
        &binding.archive_sha256,
        &binding.manifest_digest,
        &binding.entry_inventory_digest,
        &binding.installation_content_digest,
        &binding.vulnerability_reattestation_receipt_digest,
        &binding.vulnerability_reattestation_material_digest,
        &binding.vulnerability_intelligence_snapshot_digest,
        &binding.security_receipt_digest,
        &binding.security_material_digest,
        &binding.sbom_digest,
        &binding.component_inventory_digest,
        &binding.dependency_inventory_digest,
        &binding.sandbox_verifier_key_record_digest,
        &binding.sandbox_verifier_key_id,
        &binding.runtime_image_digest,
        &binding.test_plan_digest,
        &binding.observation_inventory_digest,
    ]
}

fn no_effects<const N: usize>(values: [&String; N]) -> bool {
    values
        .into_iter()
        .all(|value| value == SANDBOX_REATTESTATION_NO_EFFECT)
}

fn identifier(value: &str, max: usize) -> Result<()> {
    if value.trim() != value
        || value.is_empty()
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("sandbox re-attestation identifier is invalid");
    }
    Ok(())
}

fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("sandbox re-attestation digest is invalid");
    }
    Ok(())
}

pub(crate) fn canonical_sandbox_reattestation_timestamp(
    value: &str,
) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("sandbox re-attestation timestamp is not canonical UTC nanoseconds");
    }
    Ok(parsed)
}

fn canonical_nanos(value: &str) -> Result<DateTime<chrono::FixedOffset>> {
    canonical_sandbox_reattestation_timestamp(value)
}
