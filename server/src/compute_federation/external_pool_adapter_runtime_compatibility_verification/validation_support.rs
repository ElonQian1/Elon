use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use crate::compute_federation::external_pool_adapter_artifact_package::{
    ExternalPoolAdapterArtifactManifestFile, ARTIFACT_PACKAGE_ENTRYPOINT_ROLE,
    ARTIFACT_PACKAGE_RESOURCE_ROLE,
};

use super::*;

pub(super) fn validate_entrypoint(
    value: &ExternalPoolAdapterRuntimeCompatibilityChallengeMaterial,
) -> Result<()> {
    let entries: Vec<_> = value
        .registry_release
        .release
        .manifest
        .files
        .iter()
        .filter(|file| file.role == ARTIFACT_PACKAGE_ENTRYPOINT_ROLE)
        .collect();
    if entries.len() != 1
        || entries[0].path != value.entrypoint_path
        || entries[0].sha256 != value.entrypoint_sha256
        || entries[0].size_bytes != value.entrypoint_size_bytes
        || value.entrypoint_size_bytes == 0
    {
        bail!("runtime compatibility source entrypoint is not exact");
    }
    Ok(())
}

pub(super) fn validate_fixture_resources(
    values: &[ExternalPoolAdapterRuntimeCompatibilityFixtureResourceIdentity],
    manifest: &[ExternalPoolAdapterArtifactManifestFile],
) -> Result<()> {
    let catalog = runtime_compatibility_public_fixture_catalog_for_validation();
    if values.len() != catalog.resources.len() {
        bail!("runtime compatibility fixture inventory is incomplete");
    }
    for (value, requirement) in values.iter().zip(catalog.resources) {
        digest(&value.sha256)?;
        let matches: Vec<_> = manifest
            .iter()
            .filter(|file| file.path == value.path)
            .collect();
        if value.purpose != requirement.purpose
            || value.path != requirement.path
            || value.role != ARTIFACT_PACKAGE_RESOURCE_ROLE
            || value.role != requirement.role
            || value.size_bytes < requirement.min_size_bytes
            || value.size_bytes > requirement.max_size_bytes
            || !requirement.public_fixture_only
            || matches.len() != 1
            || matches[0].role != value.role
            || matches[0].sha256 != value.sha256
            || matches[0].size_bytes != value.size_bytes
        {
            bail!("runtime compatibility fixture resource is not exact");
        }
    }
    Ok(())
}

pub(super) fn validate_observation_inventory(
    values: &[ExternalPoolAdapterRuntimeCompatibilityObservation],
) -> Result<()> {
    if values.len() != REQUIRED_RUNTIME_COMPATIBILITY_OBSERVATIONS.len() {
        bail!("runtime compatibility observation inventory is incomplete");
    }
    for (value, expected) in values
        .iter()
        .zip(REQUIRED_RUNTIME_COMPATIBILITY_OBSERVATIONS)
    {
        if value.observation_id != expected
            || value.observation_revision != 1
            || value.outcome != "passed"
            || value.duration_ms > RUNTIME_COMPATIBILITY_MAX_RUN_SECONDS * 1000
            || value.policy_violation_count != 0
        {
            bail!("runtime compatibility ordered observation is invalid");
        }
    }
    Ok(())
}

pub(super) fn validate_no_work(
    value: &ExternalPoolAdapterRuntimeCompatibilityServerRunObservationMaterial,
) -> Result<()> {
    let request = &value.fixture_resources[2];
    let response = &value.fixture_resources[3];
    digests([
        &value.no_work.probe_nonce_digest,
        &value.no_work.request_sha256,
        &value.no_work.response_sha256,
        &value.no_work.probe_root_sha256,
    ])?;
    if value.no_work.request_bytes != request.size_bytes
        || value.no_work.response_bytes != response.size_bytes
        || value.no_work.request_sha256 != request.sha256
        || value.no_work.response_sha256 != response.sha256
    {
        bail!("runtime compatibility no-work evidence is not the public fixture exchange");
    }
    Ok(())
}

pub(super) fn policy_refs(
    value: &ExternalPoolAdapterRuntimeCompatibilityChallengeMaterial,
) -> [&ExternalPoolAdapterRuntimeCompatibilityPolicyRef; 6] {
    [
        &value.runtime_launch_policy,
        &value.upstream_transport_policy,
        &value.supervisor_session_policy,
        &value.source_capsule_policy,
        &value.runner_policy,
        &value.fixture_catalog,
    ]
}

pub(super) fn no_effects(value: &ExternalPoolAdapterRuntimeCompatibilityEffects) -> bool {
    value == &runtime_compatibility_no_effects()
}

pub(super) fn no_readiness(value: &ExternalPoolAdapterRuntimeCompatibilityReadiness) -> bool {
    value == &runtime_compatibility_no_readiness()
}

pub(super) fn optional_id_digest(id: Option<&str>, value: Option<&str>) -> Result<()> {
    if id.is_some() != value.is_some() {
        bail!("runtime compatibility optional lineage pair is incomplete");
    }
    if let Some(id) = id {
        identifier(id, 200)?;
    }
    if let Some(value) = value {
        digest(value)?;
    }
    Ok(())
}

pub(super) fn identifiers<const N: usize>(values: [&str; N]) -> Result<()> {
    for value in values {
        identifier(value, 240)?;
    }
    Ok(())
}

pub(super) fn digests<const N: usize>(values: [&str; N]) -> Result<()> {
    for value in values {
        digest(value)?;
    }
    Ok(())
}

pub(super) fn identifier(value: &str, max: usize) -> Result<()> {
    if value.trim() != value
        || value.is_empty()
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("runtime compatibility identifier is invalid");
    }
    Ok(())
}

pub(super) fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        bail!("runtime compatibility digest is invalid");
    }
    Ok(())
}

pub(crate) fn canonical_runtime_compatibility_timestamp(
    value: &str,
) -> Result<DateTime<chrono::FixedOffset>> {
    canonical_timestamp(value)
}

pub(super) fn canonical_timestamp(value: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("runtime compatibility timestamp is not canonical UTC nanoseconds");
    }
    Ok(parsed)
}
