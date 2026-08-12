use std::collections::BTreeSet;

use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use super::{canonical::*, types::*};

pub(crate) fn validate_artifact_package_inspection(
    inspection: &ExternalPoolAdapterArtifactPackageInspection,
    expected: &ExternalPoolAdapterArtifactPackageExpected<'_>,
) -> Result<()> {
    validate_digest(
        &inspection.archive_sha256,
        "Artifact package archive digest",
    )?;
    validate_digest(&inspection.manifest_digest, "Artifact manifest digest")?;
    validate_digest(
        &inspection.entry_inventory_digest,
        "Artifact entry inventory digest",
    )?;
    validate_digest(
        &inspection.inspection_digest,
        "Artifact package inspection digest",
    )?;
    let manifest = &inspection.manifest;
    if inspection.archive_sha256 != expected.artifact_sha256
        || inspection.archive_size_bytes != expected.artifact_size_bytes
        || manifest.schema != ARTIFACT_PACKAGE_MANIFEST_SCHEMA
        || manifest.adapter_id != expected.adapter_id
        || manifest.release_version != expected.release_version
        || manifest.package_format != ARTIFACT_PACKAGE_FORMAT
        || manifest.runtime.kind != ARTIFACT_PACKAGE_RUNTIME_KIND
        || manifest.supported_capabilities != expected.supported_capabilities
        || manifest.capability_set_digest != expected.capability_set_digest
        || &manifest.credential_verifier != expected.credential_verifier
        || inspection.entry_count == 0
        || inspection.entry_count as usize != manifest.files.len()
        || inspection.entry_count > MAX_ARTIFACT_PACKAGE_ENTRIES as u64
        || inspection.total_uncompressed_bytes == 0
        || inspection.total_uncompressed_bytes > MAX_ARTIFACT_PACKAGE_UNCOMPRESSED_BYTES
    {
        bail!("Adapter Artifact manifest conflicts with exact admission or package bounds");
    }
    validate_identifier(&manifest.adapter_id, "manifest Adapter ID", 160)?;
    validate_identifier(&manifest.release_version, "manifest release version", 80)?;
    validate_relative_path(&manifest.runtime.entrypoint)?;
    validate_digest(
        &manifest.capability_set_digest,
        "manifest capability-set digest",
    )?;

    let mut prior: Option<&str> = None;
    let mut paths = BTreeSet::new();
    let mut entrypoints = 0;
    let mut sum = 0_u64;
    for file in &manifest.files {
        validate_relative_path(&file.path)?;
        validate_digest(&file.sha256, "manifest file digest")?;
        if prior.is_some_and(|value| value >= file.path.as_str())
            || !paths.insert(file.path.to_ascii_lowercase())
            || file.size_bytes == 0
            || file.size_bytes > MAX_ARTIFACT_PACKAGE_ENTRY_BYTES
            || !matches!(
                file.role.as_str(),
                ARTIFACT_PACKAGE_ENTRYPOINT_ROLE | ARTIFACT_PACKAGE_RESOURCE_ROLE
            )
        {
            bail!("Adapter manifest file inventory is not canonical or bounded");
        }
        if file.role == ARTIFACT_PACKAGE_ENTRYPOINT_ROLE {
            entrypoints += 1;
            if file.path != manifest.runtime.entrypoint {
                bail!("Adapter manifest entrypoint role does not match runtime entrypoint");
            }
        }
        sum = sum
            .checked_add(file.size_bytes)
            .ok_or_else(|| anyhow::anyhow!("manifest file size overflow"))?;
        prior = Some(file.path.as_str());
    }
    if entrypoints != 1
        || sum + inspection.manifest_canonical_json.len() as u64
            != inspection.total_uncompressed_bytes
    {
        bail!("Adapter manifest entrypoint or total size is not exact");
    }
    let (canonical_manifest, digest) = canonical_manifest(manifest)?;
    if canonical_manifest != inspection.manifest_canonical_json
        || digest != inspection.manifest_digest
        || package_inspection_digest(inspection)? != inspection.inspection_digest
    {
        bail!("Adapter package inspection is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_artifact_package_receipt(
    receipt: &ExternalPoolAdapterArtifactPackageReceipt,
) -> Result<()> {
    if receipt.schema != ARTIFACT_PACKAGE_RECEIPT_SCHEMA
        || receipt.canonicalization != ARTIFACT_PACKAGE_CANONICALIZATION
        || receipt.digest_algorithm != ARTIFACT_PACKAGE_DIGEST_ALGORITHM
    {
        bail!("Artifact package receipt metadata is unsupported");
    }
    validate_identifier(&receipt.package_receipt_id, "package receipt ID", 160)?;
    validate_digest(&receipt.package_receipt_digest, "package receipt digest")?;
    validate_digest(&receipt.package_material_digest, "package material digest")?;
    let package = &receipt.package;
    for (value, label) in [
        (&package.admission_digest, "package admission digest"),
        (
            &package.source_receipt_digest,
            "package source receipt digest",
        ),
        (
            &package.provenance_receipt_digest,
            "package provenance receipt digest",
        ),
        (&package.archive_sha256, "package archive digest"),
        (&package.manifest_digest, "package manifest digest"),
        (&package.entry_inventory_digest, "package inventory digest"),
        (&package.inspection_digest, "package inspection digest"),
    ] {
        validate_digest(value, label)?;
    }
    for (value, label, max) in [
        (&package.admission_id, "package admission ID", 160),
        (
            &package.provenance_receipt_id,
            "package provenance receipt ID",
            160,
        ),
        (
            &package.inspected_by_admin_user_id,
            "package inspector",
            160,
        ),
        (&package.idempotency_scope, "package idempotency scope", 200),
        (&package.idempotency_key, "package idempotency key", 160),
    ] {
        validate_identifier(value, label, max)?;
    }
    let inspection = ExternalPoolAdapterArtifactPackageInspection {
        archive_sha256: package.archive_sha256.clone(),
        archive_size_bytes: package.archive_size_bytes,
        manifest: package.manifest.clone(),
        manifest_canonical_json: package.manifest_canonical_json.clone(),
        manifest_digest: package.manifest_digest.clone(),
        entry_inventory_digest: package.entry_inventory_digest.clone(),
        entry_count: package.entry_count,
        total_uncompressed_bytes: package.total_uncompressed_bytes,
        inspection_digest: package.inspection_digest.clone(),
    };
    let expected = ExternalPoolAdapterArtifactPackageExpected {
        adapter_id: &package.manifest.adapter_id,
        release_version: &package.manifest.release_version,
        artifact_sha256: &package.archive_sha256,
        artifact_size_bytes: package.archive_size_bytes,
        supported_capabilities: &package.manifest.supported_capabilities,
        capability_set_digest: &package.manifest.capability_set_digest,
        credential_verifier: &package.manifest.credential_verifier,
    };
    validate_artifact_package_inspection(&inspection, &expected)?;
    canonical_nanos(&package.inspected_at)?;
    if package.recorded_at != package.inspected_at
        || package.confirmation != ARTIFACT_PACKAGE_CONFIRMATION
        || package.evidence_scope != ARTIFACT_PACKAGE_EVIDENCE_SCOPE
        || package.artifact_format_effect != ARTIFACT_PACKAGE_FORMAT_EFFECT
        || package.artifact_security_effect != ARTIFACT_PACKAGE_NO_EFFECT
        || package.conformance_effect != ARTIFACT_PACKAGE_NO_EFFECT
        || package.adapter_effect != ARTIFACT_PACKAGE_NO_EFFECT
        || package.route_effect != ARTIFACT_PACKAGE_NO_EFFECT
        || package_material_digest(package)? != receipt.package_material_digest
        || canonical_artifact_package_receipt_json_and_digest(receipt)?.1
            != receipt.package_receipt_digest
    {
        bail!("Artifact package receipt material is not exact");
    }
    Ok(())
}

pub(crate) fn validate_identifier(value: &str, label: &str, maximum: usize) -> Result<()> {
    if value.trim() != value
        || value.is_empty()
        || value.chars().count() > maximum
        || value.chars().any(char::is_control)
    {
        bail!("{label} is invalid");
    }
    Ok(())
}

pub(super) fn validate_relative_path(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 160
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains("//")
        || value.contains('\\')
        || value
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/'))
    {
        bail!("Adapter package path is not a canonical relative path");
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("{label} must be a lowercase SHA-256 digest");
    }
    Ok(())
}

fn canonical_nanos(value: &str) -> Result<()> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("Artifact package timestamp is not canonical UTC nanoseconds");
    }
    Ok(())
}
