use anyhow::{bail, Result};
use chrono::Duration;

use super::{super::*, observations::validate_observations, roots::*, support::*};

pub(crate) fn validate_task_protocol_conformance_run_evidence(
    evidence: &TaskProtocolConformanceRunEvidence,
    task_protocol_profile_digest: &str,
    fixture_catalog_digest: &str,
) -> Result<()> {
    for value in [
        &evidence.run_nonce_digest,
        &evidence.source_capsule_sha256,
        &evidence.launch_image_sha256,
        &evidence.public_fixture_delivery_root,
        &evidence.session_roots_digest,
        &evidence.session_transcript_digest,
        &evidence.delivery_inventory_digest,
        &evidence.exchange_inventory_digest,
        &evidence.task_observation_root,
    ] {
        digest(value)?;
    }
    if evidence.session_transcript_digest != evidence.session_roots_digest {
        bail!("task-protocol conformance session transcript root is not exact")
    }
    if evidence.source_capsule_size_bytes == 0
        || evidence.source_capsule_size_bytes > TASK_PROTOCOL_CONFORMANCE_MAX_SAFE_INTEGER
        || evidence.launch_image_size_bytes == 0
        || evidence.launch_image_size_bytes > TASK_PROTOCOL_CONFORMANCE_MAX_SAFE_INTEGER
        || evidence.source_capsule_sha256 == evidence.launch_image_sha256
    {
        bail!("task-protocol conformance capsule evidence is invalid")
    }
    validate_observations(
        &evidence.exchanges,
        &evidence.capabilities,
        &evidence.cleanup,
        task_protocol_profile_digest,
        fixture_catalog_digest,
        &evidence.delivery_inventory_digest,
        &evidence.exchange_inventory_digest,
        &evidence.task_observation_root,
    )?;
    let started = canonical_nanos(&evidence.run_started_at)?;
    let completed = canonical_nanos(&evidence.run_completed_at)?;
    let elapsed = completed.signed_duration_since(started).num_milliseconds();
    if elapsed < 0
        || u64::try_from(elapsed)? != evidence.duration_ms
        || evidence.duration_ms > TASK_PROTOCOL_CONFORMANCE_MAX_SAFE_INTEGER
    {
        bail!("task-protocol conformance run duration is not exact")
    }
    Ok(())
}

pub(crate) fn validate_task_protocol_conformance_run_material(
    value: &ExternalPoolAdapterTaskProtocolConformanceRunMaterial,
) -> Result<()> {
    validate_roots(value)?;
    digest(&value.public_fixture_delivery_root)?;
    validate_catalog_and_subjects(value)?;
    if value.session_transcript_digest != value.session_roots_digest {
        bail!("task-protocol conformance session transcript root is not exact")
    }
    validate_lineage(value)?;
    validate_observations(
        &value.exchanges,
        &value.capabilities,
        &value.cleanup,
        &value.task_protocol_profile_digest,
        &value.fixture_catalog_digest,
        &value.delivery_inventory_digest,
        &value.exchange_inventory_digest,
        &value.task_observation_root,
    )?;
    if value.exchanges.iter().any(|exchange| {
        exchange.synthetic_executor_digest
            != value.synthetic_subjects.fixture_executor.subject_digest
    }) {
        bail!("task-protocol conformance exchange executor is not the synthetic subject")
    }
    validate_times(value)?;
    if value.evidence_scope != TASK_PROTOCOL_CONFORMANCE_EVIDENCE_SCOPE
        || value.receipt_status != TASK_PROTOCOL_CONFORMANCE_RUN_STATUS
        || value.non_production_authority_status
            != TASK_PROTOCOL_CONFORMANCE_NON_PRODUCTION_AUTHORITY
        || value.effects != task_protocol_conformance_no_effects()
        || value.readiness != task_protocol_conformance_no_readiness()
    {
        bail!("task-protocol conformance run authority or effect is not exact")
    }
    Ok(())
}

pub(crate) fn validate_task_protocol_conformance_run_receipt(
    receipt: &ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
) -> Result<()> {
    metadata(
        &receipt.schema,
        TASK_PROTOCOL_CONFORMANCE_RUN_RECEIPT_SCHEMA,
        &receipt.run_receipt_id,
        &receipt.run_receipt_digest,
        &receipt.run_material_digest,
        &receipt.canonicalization,
        &receipt.digest_algorithm,
    )?;
    validate_task_protocol_conformance_run_material(&receipt.run)?;
    if task_protocol_conformance_run_material_digest(&receipt.run)? != receipt.run_material_digest
        || canonical_task_protocol_conformance_run_receipt_json_and_digest(receipt)?.1
            != receipt.run_receipt_digest
    {
        bail!("task-protocol conformance run receipt digest is not exact")
    }
    Ok(())
}

pub(crate) fn validate_task_protocol_conformance_revocation_material(
    value: &ExternalPoolAdapterTaskProtocolConformanceRevocationMaterial,
) -> Result<()> {
    identifier(&value.run_receipt_id)?;
    identifier(&value.registry_release_id)?;
    digest(&value.run_receipt_digest)?;
    digest(&value.registry_release_digest)?;
    reason(&value.reason)?;
    canonical_nanos(&value.revoked_at)?;
    if value.recorded_at != value.revoked_at
        || value.revocation_status != TASK_PROTOCOL_CONFORMANCE_REVOCATION_STATUS
        || value.effects != task_protocol_conformance_no_effects()
        || value.readiness != task_protocol_conformance_no_readiness()
    {
        bail!("task-protocol conformance revocation material is not exact")
    }
    Ok(())
}

pub(crate) fn validate_task_protocol_conformance_revocation_receipt(
    receipt: &ExternalPoolAdapterTaskProtocolConformanceRevocationReceipt,
) -> Result<()> {
    metadata(
        &receipt.schema,
        TASK_PROTOCOL_CONFORMANCE_REVOCATION_RECEIPT_SCHEMA,
        &receipt.revocation_receipt_id,
        &receipt.revocation_receipt_digest,
        &receipt.revocation_material_digest,
        &receipt.canonicalization,
        &receipt.digest_algorithm,
    )?;
    validate_task_protocol_conformance_revocation_material(&receipt.revocation)?;
    if task_protocol_conformance_revocation_material_digest(&receipt.revocation)?
        != receipt.revocation_material_digest
        || canonical_task_protocol_conformance_revocation_receipt_json_and_digest(receipt)?.1
            != receipt.revocation_receipt_digest
    {
        bail!("task-protocol conformance revocation receipt digest is not exact")
    }
    Ok(())
}

fn validate_lineage(value: &ExternalPoolAdapterTaskProtocolConformanceRunMaterial) -> Result<()> {
    if value.sequence == 0
        || value.sequence > TASK_PROTOCOL_CONFORMANCE_MAX_SAFE_INTEGER
        || value.predecessor_run_receipt_id.is_some()
            != value.predecessor_run_receipt_digest.is_some()
        || (value.sequence == 1) != value.predecessor_run_receipt_id.is_none()
    {
        bail!("task-protocol conformance predecessor lineage is invalid")
    }
    if let Some(id) = &value.predecessor_run_receipt_id {
        identifier(id)?;
    }
    if let Some(value) = &value.predecessor_run_receipt_digest {
        digest(value)?;
    }
    Ok(())
}

fn validate_times(value: &ExternalPoolAdapterTaskProtocolConformanceRunMaterial) -> Result<()> {
    let started = canonical_nanos(&value.run_started_at)?;
    let completed = canonical_nanos(&value.run_completed_at)?;
    let checked = canonical_nanos(&value.post_cleanup_checked_at)?;
    let expires = canonical_nanos(&value.expires_at)?;
    let vulnerability_expires =
        canonical_nanos(&value.vulnerability_reattestation.intelligence_expires_at)?;
    let sandbox_expires = canonical_nanos(&value.sandbox_reattestation.report_expires_at)?;
    let compatibility_expires = canonical_nanos(&value.runtime_compatibility.expires_at)?;
    let ttl_expires = checked
        .checked_add_signed(Duration::seconds(TASK_PROTOCOL_CONFORMANCE_EXPIRY_SECONDS))
        .ok_or_else(|| anyhow::anyhow!("task-protocol conformance TTL overflow"))?;
    let expected_expires = [
        vulnerability_expires,
        sandbox_expires,
        compatibility_expires,
    ]
    .into_iter()
    .fold(ttl_expires, std::cmp::min);
    let elapsed = completed.signed_duration_since(started).num_milliseconds();
    if elapsed < 0
        || u64::try_from(elapsed)? != value.duration_ms
        || value.duration_ms > TASK_PROTOCOL_CONFORMANCE_MAX_SAFE_INTEGER
        || completed > checked
        || checked >= expires
        || expires != expected_expires
        || value.recorded_at != value.post_cleanup_checked_at
    {
        bail!("task-protocol conformance timestamps or TTL are not exact")
    }
    Ok(())
}
