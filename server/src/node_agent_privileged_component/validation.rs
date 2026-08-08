//! Non-authorizing shape checks for first-party privileged component envelopes.
//!
//! These checks deliberately do not recompute RFC 8785 JCS or verify Ed25519/Windows catalog
//! signatures. Consequently they return no trusted capability. A future installer must first add
//! a Bootstrap-pinned first-party key resolver, WinVerifyTrust/catalog verification and an
//! assigned minifilter altitude, then introduce a separate unforgeable admission type.

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Utc};

use super::contract::*;

const ED25519_SIGNATURE_BYTES: usize = 64;
const MAX_IDENTIFIER_BYTES: usize = 192;
const MAX_PACKAGE_BYTES: i64 = 512 * 1024 * 1024;
const MAX_UNPACKED_BYTES: i64 = 512 * 1024 * 1024;
const MAX_PLAN_LIFETIME_HOURS: i64 = 24;
const MAX_MANIFEST_LIFETIME_DAYS: i64 = 366;

pub(crate) fn validate_signed_privileged_component_manifest_shape(
    signed: &SignedPrivilegedComponentManifest,
) -> Result<()> {
    if signed.schema != SIGNED_PRIVILEGED_COMPONENT_MANIFEST_SCHEMA
        || signed.canonicalization != PRIVILEGED_COMPONENT_CANONICALIZATION
        || signed.manifest_digest_algorithm != PRIVILEGED_COMPONENT_DIGEST_ALGORITHM
        || !is_sha256(&signed.manifest_digest)
    {
        bail!("PRIVILEGED_COMPONENT_SIGNED_MANIFEST_METADATA_INVALID");
    }
    validate_signature_metadata(&signed.signature, PRIVILEGED_COMPONENT_RELEASE_KEY_PURPOSE)?;
    validate_manifest_shape(&signed.manifest)
}

pub(crate) fn validate_signed_privileged_component_install_plan_shape(
    signed: &SignedPrivilegedComponentInstallPlan,
) -> Result<()> {
    if signed.schema != SIGNED_PRIVILEGED_COMPONENT_INSTALL_PLAN_SCHEMA
        || signed.canonicalization != PRIVILEGED_COMPONENT_CANONICALIZATION
        || signed.plan_digest_algorithm != PRIVILEGED_COMPONENT_DIGEST_ALGORITHM
        || !is_sha256(&signed.plan_digest)
    {
        bail!("PRIVILEGED_COMPONENT_SIGNED_PLAN_METADATA_INVALID");
    }
    validate_signature_metadata(
        &signed.signature,
        PRIVILEGED_COMPONENT_INSTALL_PLAN_KEY_PURPOSE,
    )?;
    validate_install_plan_shape(&signed.plan)
}

pub(crate) fn validate_install_plan_manifest_binding(
    signed_plan: &SignedPrivilegedComponentInstallPlan,
    signed_manifest: &SignedPrivilegedComponentManifest,
) -> Result<()> {
    validate_signed_privileged_component_install_plan_shape(signed_plan)?;
    validate_signed_privileged_component_manifest_shape(signed_manifest)?;
    if signed_plan.signature.signing_key_id == signed_manifest.signature.signing_key_id {
        bail!("PRIVILEGED_COMPONENT_SIGNING_KEY_ID_REUSE");
    }
    let plan = &signed_plan.plan;
    let manifest = &signed_manifest.manifest;
    if plan.component_id != manifest.component_id
        || plan.target_manifest_digest != signed_manifest.manifest_digest
        || plan.target_release_identity != manifest.release_identity
        || plan.target_package_digest != manifest.package.package_digest
        || plan.target_rollback_generation != manifest.rollback_generation
        || plan.target_architecture != manifest.target.architecture
        || !version_is_in_range(&plan.node_version, &manifest.node_compatibility)?
    {
        bail!("PRIVILEGED_COMPONENT_PLAN_MANIFEST_BINDING_MISMATCH");
    }
    Ok(())
}

/// This is the current hard deployment gate. Shape validation is not trust verification, and a
/// production altitude has not been assigned, so no caller can use this function to authorize an
/// install yet.
pub(crate) fn enforce_current_privileged_component_installation_gate(
    signed_plan: &SignedPrivilegedComponentInstallPlan,
    signed_manifest: &SignedPrivilegedComponentManifest,
) -> Result<()> {
    validate_install_plan_manifest_binding(signed_plan, signed_manifest)?;
    let expected_altitude = WINDOWS_NAMESPACE_FENCE_ASSIGNED_ALTITUDE
        .ok_or_else(|| anyhow::anyhow!("PRIVILEGED_COMPONENT_MINIFILTER_ALTITUDE_UNASSIGNED"))?;
    if signed_manifest.manifest.minifilter.filter_altitude != expected_altitude {
        bail!("PRIVILEGED_COMPONENT_MINIFILTER_ALTITUDE_MISMATCH");
    }
    bail!("PRIVILEGED_COMPONENT_TRUST_VERIFIER_UNAVAILABLE")
}

fn validate_manifest_shape(manifest: &PrivilegedComponentManifest) -> Result<()> {
    if manifest.schema != PRIVILEGED_COMPONENT_MANIFEST_SCHEMA
        || manifest.component_id != WINDOWS_NAMESPACE_FENCE_COMPONENT_ID
        || !is_semver_triplet(&manifest.component_version)
        || !is_full_git_sha(&manifest.build_git_sha)
        || manifest.release_identity
            != format!("{}+{}", manifest.component_version, manifest.build_git_sha)
        || manifest.rollback_generation <= 0
    {
        bail!("PRIVILEGED_COMPONENT_MANIFEST_IDENTITY_INVALID");
    }
    validate_target(&manifest.target)?;
    validate_minifilter_identity(&manifest.minifilter)?;
    validate_protocol(&manifest.protocol)?;
    validate_package(&manifest.package)?;
    if manifest.protocol.driver_build_digest != manifest.package.files[0].digest {
        bail!("PRIVILEGED_COMPONENT_DRIVER_BUILD_DIGEST_MISMATCH");
    }
    validate_windows_signing(&manifest.windows_signing, &manifest.package)?;
    validate_node_version_range(&manifest.node_compatibility)?;
    validate_time_window(
        &manifest.generated_at,
        &manifest.expires_at,
        chrono::Duration::days(MAX_MANIFEST_LIFETIME_DAYS),
        "PRIVILEGED_COMPONENT_MANIFEST_TIME",
    )
}

fn validate_target(target: &PrivilegedComponentTarget) -> Result<()> {
    if target.operating_system != "windows" || target.architecture != "x86_64" {
        bail!("PRIVILEGED_COMPONENT_TARGET_INVALID");
    }
    Ok(())
}

fn validate_minifilter_identity(identity: &WindowsMinifilterIdentity) -> Result<()> {
    if identity.backend_kind != WINDOWS_NAMESPACE_FENCE_BACKEND_KIND
        || identity.service_name != WINDOWS_NAMESPACE_FENCE_SERVICE_NAME
        || identity.filter_name != WINDOWS_NAMESPACE_FENCE_FILTER_NAME
        || identity.instance_name != WINDOWS_NAMESPACE_FENCE_INSTANCE_NAME
        || identity.communication_port_name != WINDOWS_NAMESPACE_FENCE_PORT_NAME
        || !is_minifilter_altitude(&identity.filter_altitude)
        || !identity
            .supported_filesystems
            .iter()
            .map(String::as_str)
            .eq(["ntfs", "refs"])
        || !identity.single_client_connection_required
        || !identity.reject_unload_with_active_grants
    {
        bail!("PRIVILEGED_COMPONENT_MINIFILTER_IDENTITY_INVALID");
    }
    Ok(())
}

fn validate_protocol(protocol: &PrivilegedComponentProtocol) -> Result<()> {
    if protocol.protocol_id != WINDOWS_NAMESPACE_FENCE_PROTOCOL_ID
        || protocol.protocol_revision != WINDOWS_NAMESPACE_FENCE_PROTOCOL_REVISION
        || protocol.wire_magic_ascii != WINDOWS_NAMESPACE_FENCE_PROTOCOL_MAGIC
        || protocol.wire_major_revision != WINDOWS_NAMESPACE_FENCE_WIRE_MAJOR_REVISION
        || protocol.wire_minor_revision != WINDOWS_NAMESPACE_FENCE_WIRE_MINOR_REVISION
        || protocol.wire_byte_order != WINDOWS_NAMESPACE_FENCE_WIRE_BYTE_ORDER
        || protocol.wire_schema_sha256 != WINDOWS_NAMESPACE_FENCE_WIRE_SCHEMA_SHA256
        || !is_sha256(&protocol.driver_build_digest)
        || protocol.required_feature_mask != WINDOWS_NAMESPACE_FENCE_REQUIRED_FEATURE_MASK
        || !protocol.commands.iter().map(String::as_str).eq([
            "describe_session",
            "acquire_fence",
            "query_fence",
            "release_fence",
        ])
    {
        bail!("PRIVILEGED_COMPONENT_PROTOCOL_INVALID");
    }
    Ok(())
}

fn validate_package(package: &PrivilegedComponentPackage) -> Result<()> {
    if package.media_type != "application/zip"
        || package.archive_format != "zip"
        || package.digest_algorithm != PRIVILEGED_COMPONENT_DIGEST_ALGORITHM
        || !is_sha256(&package.package_digest)
        || package.package_size_bytes <= 0
        || package.package_size_bytes > MAX_PACKAGE_BYTES
        || package.unpacked_size_bytes <= 0
        || package.unpacked_size_bytes > MAX_UNPACKED_BYTES
        || package.files.len() != 3
    {
        bail!("PRIVILEGED_COMPONENT_PACKAGE_INVALID");
    }
    let expected = [
        (
            PrivilegedComponentFileRole::DriverBinary,
            WINDOWS_NAMESPACE_FENCE_DRIVER_FILE,
        ),
        (
            PrivilegedComponentFileRole::DriverInf,
            WINDOWS_NAMESPACE_FENCE_INF_FILE,
        ),
        (
            PrivilegedComponentFileRole::DriverCatalog,
            WINDOWS_NAMESPACE_FENCE_CATALOG_FILE,
        ),
    ];
    let mut total = 0_i64;
    for (file, (role, path)) in package.files.iter().zip(expected) {
        if file.role != role
            || file.relative_path != path
            || file.digest_algorithm != PRIVILEGED_COMPONENT_DIGEST_ALGORITHM
            || !is_sha256(&file.digest)
            || file.size_bytes <= 0
        {
            bail!("PRIVILEGED_COMPONENT_PACKAGE_FILE_INVALID");
        }
        total = total
            .checked_add(file.size_bytes)
            .ok_or_else(|| anyhow::anyhow!("PRIVILEGED_COMPONENT_PACKAGE_SIZE_OVERFLOW"))?;
    }
    if total != package.unpacked_size_bytes {
        bail!("PRIVILEGED_COMPONENT_UNPACKED_SIZE_MISMATCH");
    }
    Ok(())
}

fn validate_windows_signing(
    signing: &WindowsDriverSigningPolicy,
    package: &PrivilegedComponentPackage,
) -> Result<()> {
    let catalog = &package.files[2];
    if signing.catalog_relative_path != WINDOWS_NAMESPACE_FENCE_CATALOG_FILE
        || signing.catalog_digest_algorithm != PRIVILEGED_COMPONENT_DIGEST_ALGORITHM
        || signing.catalog_digest != catalog.digest
        || !is_canonical_text(&signing.expected_catalog_publisher, 256)
        || !is_sha256(&signing.expected_catalog_certificate_sha256)
        || !signing.microsoft_kernel_trust_required
        || signing.test_signing_allowed
    {
        bail!("PRIVILEGED_COMPONENT_WINDOWS_SIGNING_POLICY_INVALID");
    }
    Ok(())
}

fn validate_install_plan_shape(plan: &PrivilegedComponentInstallPlan) -> Result<()> {
    if plan.schema != PRIVILEGED_COMPONENT_INSTALL_PLAN_SCHEMA
        || plan.component_id != WINDOWS_NAMESPACE_FENCE_COMPONENT_ID
        || !is_canonical_text(&plan.plan_id, MAX_IDENTIFIER_BYTES)
        || !is_semver_triplet(&plan.node_version)
        || !is_release_identity(&plan.node_release_identity, &plan.node_version)
        || plan.target_architecture != "x86_64"
        || !is_sha256(&plan.target_manifest_digest)
        || !is_sha256(&plan.target_package_digest)
        || !is_release_identity_shape(&plan.target_release_identity)
        || plan.target_rollback_generation <= 0
        || !plan.explicit_user_consent_required
        || !plan.elevation_required
        || !plan.requires_no_active_fences
        || plan.background_install_allowed
        || plan.test_signing_allowed
    {
        bail!("PRIVILEGED_COMPONENT_INSTALL_PLAN_INVALID");
    }
    validate_plan_action(plan)?;
    validate_time_window(
        &plan.generated_at,
        &plan.expires_at,
        chrono::Duration::hours(MAX_PLAN_LIFETIME_HOURS),
        "PRIVILEGED_COMPONENT_PLAN_TIME",
    )
}

fn validate_plan_action(plan: &PrivilegedComponentInstallPlan) -> Result<()> {
    let expected_digest = plan.expected_installed_manifest_digest.as_deref();
    let expected_release = plan.expected_installed_release_identity.as_deref();
    let expected_generation = plan.expected_installed_rollback_generation;
    let valid = match plan.action.as_str() {
        PRIVILEGED_COMPONENT_PLAN_ACTION_INSTALL => {
            expected_digest.is_none() && expected_release.is_none() && expected_generation.is_none()
        }
        PRIVILEGED_COMPONENT_PLAN_ACTION_UPGRADE => {
            expected_digest.is_some_and(is_sha256)
                && expected_release.is_some_and(is_release_identity_shape)
                && expected_generation.is_some_and(|generation| {
                    generation > 0 && generation < plan.target_rollback_generation
                })
        }
        _ => false,
    };
    if !valid {
        bail!("PRIVILEGED_COMPONENT_PLAN_ACTION_INVALID");
    }
    Ok(())
}

fn validate_signature_metadata(
    signature: &PrivilegedComponentSignature,
    expected_key_purpose: &str,
) -> Result<()> {
    if signature.algorithm != PRIVILEGED_COMPONENT_SIGNATURE_ALGORITHM
        || signature.key_purpose != expected_key_purpose
        || !is_canonical_text(&signature.signing_key_id, MAX_IDENTIFIER_BYTES)
    {
        bail!("PRIVILEGED_COMPONENT_SIGNATURE_METADATA_INVALID");
    }
    let decoded = STANDARD
        .decode(&signature.signature_base64)
        .context("PRIVILEGED_COMPONENT_SIGNATURE_BASE64_INVALID")?;
    if decoded.len() != ED25519_SIGNATURE_BYTES
        || STANDARD.encode(&decoded) != signature.signature_base64
    {
        bail!("PRIVILEGED_COMPONENT_SIGNATURE_ENCODING_NON_CANONICAL");
    }
    Ok(())
}

#[cfg(test)]
mod tests;

fn validate_node_version_range(range: &PrivilegedComponentNodeVersionRange) -> Result<()> {
    let minimum = parse_semver_triplet(&range.minimum_node_version)?;
    let maximum = parse_semver_triplet(&range.maximum_node_version)?;
    if minimum > maximum {
        bail!("PRIVILEGED_COMPONENT_NODE_VERSION_RANGE_INVALID");
    }
    Ok(())
}

fn version_is_in_range(version: &str, range: &PrivilegedComponentNodeVersionRange) -> Result<bool> {
    let value = parse_semver_triplet(version)?;
    let minimum = parse_semver_triplet(&range.minimum_node_version)?;
    let maximum = parse_semver_triplet(&range.maximum_node_version)?;
    Ok((minimum..=maximum).contains(&value))
}

fn validate_time_window(
    generated_at: &str,
    expires_at: &str,
    maximum_lifetime: chrono::Duration,
    code: &str,
) -> Result<()> {
    let generated = DateTime::parse_from_rfc3339(generated_at)
        .with_context(|| format!("{code}_GENERATED_AT_INVALID"))?
        .with_timezone(&Utc);
    let expires = DateTime::parse_from_rfc3339(expires_at)
        .with_context(|| format!("{code}_EXPIRES_AT_INVALID"))?
        .with_timezone(&Utc);
    if expires <= generated || expires - generated > maximum_lifetime {
        bail!("{code}_WINDOW_INVALID");
    }
    Ok(())
}

fn parse_semver_triplet(value: &str) -> Result<(u64, u64, u64)> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || (part.len() > 1 && part.starts_with('0')))
    {
        bail!("PRIVILEGED_COMPONENT_VERSION_INVALID");
    }
    Ok((
        parts[0]
            .parse()
            .context("PRIVILEGED_COMPONENT_VERSION_MAJOR")?,
        parts[1]
            .parse()
            .context("PRIVILEGED_COMPONENT_VERSION_MINOR")?,
        parts[2]
            .parse()
            .context("PRIVILEGED_COMPONENT_VERSION_PATCH")?,
    ))
}

fn is_semver_triplet(value: &str) -> bool {
    parse_semver_triplet(value).is_ok()
}

fn is_full_git_sha(value: &str) -> bool {
    value.len() == 40 && is_lower_hex(value)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && is_lower_hex(value)
}

fn is_lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_release_identity(value: &str, version: &str) -> bool {
    value
        .strip_prefix(version)
        .and_then(|suffix| suffix.strip_prefix('+'))
        .is_some_and(is_full_git_sha)
}

fn is_release_identity_shape(value: &str) -> bool {
    let Some((version, git_sha)) = value.split_once('+') else {
        return false;
    };
    is_semver_triplet(version) && is_full_git_sha(git_sha)
}

fn is_canonical_text(value: &str, maximum_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum_bytes
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn is_minifilter_altitude(value: &str) -> bool {
    if value.is_empty() || value.len() > 32 || value.starts_with('.') || value.ends_with('.') {
        return false;
    }
    let mut decimal_points = 0;
    for byte in value.bytes() {
        if byte == b'.' {
            decimal_points += 1;
        } else if !byte.is_ascii_digit() {
            return false;
        }
    }
    decimal_points <= 1
}
