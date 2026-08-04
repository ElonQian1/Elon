use std::collections::HashSet;

use anyhow::{bail, Result};

use super::{
    identity::ComputePluginReleaseRef,
    plugin_manifest::{
        resource_limits_are_non_negative, ComputePluginDownloadDependency, ComputePluginManifest,
        ComputePluginPermissionProfile, ComputePluginResourceLimits, ComputePluginTarget,
        SignedComputePluginManifest, COMPUTE_PLUGIN_DIGEST_ALGORITHM,
        COMPUTE_PLUGIN_ENTRYPOINT_SIDECAR, COMPUTE_PLUGIN_MAX_ENTRYPOINT_ARGUMENTS,
        COMPUTE_PLUGIN_MAX_PACKAGE_FILES,
    },
    signed_artifact_verification::{
        verify_manifest_signature, ComputePluginPublisherKeyResolver,
        SignatureVerifiedComputePluginManifest,
    },
};

const MAX_IDENTIFIER_BYTES: usize = 200;
const MAX_REASONABLE_LIST_ITEMS: usize = 4_096;

#[derive(Debug, Clone)]
pub(crate) struct ValidatedComputePluginManifest {
    signed: SignedComputePluginManifest,
}

impl ValidatedComputePluginManifest {
    pub(crate) fn signed(&self) -> &SignedComputePluginManifest {
        &self.signed
    }

    pub(crate) fn manifest(&self) -> &ComputePluginManifest {
        &self.signed.manifest
    }

    pub(crate) fn release_ref(&self) -> ComputePluginReleaseRef {
        let manifest = self.manifest();
        ComputePluginReleaseRef {
            plugin_id: manifest.plugin_id.clone(),
            plugin_version: manifest.plugin_version.clone(),
            target_id: manifest.target.target_id.clone(),
            manifest_digest: self.signed.manifest_digest.clone(),
            package_digest: manifest.package.package_digest.clone(),
        }
    }
}

pub(crate) fn verify_and_validate_manifest(
    signed: &SignedComputePluginManifest,
    resolver: &dyn ComputePluginPublisherKeyResolver,
) -> Result<ValidatedComputePluginManifest> {
    let verified = verify_manifest_signature(signed, resolver)?;
    validate_verified_manifest(verified)
}

fn validate_verified_manifest(
    verified: SignatureVerifiedComputePluginManifest,
) -> Result<ValidatedComputePluginManifest> {
    let manifest = &verified.signed().manifest;
    validate_identifier("MANIFEST_PLUGIN_ID", &manifest.plugin_id)?;
    validate_identifier("MANIFEST_PLUGIN_VERSION", &manifest.plugin_version)?;
    validate_identifier("MANIFEST_PUBLISHER_ID", &manifest.publisher_id)?;
    validate_package(manifest)?;
    validate_host_api(manifest)?;
    validate_sorted_strings("MANIFEST_TASK_KINDS", &manifest.task_kinds, false)?;
    validate_target(&manifest.target)?;
    validate_entrypoint(manifest)?;
    validate_dependencies(manifest)?;
    validate_resource_limits(&manifest.requested_resources)?;
    validate_permissions(&manifest.requested_permissions)?;
    validate_state_compatibility(manifest)?;
    Ok(ValidatedComputePluginManifest {
        signed: verified.signed().clone(),
    })
}

fn validate_package(manifest: &ComputePluginManifest) -> Result<()> {
    let package = &manifest.package;
    validate_identifier("MANIFEST_PACKAGE_MEDIA_TYPE", &package.media_type)?;
    validate_identifier("MANIFEST_ARCHIVE_FORMAT", &package.archive_format)?;
    if package.digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || !is_sha256(&package.package_digest)
        || package.package_size_bytes <= 0
        || package.unpacked_size_bytes <= 0
        || package.files.is_empty()
        || package.files.len() > COMPUTE_PLUGIN_MAX_PACKAGE_FILES
    {
        bail!("MANIFEST_PACKAGE_INVALID: package digest, size or file count is invalid");
    }
    let mut previous_path: Option<&str> = None;
    let mut unpacked_total = 0_i64;
    for file in &package.files {
        if !is_normalized_relative_path(&file.relative_path)
            || !is_sha256(&file.digest)
            || file.size_bytes < 0
            || previous_path.is_some_and(|previous| previous >= file.relative_path.as_str())
        {
            bail!("MANIFEST_PACKAGE_FILE_INVALID: file list is not canonical");
        }
        previous_path = Some(&file.relative_path);
        unpacked_total = unpacked_total
            .checked_add(file.size_bytes)
            .ok_or_else(|| anyhow::anyhow!("MANIFEST_PACKAGE_SIZE_OVERFLOW"))?;
    }
    if unpacked_total != package.unpacked_size_bytes {
        bail!("MANIFEST_PACKAGE_SIZE_MISMATCH: unpacked bytes do not match the file list");
    }
    Ok(())
}

fn validate_host_api(manifest: &ComputePluginManifest) -> Result<()> {
    validate_identifier("MANIFEST_HOST_PROTOCOL", &manifest.host_api.protocol_id)?;
    if manifest.host_api.minimum_revision == 0
        || manifest.host_api.maximum_revision < manifest.host_api.minimum_revision
    {
        bail!("MANIFEST_HOST_API_RANGE: Host API range is invalid");
    }
    Ok(())
}

fn validate_target(target: &ComputePluginTarget) -> Result<()> {
    validate_identifier("MANIFEST_TARGET_ID", &target.target_id)?;
    validate_identifier("MANIFEST_TARGET_OS", &target.operating_system)?;
    validate_identifier("MANIFEST_TARGET_ARCH", &target.architecture)?;
    if target.accelerator_kind.is_some() != target.accelerator_abi.is_some() {
        bail!("MANIFEST_TARGET_ACCELERATOR: kind and ABI must be supplied together");
    }
    if let Some(kind) = &target.accelerator_kind {
        validate_identifier("MANIFEST_ACCELERATOR_KIND", kind)?;
    }
    if let Some(abi) = &target.accelerator_abi {
        validate_identifier("MANIFEST_ACCELERATOR_ABI", abi)?;
    }
    let mut previous: Option<&str> = None;
    for requirement in &target.minimum_driver_versions {
        validate_identifier("MANIFEST_DRIVER_FAMILY", &requirement.driver_family)?;
        validate_identifier("MANIFEST_DRIVER_VERSION", &requirement.minimum_version)?;
        if previous.is_some_and(|value| value >= requirement.driver_family.as_str()) {
            bail!("MANIFEST_DRIVER_ORDER: driver requirements must be sorted and unique");
        }
        previous = Some(&requirement.driver_family);
    }
    Ok(())
}

fn validate_entrypoint(manifest: &ComputePluginManifest) -> Result<()> {
    let entrypoint = &manifest.entrypoint;
    if entrypoint.entrypoint_kind != COMPUTE_PLUGIN_ENTRYPOINT_SIDECAR
        || !is_normalized_relative_path(&entrypoint.relative_path)
        || entrypoint.arguments.len() > COMPUTE_PLUGIN_MAX_ENTRYPOINT_ARGUMENTS
        || entrypoint
            .arguments
            .iter()
            .any(|argument| argument.len() > 4_096 || argument.chars().any(char::is_control))
    {
        bail!("MANIFEST_ENTRYPOINT_INVALID: only a bounded sidecar entrypoint is accepted");
    }
    if !manifest
        .package
        .files
        .iter()
        .any(|file| file.relative_path == entrypoint.relative_path && file.executable)
    {
        bail!("MANIFEST_ENTRYPOINT_MISSING: entrypoint is not an executable package file");
    }
    let health = &entrypoint.health_check;
    validate_identifier("MANIFEST_HEALTH_PROTOCOL", &health.protocol)?;
    if health.timeout_ms <= 0
        || health.interval_ms <= 0
        || health.healthy_after_successes <= 0
        || health.unhealthy_after_failures <= 0
    {
        bail!("MANIFEST_HEALTH_INVALID: health thresholds must be positive");
    }
    Ok(())
}

fn validate_dependencies(manifest: &ComputePluginManifest) -> Result<()> {
    if manifest.system_dependencies.len() > MAX_REASONABLE_LIST_ITEMS
        || manifest.download_dependencies.len() > MAX_REASONABLE_LIST_ITEMS
    {
        bail!("MANIFEST_DEPENDENCY_LIMIT: dependency list is oversized");
    }
    let mut previous_system: Option<&str> = None;
    for dependency in &manifest.system_dependencies {
        validate_identifier("MANIFEST_SYSTEM_DEPENDENCY_ID", &dependency.dependency_id)?;
        validate_identifier(
            "MANIFEST_SYSTEM_DEPENDENCY_VERSION",
            &dependency.version_requirement,
        )?;
        if previous_system.is_some_and(|value| value >= dependency.dependency_id.as_str()) {
            bail!("MANIFEST_SYSTEM_DEPENDENCY_ORDER: dependencies must be sorted and unique");
        }
        previous_system = Some(&dependency.dependency_id);
    }
    let mut previous_download: Option<&str> = None;
    for dependency in &manifest.download_dependencies {
        validate_download_dependency(dependency)?;
        if previous_download.is_some_and(|value| value >= dependency.artifact_id.as_str()) {
            bail!("MANIFEST_DOWNLOAD_DEPENDENCY_ORDER: dependencies must be sorted and unique");
        }
        previous_download = Some(&dependency.artifact_id);
    }
    Ok(())
}

fn validate_download_dependency(dependency: &ComputePluginDownloadDependency) -> Result<()> {
    validate_identifier("MANIFEST_DOWNLOAD_ARTIFACT_ID", &dependency.artifact_id)?;
    validate_identifier("MANIFEST_DOWNLOAD_MEDIA_TYPE", &dependency.media_type)?;
    if dependency.digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || !is_sha256(&dependency.digest)
        || dependency.size_bytes <= 0
    {
        bail!("MANIFEST_DOWNLOAD_DEPENDENCY_INVALID: digest or size is invalid");
    }
    Ok(())
}

fn validate_resource_limits(limits: &ComputePluginResourceLimits) -> Result<()> {
    if !resource_limits_are_non_negative(limits)
        || limits.max_cpu_millicores == 0
        || limits.max_memory_bytes == 0
        || limits.max_disk_bytes == 0
        || limits.max_processes == 0
        || limits.max_sidecar_uptime_seconds == 0
    {
        bail!("MANIFEST_RESOURCE_LIMITS: executable resource ceilings must be positive");
    }
    Ok(())
}

fn validate_permissions(permissions: &ComputePluginPermissionProfile) -> Result<()> {
    if !permissions.allow_network_egress && !permissions.allowed_egress_domains.is_empty() {
        bail!("MANIFEST_NETWORK_SCOPE: domains require network egress permission");
    }
    validate_sorted_strings(
        "MANIFEST_EGRESS_DOMAINS",
        &permissions.allowed_egress_domains,
        true,
    )?;
    if permissions
        .allowed_egress_domains
        .iter()
        .any(|domain| !is_canonical_domain(domain))
    {
        bail!("MANIFEST_EGRESS_DOMAIN_INVALID: domains must be exact lowercase DNS names");
    }
    let filesystem = permissions
        .filesystem_scopes
        .iter()
        .map(|scope| format!("{scope:?}"))
        .collect::<HashSet<_>>();
    let devices = permissions
        .device_scopes
        .iter()
        .map(|scope| format!("{scope:?}"))
        .collect::<HashSet<_>>();
    if filesystem.len() != permissions.filesystem_scopes.len()
        || devices.len() != permissions.device_scopes.len()
    {
        bail!("MANIFEST_PERMISSION_DUPLICATE: permission scopes must be unique");
    }
    Ok(())
}

fn validate_state_compatibility(manifest: &ComputePluginManifest) -> Result<()> {
    let Some(state) = &manifest.state_compatibility else {
        return Ok(());
    };
    validate_identifier("MANIFEST_STATE_SCHEMA", &state.state_schema)?;
    validate_identifier("MANIFEST_STATE_WRITES", &state.writes_version)?;
    validate_sorted_strings("MANIFEST_STATE_READS", &state.reads_versions, false)
}

fn validate_sorted_strings(code: &str, values: &[String], allow_empty: bool) -> Result<()> {
    if (!allow_empty && values.is_empty())
        || values.len() > MAX_REASONABLE_LIST_ITEMS
        || values.iter().any(|value| {
            value.is_empty()
                || value.len() > MAX_IDENTIFIER_BYTES
                || value.trim() != value
                || value.chars().any(char::is_control)
        })
        || values.windows(2).any(|pair| pair[0] >= pair[1])
    {
        bail!("{code}: list must be bounded, sorted and unique");
    }
    Ok(())
}

fn validate_identifier(code: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        bail!("{code}: identifier is empty, oversized or non-canonical");
    }
    Ok(())
}

pub(super) fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn is_normalized_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1_024
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value.contains(':')
        && !value.chars().any(char::is_control)
        && value
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

fn is_canonical_domain(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
        && value
            .split('.')
            .all(|label| !label.is_empty() && !label.starts_with('-') && !label.ends_with('-'))
}
