use anyhow::{bail, Result};

use crate::compute_federation::{
    external_pool_adapter_release::COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ROUTE_KIND,
    external_pool_adapter_runtime_compatibility_verification::{
        RUNTIME_COMPATIBILITY_V2_PROFILE_ID, RUNTIME_COMPATIBILITY_V2_PROFILE_REVISION,
    },
    external_pool_adapter_sandbox_reattestation::SANDBOX_REATTESTATION_POLICY_ID,
};

use super::{super::*, support::*};

pub(super) fn validate_roots(
    value: &ExternalPoolAdapterTaskProtocolConformanceRunMaterial,
) -> Result<()> {
    let release = &value.registry_release;
    for item in [
        &release.registry_release_id,
        &release.admission_id,
        &release.package_receipt_id,
        &release.source_receipt_id,
        &release.adapter_id,
        &release.release_version,
    ] {
        identifier(item)?;
    }
    text(&release.entrypoint_path, 1, 500)?;
    for item in [
        &release.registry_release_digest,
        &release.registry_release_material_digest,
        &release.admission_digest,
        &release.package_receipt_digest,
        &release.package_material_digest,
        &release.source_receipt_digest,
        &release.implementation_digest,
        &release.declared_implementation_sha256,
        &release.entrypoint_sha256,
        &release.installation_content_digest,
        &release.capability_set_digest,
    ] {
        digest(item)?;
    }
    let profile = server_task_protocol_conformance_profile_catalog()?;
    if release.route_kind != COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ROUTE_KIND
        || release.entrypoint_size_bytes == 0
        || release.entrypoint_size_bytes > TASK_PROTOCOL_CONFORMANCE_MAX_SAFE_INTEGER
        || release.implementation_digest != release.declared_implementation_sha256
        || release.supported_capabilities != profile.profile.required_capabilities
    {
        bail!("task-protocol conformance V249 roots are not exact")
    }

    let vulnerability = &value.vulnerability_reattestation;
    identifier(&vulnerability.reattestation_receipt_id)?;
    for item in [
        &vulnerability.reattestation_receipt_digest,
        &vulnerability.reattestation_material_digest,
        &vulnerability.intelligence_snapshot_digest,
    ] {
        digest(item)?;
    }
    canonical_nanos(&vulnerability.intelligence_expires_at)?;
    if vulnerability.blocking_finding_count != 0 {
        bail!("task-protocol conformance V250 root has blocking findings")
    }

    let sandbox = &value.sandbox_reattestation;
    identifier(&sandbox.reattestation_receipt_id)?;
    for item in [
        &sandbox.reattestation_receipt_digest,
        &sandbox.reattestation_material_digest,
        &sandbox.test_plan_digest,
        &sandbox.observation_inventory_digest,
    ] {
        digest(item)?;
    }
    canonical_nanos(&sandbox.report_expires_at)?;
    if sandbox.sandbox_policy_id != SANDBOX_REATTESTATION_POLICY_ID
        || sandbox.passed_capability_count
            != u64::try_from(TASK_PROTOCOL_CONFORMANCE_CAPABILITY_COUNT)?
        || sandbox.policy_violation_count != 0
    {
        bail!("task-protocol conformance V252 roots are not exact")
    }

    let key = &value.sandbox_verifier_key;
    for item in [
        &key.key_record_id,
        &key.key_id,
        &key.verifier_operator,
        &key.verifier_product,
    ] {
        identifier(item)?;
    }
    digest(&key.key_record_digest)?;

    let runtime = &value.runtime_compatibility;
    for item in [
        &runtime.verification_receipt_id,
        &runtime.run_observation_id,
        &runtime.runner_execution_id,
        &runtime.profile_id,
    ] {
        identifier(item)?;
    }
    for item in [
        &runtime.verification_receipt_digest,
        &runtime.verification_material_digest,
        &runtime.run_observation_digest,
        &runtime.run_observation_material_digest,
        &runtime.profile_digest,
        &runtime.runner_policy_digest,
        &runtime.fixture_catalog_digest,
        &runtime.supervisor_session_policy_digest,
        &runtime.source_capsule_sha256,
        &runtime.launch_image_sha256,
        &runtime.public_fixture_delivery_root,
    ] {
        digest(item)?;
    }
    canonical_nanos(&runtime.expires_at)?;
    if runtime.profile_id != RUNTIME_COMPATIBILITY_V2_PROFILE_ID
        || runtime.profile_revision != RUNTIME_COMPATIBILITY_V2_PROFILE_REVISION
        || runtime.source_capsule_size_bytes == 0
        || runtime.source_capsule_size_bytes > TASK_PROTOCOL_CONFORMANCE_MAX_SAFE_INTEGER
        || runtime.launch_image_size_bytes == 0
        || runtime.launch_image_size_bytes > TASK_PROTOCOL_CONFORMANCE_MAX_SAFE_INTEGER
        || runtime.source_capsule_sha256 == runtime.launch_image_sha256
    {
        bail!("task-protocol conformance V268 roots are not exact")
    }
    Ok(())
}

pub(super) fn validate_catalog_and_subjects(
    value: &ExternalPoolAdapterTaskProtocolConformanceRunMaterial,
) -> Result<()> {
    let profile = server_task_protocol_conformance_profile_catalog()?;
    let fixture = server_task_protocol_conformance_fixture_catalog()?;
    if value.task_protocol_profile_id != profile.profile.profile_id
        || value.task_protocol_profile_revision != profile.profile.profile_revision
        || value.task_protocol_profile_digest != profile.profile_digest
        || value.fixture_catalog_id != fixture.catalog.catalog_id
        || value.fixture_catalog_revision != fixture.catalog.catalog_revision
        || value.fixture_catalog_digest != fixture.catalog_digest
    {
        bail!("task-protocol conformance catalog roots drifted")
    }
    let subjects = derive_task_protocol_conformance_synthetic_subjects(
        &value.registry_release,
        &value.task_protocol_profile_digest,
        &value.fixture_catalog_digest,
    )?;
    if value.synthetic_subjects != subjects {
        bail!("task-protocol conformance synthetic subjects are not exact")
    }
    let expected_roots = vec![
        value
            .runtime_compatibility
            .supervisor_session_policy_digest
            .clone(),
        value.task_protocol_profile_digest.clone(),
        value.run_nonce_digest.clone(),
        value.fixture_catalog_digest.clone(),
        value.registry_release.registry_release_digest.clone(),
        value.registry_release.installation_content_digest.clone(),
        value.registry_release.capability_set_digest.clone(),
        value
            .sandbox_reattestation
            .reattestation_receipt_digest
            .clone(),
        value
            .runtime_compatibility
            .verification_receipt_digest
            .clone(),
        value.runtime_compatibility.source_capsule_sha256.clone(),
        value.runtime_compatibility.launch_image_sha256.clone(),
        value.public_fixture_delivery_root.clone(),
        value.synthetic_subjects.fixture_lane.subject_digest.clone(),
        value
            .synthetic_subjects
            .fixture_executor
            .subject_digest
            .clone(),
    ];
    if value.session_root_digests != expected_roots
        || task_protocol_conformance_session_roots_digest(&expected_roots)?
            != value.session_roots_digest
    {
        bail!("task-protocol conformance ordered session roots are not exact")
    }
    Ok(())
}
