use std::collections::HashSet;

use anyhow::{bail, Result};

use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef,
    manifest_validation::{is_normalized_relative_path, is_sha256},
    plugin_manifest::{
        resource_limits_are_non_negative, SignedComputePluginManifest,
        COMPUTE_PLUGIN_ARCHIVE_FORMAT_ZIP, COMPUTE_PLUGIN_DIGEST_ALGORITHM,
        COMPUTE_PLUGIN_ENTRYPOINT_SIDECAR, COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION,
        COMPUTE_PLUGIN_MANIFEST_SCHEMA, COMPUTE_PLUGIN_MAX_ENTRYPOINT_ARGUMENTS,
        COMPUTE_PLUGIN_MAX_PACKAGE_BYTES, COMPUTE_PLUGIN_MAX_PACKAGE_FILES,
        COMPUTE_PLUGIN_MAX_UNPACKED_BYTES, COMPUTE_PLUGIN_PACKAGE_MEDIA_TYPE_ZIP,
        COMPUTE_PLUGIN_SIGNATURE_ALGORITHM, SIGNED_COMPUTE_PLUGIN_MANIFEST_SCHEMA,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

const MAX_IDENTIFIER_BYTES: usize = 200;
const MAX_LIST_ITEMS: usize = 4_096;

pub(super) fn validate_work_admission_signed_manifest(
    signed: &SignedComputePluginManifest,
    expected_envelope_digest: &str,
) -> Result<ComputePluginReleaseRef> {
    let manifest = &signed.manifest;
    if signed.schema != SIGNED_COMPUTE_PLUGIN_MANIFEST_SCHEMA
        || signed.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || signed.manifest_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || !is_sha256(&signed.manifest_digest)
        || jcs_sha256_hex(manifest)? != signed.manifest_digest
        || !is_sha256(expected_envelope_digest)
        || jcs_sha256_hex(signed)? != expected_envelope_digest
        || signed.signature.algorithm != COMPUTE_PLUGIN_SIGNATURE_ALGORITHM
        || !identifier(&signed.signature.signing_key_id)
        || signed.signature.signature_base64.trim().is_empty()
        || manifest.schema != COMPUTE_PLUGIN_MANIFEST_SCHEMA
        || !identifier(&manifest.plugin_id)
        || !identifier(&manifest.plugin_version)
        || !identifier(&manifest.publisher_id)
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_SIGNED_MANIFEST_INVALID");
    }
    validate_package(signed)?;
    validate_host_and_tasks(signed)?;
    validate_target(signed)?;
    validate_entrypoint(signed)?;
    validate_dependencies(signed)?;
    validate_resources_and_permissions(signed)?;
    validate_state(signed)?;
    Ok(ComputePluginReleaseRef {
        plugin_id: manifest.plugin_id.clone(),
        plugin_version: manifest.plugin_version.clone(),
        target_id: manifest.target.target_id.clone(),
        manifest_digest: signed.manifest_digest.clone(),
        package_digest: manifest.package.package_digest.clone(),
    })
}

fn validate_package(signed: &SignedComputePluginManifest) -> Result<()> {
    let package = &signed.manifest.package;
    if package.media_type != COMPUTE_PLUGIN_PACKAGE_MEDIA_TYPE_ZIP
        || package.archive_format != COMPUTE_PLUGIN_ARCHIVE_FORMAT_ZIP
        || package.digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || !is_sha256(&package.package_digest)
        || !(1..=COMPUTE_PLUGIN_MAX_PACKAGE_BYTES).contains(&package.package_size_bytes)
        || !(1..=COMPUTE_PLUGIN_MAX_UNPACKED_BYTES).contains(&package.unpacked_size_bytes)
        || package.files.is_empty()
        || package.files.len() > COMPUTE_PLUGIN_MAX_PACKAGE_FILES
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_MANIFEST_PACKAGE_INVALID");
    }
    let mut previous = None;
    let mut total = 0_i64;
    for file in &package.files {
        if !is_normalized_relative_path(&file.relative_path)
            || !is_sha256(&file.digest)
            || file.size_bytes < 0
            || previous.is_some_and(|value| value >= file.relative_path.as_str())
        {
            bail!("COMPUTE_PLUGIN_WORK_ADMISSION_MANIFEST_FILE_INVALID");
        }
        previous = Some(file.relative_path.as_str());
        total = total
            .checked_add(file.size_bytes)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_FILE_SIZE_OVERFLOW"))?;
    }
    if total != package.unpacked_size_bytes {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_MANIFEST_FILE_SIZE_CHANGED");
    }
    Ok(())
}

fn validate_host_and_tasks(signed: &SignedComputePluginManifest) -> Result<()> {
    let manifest = &signed.manifest;
    if !identifier(&manifest.host_api.protocol_id)
        || manifest.host_api.minimum_revision == 0
        || manifest.host_api.maximum_revision < manifest.host_api.minimum_revision
        || !sorted_strings(&manifest.task_kinds, false)
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_MANIFEST_HOST_INVALID");
    }
    Ok(())
}

fn validate_target(signed: &SignedComputePluginManifest) -> Result<()> {
    let target = &signed.manifest.target;
    if !identifier(&target.target_id)
        || !identifier(&target.operating_system)
        || !identifier(&target.architecture)
        || target.accelerator_kind.is_some() != target.accelerator_abi.is_some()
        || target
            .accelerator_kind
            .as_deref()
            .is_some_and(|v| !identifier(v))
        || target
            .accelerator_abi
            .as_deref()
            .is_some_and(|v| !identifier(v))
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_MANIFEST_TARGET_INVALID");
    }
    let mut previous = None;
    for driver in &target.minimum_driver_versions {
        if !identifier(&driver.driver_family)
            || !identifier(&driver.minimum_version)
            || previous.is_some_and(|value| value >= driver.driver_family.as_str())
        {
            bail!("COMPUTE_PLUGIN_WORK_ADMISSION_MANIFEST_DRIVER_INVALID");
        }
        previous = Some(driver.driver_family.as_str());
    }
    Ok(())
}

fn validate_entrypoint(signed: &SignedComputePluginManifest) -> Result<()> {
    let manifest = &signed.manifest;
    let entrypoint = &manifest.entrypoint;
    let health = &entrypoint.health_check;
    if entrypoint.entrypoint_kind != COMPUTE_PLUGIN_ENTRYPOINT_SIDECAR
        || !is_normalized_relative_path(&entrypoint.relative_path)
        || entrypoint.arguments.len() > COMPUTE_PLUGIN_MAX_ENTRYPOINT_ARGUMENTS
        || entrypoint
            .arguments
            .iter()
            .any(|value| value.len() > 4_096 || value.chars().any(char::is_control))
        || !manifest
            .package
            .files
            .iter()
            .any(|file| file.relative_path == entrypoint.relative_path && file.executable)
        || !identifier(&health.protocol)
        || health.timeout_ms <= 0
        || health.interval_ms <= 0
        || health.healthy_after_successes <= 0
        || health.unhealthy_after_failures <= 0
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_MANIFEST_ENTRYPOINT_INVALID");
    }
    Ok(())
}

fn validate_dependencies(signed: &SignedComputePluginManifest) -> Result<()> {
    let manifest = &signed.manifest;
    if manifest.system_dependencies.len() > MAX_LIST_ITEMS
        || manifest.download_dependencies.len() > MAX_LIST_ITEMS
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_MANIFEST_DEPENDENCIES_OVERSIZED");
    }
    let mut previous = None;
    for dependency in &manifest.system_dependencies {
        if !identifier(&dependency.dependency_id)
            || !identifier(&dependency.version_requirement)
            || previous.is_some_and(|value| value >= dependency.dependency_id.as_str())
        {
            bail!("COMPUTE_PLUGIN_WORK_ADMISSION_SYSTEM_DEPENDENCY_INVALID");
        }
        previous = Some(dependency.dependency_id.as_str());
    }
    let mut previous = None;
    let mut digests = HashSet::from([manifest.package.package_digest.as_str()]);
    for dependency in &manifest.download_dependencies {
        if !identifier(&dependency.artifact_id)
            || !identifier(&dependency.media_type)
            || dependency.digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
            || !is_sha256(&dependency.digest)
            || dependency.size_bytes <= 0
            || previous.is_some_and(|value| value >= dependency.artifact_id.as_str())
            || !digests.insert(dependency.digest.as_str())
        {
            bail!("COMPUTE_PLUGIN_WORK_ADMISSION_DOWNLOAD_DEPENDENCY_INVALID");
        }
        previous = Some(dependency.artifact_id.as_str());
    }
    Ok(())
}

fn validate_resources_and_permissions(signed: &SignedComputePluginManifest) -> Result<()> {
    let manifest = &signed.manifest;
    let resources = &manifest.requested_resources;
    let permissions = &manifest.requested_permissions;
    if !resource_limits_are_non_negative(resources)
        || resources.max_cpu_millicores == 0
        || resources.max_memory_bytes == 0
        || resources.max_disk_bytes == 0
        || resources.max_processes == 0
        || resources.max_sidecar_uptime_seconds == 0
        || permissions.allow_network_egress == permissions.allowed_egress_domains.is_empty()
        || !sorted_strings(&permissions.allowed_egress_domains, true)
        || permissions
            .allowed_egress_domains
            .iter()
            .any(|value| !canonical_domain(value))
        || !strictly_sorted(permissions.filesystem_scopes.iter().map(|v| v.wire_name()))
        || !strictly_sorted(permissions.device_scopes.iter().map(|v| v.wire_name()))
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_MANIFEST_AUTHORIZATION_INVALID");
    }
    Ok(())
}

fn validate_state(signed: &SignedComputePluginManifest) -> Result<()> {
    let Some(state) = &signed.manifest.state_compatibility else {
        return Ok(());
    };
    if !identifier(&state.state_schema)
        || !identifier(&state.writes_version)
        || !sorted_strings(&state.reads_versions, false)
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_MANIFEST_STATE_INVALID");
    }
    Ok(())
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn canonical_domain(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
        && value
            .split('.')
            .all(|label| !label.is_empty() && !label.starts_with('-') && !label.ends_with('-'))
}

fn sorted_strings(values: &[String], allow_empty: bool) -> bool {
    (allow_empty || !values.is_empty())
        && values.len() <= MAX_LIST_ITEMS
        && values.iter().all(|value| identifier(value))
        && values.windows(2).all(|pair| pair[0] < pair[1])
}

fn strictly_sorted<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let mut previous = None;
    values.into_iter().all(|value| {
        let valid = previous.is_none_or(|prior| prior < value);
        previous = Some(value);
        valid
    })
}
