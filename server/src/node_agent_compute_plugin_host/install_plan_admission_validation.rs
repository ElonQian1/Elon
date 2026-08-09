use std::collections::HashSet;

use anyhow::{bail, Result};
use serde::Serialize;

use super::{
    identity::ComputePluginReleaseRef,
    install_plan::{
        ComputePluginGrantBinding, ComputePluginPlanItem, ComputePluginPlannedDownload,
        PLAN_ACTION_CANCEL_CANDIDATE, PLAN_ACTION_DISABLE, PLAN_ACTION_INSTALL, PLAN_ACTION_KEEP,
        PLAN_ACTION_REAUTHORIZE_EXISTING, PLAN_ACTION_REMOVE, PLAN_ACTION_UPGRADE,
        PLAN_TARGET_ENABLED,
    },
    install_plan_admission::ComputePluginLiveAdmissionState,
    install_plan_reauthorization::validate_reauthorization_source,
    lifecycle::{ComputePluginInventorySnapshot, ComputePluginLocalRecord},
    manifest_validation::{is_sha256, ValidatedComputePluginManifest},
    plugin_manifest::{
        resource_limits_are_non_negative, ComputePluginPermissionProfile,
        ComputePluginResourceLimits,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

const MAX_PLAN_ITEMS: usize = 256;
const MAX_REASON_CODES: usize = 64;
const GRANT_DIGEST_SCHEMA: &str = "elon.compute_plugin.grant_binding.v1";
const ARTIFACT_PLUGIN_PACKAGE: &str = "plugin_package";
const ARTIFACT_PLUGIN_DEPENDENCY: &str = "plugin_dependency";

pub(super) fn validate_expected_local_state(
    item: &ComputePluginPlanItem,
    inventory: &ComputePluginInventorySnapshot,
    drain_before_replace: bool,
) -> Result<()> {
    let plugin_id = item_plugin_id(item)?;
    let current = inventory
        .plugins
        .iter()
        .find(|record| record.plugin_id == plugin_id);
    if item.action == PLAN_ACTION_INSTALL {
        if current.is_some() {
            bail!("COMPUTE_PLUGIN_INSTALL_NOT_ABSENT: install target already has local state");
        }
        return Ok(());
    }
    let record = current.ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_CURRENT_MISSING: expected plugin is absent")
    })?;
    if item.action == PLAN_ACTION_CANCEL_CANDIDATE {
        validate_candidate_binding(item, record)?;
        if item.target_activation != PLAN_TARGET_ENABLED
            && record.active_attempts > 0
            && !drain_before_replace
        {
            bail!("COMPUTE_PLUGIN_DRAIN_REQUIRED: active attempts require an explicit drain plan");
        }
        return Ok(());
    }
    if record.candidate_slot_ref.is_some() {
        bail!("COMPUTE_PLUGIN_CANDIDATE_BUSY: another candidate is already staged");
    }
    let expected_release = item.expected_current_release.as_ref().ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_CURRENT_BINDING: expected release is missing")
    })?;
    if active_release(record) != Some(expected_release)
        || item.expected_install_generation != Some(record.install_generation)
    {
        bail!("COMPUTE_PLUGIN_CURRENT_BINDING: active release or generation changed");
    }
    if item.target_activation != PLAN_TARGET_ENABLED
        && record.active_attempts > 0
        && !drain_before_replace
    {
        bail!("COMPUTE_PLUGIN_DRAIN_REQUIRED: active attempts require an explicit drain plan");
    }
    if !matches!(
        item.action.as_str(),
        PLAN_ACTION_UPGRADE
            | PLAN_ACTION_KEEP
            | PLAN_ACTION_REAUTHORIZE_EXISTING
            | PLAN_ACTION_DISABLE
            | PLAN_ACTION_REMOVE
    ) {
        bail!("COMPUTE_PLUGIN_PLAN_ACTION: unsupported action");
    }
    validate_reauthorization_source(item, record)?;
    Ok(())
}

fn validate_candidate_binding(
    item: &ComputePluginPlanItem,
    record: &ComputePluginLocalRecord,
) -> Result<()> {
    let expected_candidate = item.expected_candidate_release.as_ref().ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_BINDING: expected candidate is missing")
    })?;
    let candidate_release = record.candidate_slot_ref.as_ref().and_then(|slot_ref| {
        record
            .slots
            .iter()
            .find(|slot| &slot.slot_ref == slot_ref)
            .map(|slot| &slot.release)
    });
    if candidate_release != Some(expected_candidate)
        || item.expected_install_generation != Some(record.install_generation)
        || active_release(record) != item.expected_current_release.as_ref()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_BINDING: candidate, active release or generation changed");
    }
    Ok(())
}

fn active_release(record: &ComputePluginLocalRecord) -> Option<&ComputePluginReleaseRef> {
    let active = record.active_slot_ref.as_ref()?;
    record
        .slots
        .iter()
        .find(|slot| &slot.slot_ref == active)
        .map(|slot| &slot.release)
}

pub(super) fn validate_target_compatibility(
    manifest: &ValidatedComputePluginManifest,
    live: &ComputePluginLiveAdmissionState,
) -> Result<()> {
    let value = manifest.manifest();
    if value.target.target_id != live.target_id
        || value.host_api.protocol_id != live.host_api_protocol_id
        || live.host_api_revision < value.host_api.minimum_revision
        || live.host_api_revision > value.host_api.maximum_revision
    {
        bail!("COMPUTE_PLUGIN_TARGET_MISMATCH: target or Host API is incompatible");
    }
    Ok(())
}

pub(super) fn validate_grant(
    grant: Option<&ComputePluginGrantBinding>,
    manifest: &ValidatedComputePluginManifest,
) -> Result<()> {
    let grant = grant.ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_GRANT_MISSING"))?;
    if !is_identifier(&grant.grant_ref)
        || !is_sha256(&grant.grant_digest)
        || !resource_limits_are_non_negative(&grant.granted_resources)
        || !executable_resources_are_positive(&grant.granted_resources)
        || !resources_are_subset(
            &grant.granted_resources,
            &manifest.manifest().requested_resources,
        )
        || !permissions_are_subset(
            &grant.granted_permissions,
            &manifest.manifest().requested_permissions,
        )
    {
        bail!("COMPUTE_PLUGIN_GRANT_EXCEEDS_REQUEST: grant is invalid or exceeds Manifest request");
    }
    #[derive(Serialize)]
    struct GrantDigest<'a> {
        schema: &'static str,
        grant_ref: &'a str,
        granted_permissions: &'a ComputePluginPermissionProfile,
        granted_resources: &'a ComputePluginResourceLimits,
    }
    let digest = jcs_sha256_hex(&GrantDigest {
        schema: GRANT_DIGEST_SCHEMA,
        grant_ref: &grant.grant_ref,
        granted_permissions: &grant.granted_permissions,
        granted_resources: &grant.granted_resources,
    })?;
    if digest != grant.grant_digest {
        bail!("COMPUTE_PLUGIN_GRANT_DIGEST: grant digest does not match canonical grant");
    }
    Ok(())
}

fn executable_resources_are_positive(resources: &ComputePluginResourceLimits) -> bool {
    resources.max_cpu_millicores > 0
        && resources.max_memory_bytes > 0
        && resources.max_disk_bytes > 0
        && resources.max_processes > 0
        && resources.max_sidecar_uptime_seconds > 0
}

fn resources_are_subset(
    granted: &ComputePluginResourceLimits,
    requested: &ComputePluginResourceLimits,
) -> bool {
    granted.max_cpu_millicores <= requested.max_cpu_millicores
        && granted.max_memory_bytes <= requested.max_memory_bytes
        && granted.max_vram_bytes <= requested.max_vram_bytes
        && granted.max_disk_bytes <= requested.max_disk_bytes
        && granted.max_processes <= requested.max_processes
        && granted.max_sidecar_uptime_seconds <= requested.max_sidecar_uptime_seconds
}

fn permissions_are_subset(
    granted: &ComputePluginPermissionProfile,
    requested: &ComputePluginPermissionProfile,
) -> bool {
    (!granted.allow_network_egress || requested.allow_network_egress)
        && (!granted.allow_network_egress || !granted.allowed_egress_domains.is_empty())
        && (granted.allow_network_egress || granted.allowed_egress_domains.is_empty())
        && sorted_unique(&granted.allowed_egress_domains)
        && granted
            .allowed_egress_domains
            .iter()
            .all(|domain| requested.allowed_egress_domains.contains(domain))
        && (!granted.allow_child_processes || requested.allow_child_processes)
        && strictly_sorted_wire(
            granted
                .filesystem_scopes
                .iter()
                .map(|scope| scope.wire_name()),
        )
        && granted
            .filesystem_scopes
            .iter()
            .all(|scope| requested.filesystem_scopes.contains(scope))
        && strictly_sorted_wire(granted.device_scopes.iter().map(|scope| scope.wire_name()))
        && granted
            .device_scopes
            .iter()
            .all(|scope| requested.device_scopes.contains(scope))
}

fn strictly_sorted_wire<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let mut previous: Option<&str> = None;
    for value in values {
        if previous.is_some_and(|prior| prior >= value) {
            return false;
        }
        previous = Some(value);
    }
    true
}

pub(super) fn validate_download_closure(
    item: &ComputePluginPlanItem,
    manifest: &ValidatedComputePluginManifest,
) -> Result<i64> {
    let package = &manifest.manifest().package;
    let expected_count = 1 + manifest.manifest().download_dependencies.len();
    if item.downloads.len() != expected_count {
        bail!("COMPUTE_PLUGIN_DOWNLOAD_CLOSURE: package and dependencies must be exact");
    }
    let package_download = &item.downloads[0];
    if package_download.artifact_kind != ARTIFACT_PLUGIN_PACKAGE
        || package_download.artifact_id != format!("sha256:{}", package.package_digest)
        || package_download.digest != package.package_digest
        || package_download.size_bytes != package.package_size_bytes
        || !download_shape_is_valid(package_download)
    {
        bail!("COMPUTE_PLUGIN_PACKAGE_DOWNLOAD: package download does not match Manifest");
    }
    for (download, dependency) in item.downloads[1..]
        .iter()
        .zip(&manifest.manifest().download_dependencies)
    {
        if download.artifact_kind != ARTIFACT_PLUGIN_DEPENDENCY
            || download.artifact_id != dependency.artifact_id
            || download.digest != dependency.digest
            || download.size_bytes != dependency.size_bytes
            || !download_shape_is_valid(download)
        {
            bail!(
                "COMPUTE_PLUGIN_DEPENDENCY_DOWNLOAD: dependency download does not match Manifest"
            );
        }
    }
    let mut artifact_ids = HashSet::new();
    let mut content_bindings = HashSet::new();
    let download_bytes = item.downloads.iter().try_fold(0_i64, |total, download| {
        if !artifact_ids.insert(download.artifact_id.as_str())
            || !content_bindings.insert((download.digest.as_str(), download.size_bytes))
        {
            return None;
        }
        total.checked_add(download.size_bytes)
    });
    let minimum_disk_bytes = download_bytes
        .and_then(|bytes| bytes.checked_add(package.unpacked_size_bytes))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "COMPUTE_PLUGIN_DOWNLOAD_DUPLICATE: artifact closure is duplicated or oversized"
            )
        })?;
    Ok(minimum_disk_bytes)
}

fn download_shape_is_valid(download: &ComputePluginPlannedDownload) -> bool {
    is_sha256(&download.digest)
        && download.size_bytes > 0
        && !download.artifact_id.is_empty()
        && download.artifact_id.len() <= 256
        && opaque_ref_is_valid(&download.source_ref)
        && matches!(
            download.cache_class.as_str(),
            "pinned" | "active" | "warm" | "evictable"
        )
}

pub(super) fn reject_duplicate_manifests(
    manifests: &[ValidatedComputePluginManifest],
) -> Result<()> {
    let unique = manifests
        .iter()
        .map(|manifest| manifest.signed().manifest_digest.as_str())
        .collect::<HashSet<_>>();
    if unique.len() != manifests.len() || manifests.len() > MAX_PLAN_ITEMS {
        bail!("COMPUTE_PLUGIN_MANIFEST_DUPLICATE: manifest set is duplicated or oversized");
    }
    Ok(())
}

pub(super) fn validate_reason_codes(item: &ComputePluginPlanItem) -> Result<()> {
    if item.reason_codes.len() > MAX_REASON_CODES || !sorted_unique(&item.reason_codes) {
        bail!("COMPUTE_PLUGIN_REASON_CODES: reason codes must be sorted and unique");
    }
    Ok(())
}

pub(super) fn item_plugin_id(item: &ComputePluginPlanItem) -> Result<&str> {
    item.target_release
        .as_ref()
        .or(item.expected_candidate_release.as_ref())
        .or(item.expected_current_release.as_ref())
        .map(|release| release.plugin_id.as_str())
        .filter(|value| is_identifier(value))
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_ITEM_ID: plugin identity is missing"))
}

fn sorted_unique(values: &[String]) -> bool {
    values.iter().all(|value| is_identifier(value))
        && values.windows(2).all(|pair| pair[0] < pair[1])
}

pub(super) fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn opaque_ref_is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 200
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}
