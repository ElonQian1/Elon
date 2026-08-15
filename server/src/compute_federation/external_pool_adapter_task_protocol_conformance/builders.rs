use anyhow::{bail, Result};

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_external_pool_adapter_task_protocol_conformance_run_material(
    roots: ExternalPoolAdapterTaskProtocolConformanceRunRoots,
    evidence: TaskProtocolConformanceRunEvidence,
    sequence: u64,
    predecessor: Option<ExternalPoolAdapterTaskProtocolConformancePredecessor>,
    post_cleanup_checked_at: String,
    expires_at: String,
) -> Result<ExternalPoolAdapterTaskProtocolConformanceRunMaterial> {
    if evidence.source_capsule_sha256 != roots.runtime_compatibility.source_capsule_sha256
        || evidence.source_capsule_size_bytes
            != roots.runtime_compatibility.source_capsule_size_bytes
        || evidence.launch_image_sha256 != roots.runtime_compatibility.launch_image_sha256
        || evidence.launch_image_size_bytes != roots.runtime_compatibility.launch_image_size_bytes
    {
        bail!("task-protocol conformance fresh capsule evidence drifted from exact V268 roots")
    }
    let profile = server_task_protocol_conformance_profile_catalog()?;
    let fixture = server_task_protocol_conformance_fixture_catalog()?;
    let subjects = derive_task_protocol_conformance_synthetic_subjects(
        &roots.registry_release,
        &profile.profile_digest,
        &fixture.catalog_digest,
    )?;
    let session_root_digests = vec![
        roots
            .runtime_compatibility
            .supervisor_session_policy_digest
            .clone(),
        profile.profile_digest.clone(),
        evidence.run_nonce_digest.clone(),
        fixture.catalog_digest.clone(),
        roots.registry_release.registry_release_digest.clone(),
        roots.registry_release.installation_content_digest.clone(),
        roots.registry_release.capability_set_digest.clone(),
        roots
            .sandbox_reattestation
            .reattestation_receipt_digest
            .clone(),
        roots
            .runtime_compatibility
            .verification_receipt_digest
            .clone(),
        evidence.source_capsule_sha256.clone(),
        evidence.launch_image_sha256.clone(),
        evidence.public_fixture_delivery_root.clone(),
        subjects.fixture_lane.subject_digest.clone(),
        subjects.fixture_executor.subject_digest.clone(),
    ];
    let (predecessor_run_receipt_id, predecessor_run_receipt_digest) = predecessor
        .map(|value| (Some(value.run_receipt_id), Some(value.run_receipt_digest)))
        .unwrap_or((None, None));
    let TaskProtocolConformanceRunEvidence {
        run_nonce_digest,
        source_capsule_sha256: _,
        source_capsule_size_bytes: _,
        launch_image_sha256: _,
        launch_image_size_bytes: _,
        public_fixture_delivery_root,
        session_roots_digest,
        session_transcript_digest,
        delivery_inventory_digest,
        exchange_inventory_digest,
        task_observation_root,
        run_started_at,
        run_completed_at,
        duration_ms,
        exchanges,
        capabilities,
        cleanup,
    } = evidence;
    let run = ExternalPoolAdapterTaskProtocolConformanceRunMaterial {
        registry_release: roots.registry_release,
        vulnerability_reattestation: roots.vulnerability_reattestation,
        sandbox_reattestation: roots.sandbox_reattestation,
        sandbox_verifier_key: roots.sandbox_verifier_key,
        runtime_compatibility: roots.runtime_compatibility,
        task_protocol_profile_id: profile.profile.profile_id,
        task_protocol_profile_revision: profile.profile.profile_revision,
        task_protocol_profile_digest: profile.profile_digest,
        fixture_catalog_id: fixture.catalog.catalog_id,
        fixture_catalog_revision: fixture.catalog.catalog_revision,
        fixture_catalog_digest: fixture.catalog_digest,
        synthetic_subjects: subjects,
        session_root_digests,
        run_nonce_digest,
        public_fixture_delivery_root,
        session_roots_digest,
        session_transcript_digest,
        delivery_inventory_digest,
        exchange_inventory_digest,
        task_observation_root,
        exchanges,
        capabilities,
        cleanup,
        duration_ms,
        sequence,
        predecessor_run_receipt_id,
        predecessor_run_receipt_digest,
        run_started_at,
        run_completed_at,
        post_cleanup_checked_at: post_cleanup_checked_at.clone(),
        expires_at,
        recorded_at: post_cleanup_checked_at,
        evidence_scope: TASK_PROTOCOL_CONFORMANCE_EVIDENCE_SCOPE.into(),
        receipt_status: TASK_PROTOCOL_CONFORMANCE_RUN_STATUS.into(),
        non_production_authority_status: TASK_PROTOCOL_CONFORMANCE_NON_PRODUCTION_AUTHORITY.into(),
        effects: task_protocol_conformance_no_effects(),
        readiness: task_protocol_conformance_no_readiness(),
    };
    validate_task_protocol_conformance_run_material(&run)?;
    Ok(run)
}

pub(crate) fn build_external_pool_adapter_task_protocol_conformance_run_receipt(
    run_receipt_id: String,
    run: ExternalPoolAdapterTaskProtocolConformanceRunMaterial,
) -> Result<ExternalPoolAdapterTaskProtocolConformanceRunReceipt> {
    validate_task_protocol_conformance_run_material(&run)?;
    let run_material_digest = task_protocol_conformance_run_material_digest(&run)?;
    let mut receipt = ExternalPoolAdapterTaskProtocolConformanceRunReceipt {
        schema: TASK_PROTOCOL_CONFORMANCE_RUN_RECEIPT_SCHEMA.into(),
        run_receipt_id,
        run_receipt_digest: String::new(),
        run_material_digest,
        canonicalization: TASK_PROTOCOL_CONFORMANCE_CANONICALIZATION.into(),
        digest_algorithm: TASK_PROTOCOL_CONFORMANCE_DIGEST_ALGORITHM.into(),
        run,
    };
    receipt.run_receipt_digest =
        canonical_task_protocol_conformance_run_receipt_json_and_digest(&receipt)?.1;
    validate_task_protocol_conformance_run_receipt(&receipt)?;
    Ok(receipt)
}

pub(crate) fn build_external_pool_adapter_task_protocol_conformance_revocation_material(
    target: &ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
    reason: String,
    revoked_at: String,
) -> Result<ExternalPoolAdapterTaskProtocolConformanceRevocationMaterial> {
    validate_task_protocol_conformance_run_receipt(target)?;
    let revocation = ExternalPoolAdapterTaskProtocolConformanceRevocationMaterial {
        run_receipt_id: target.run_receipt_id.clone(),
        run_receipt_digest: target.run_receipt_digest.clone(),
        registry_release_id: target.run.registry_release.registry_release_id.clone(),
        registry_release_digest: target.run.registry_release.registry_release_digest.clone(),
        reason,
        revoked_at: revoked_at.clone(),
        recorded_at: revoked_at,
        revocation_status: TASK_PROTOCOL_CONFORMANCE_REVOCATION_STATUS.into(),
        effects: task_protocol_conformance_no_effects(),
        readiness: task_protocol_conformance_no_readiness(),
    };
    validate_task_protocol_conformance_revocation_material(&revocation)?;
    Ok(revocation)
}

pub(crate) fn build_external_pool_adapter_task_protocol_conformance_revocation_receipt(
    revocation_receipt_id: String,
    revocation: ExternalPoolAdapterTaskProtocolConformanceRevocationMaterial,
) -> Result<ExternalPoolAdapterTaskProtocolConformanceRevocationReceipt> {
    validate_task_protocol_conformance_revocation_material(&revocation)?;
    let revocation_material_digest =
        task_protocol_conformance_revocation_material_digest(&revocation)?;
    let mut receipt = ExternalPoolAdapterTaskProtocolConformanceRevocationReceipt {
        schema: TASK_PROTOCOL_CONFORMANCE_REVOCATION_RECEIPT_SCHEMA.into(),
        revocation_receipt_id,
        revocation_receipt_digest: String::new(),
        revocation_material_digest,
        canonicalization: TASK_PROTOCOL_CONFORMANCE_CANONICALIZATION.into(),
        digest_algorithm: TASK_PROTOCOL_CONFORMANCE_DIGEST_ALGORITHM.into(),
        revocation,
    };
    receipt.revocation_receipt_digest =
        canonical_task_protocol_conformance_revocation_receipt_json_and_digest(&receipt)?.1;
    validate_task_protocol_conformance_revocation_receipt(&receipt)?;
    Ok(receipt)
}
