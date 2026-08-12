use std::collections::BTreeSet;

use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use super::{canonical::*, types::*};
use crate::compute_federation::external_pool_adapter_artifact_package::{
    ExternalPoolAdapterArtifactManifest, ARTIFACT_PACKAGE_FORMAT_EFFECT,
};

const ALLOWED_LICENSES: &[&str] = &[
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "MIT",
    "MPL-2.0",
    "Unicode-3.0",
    "Zlib",
];

pub(crate) fn validate_sbom(
    sbom: &ExternalPoolAdapterArtifactSbom,
    manifest: &ExternalPoolAdapterArtifactManifest,
) -> Result<()> {
    if sbom.schema != ARTIFACT_SBOM_SCHEMA
        || sbom.adapter_id != manifest.adapter_id
        || sbom.release_version != manifest.release_version
        || sbom.components.is_empty()
        || sbom.components.len() > MAX_ARTIFACT_SBOM_COMPONENTS
    {
        bail!("Adapter SBOM conflicts with the exact package or bounds");
    }
    let expected = manifest
        .files
        .iter()
        .filter(|file| file.path != ARTIFACT_SBOM_PATH)
        .map(|file| file.path.as_str())
        .collect::<BTreeSet<_>>();
    let mut observed = BTreeSet::new();
    let mut prior: Option<&str> = None;
    for component in &sbom.components {
        identifier(&component.component_id, 160)?;
        identifier(&component.name, 160)?;
        identifier(&component.version, 80)?;
        identifier(&component.supplier, 160)?;
        if prior.is_some_and(|value| value >= component.component_id.as_str())
            || !component.package_url.starts_with("pkg:")
            || component.package_url.len() > 300
            || !ALLOWED_LICENSES.contains(&component.license_spdx_id.as_str())
            || component.file_paths.is_empty()
        {
            bail!("Adapter SBOM component or license policy is invalid");
        }
        let mut prior_path: Option<&str> = None;
        for path in &component.file_paths {
            if prior_path.is_some_and(|value| value >= path.as_str())
                || path == ARTIFACT_SBOM_PATH
                || !expected.contains(path.as_str())
                || !observed.insert(path.as_str())
            {
                bail!("Adapter SBOM file ownership is not exact and canonical");
            }
            prior_path = Some(path);
        }
        prior = Some(&component.component_id);
    }
    if observed != expected {
        bail!("Adapter SBOM does not account for every non-SBOM package file exactly once");
    }
    Ok(())
}

pub(crate) fn validate_artifact_security_inspection(
    inspection: &ExternalPoolAdapterArtifactSecurityInspection,
    expected: &ExternalPoolAdapterArtifactSecurityExpected,
) -> Result<()> {
    for value in [
        &inspection.archive_sha256,
        &inspection.package_receipt_digest,
        &inspection.package_inspection_digest,
        &inspection.manifest_digest,
        &inspection.sbom_digest,
        &inspection.component_inventory_digest,
        &inspection.license_inventory_digest,
        &inspection.scanned_file_inventory_digest,
        &inspection.scanner_rule_set_digest,
        &inspection.inspection_digest,
    ] {
        digest(value)?;
    }
    if inspection.archive_sha256 != expected.archive_sha256
        || inspection.archive_size_bytes != expected.archive_size_bytes
        || inspection.package_receipt_digest != expected.package_receipt_digest
        || inspection.package_inspection_digest != expected.package_inspection_digest
        || inspection.manifest_digest != expected.manifest_digest
        || inspection.component_count == 0
        || inspection.component_count > MAX_ARTIFACT_SBOM_COMPONENTS as u64
        || inspection.license_count == 0
        || inspection.license_count > inspection.component_count
        || inspection.scanned_file_count != expected.manifest.files.len() as u64
        || inspection.scanner_rule_set_id != ARTIFACT_SECURITY_RULE_SET_ID
        || inspection.scanner_rule_set_digest != scanner_rule_set_digest(ARTIFACT_SECURITY_RULES)?
        || inspection.finding_count != 0
        || security_inspection_digest(inspection)? != inspection.inspection_digest
    {
        bail!("Adapter static security inspection is not exact");
    }
    Ok(())
}

pub(crate) fn validate_artifact_security_receipt(
    receipt: &ExternalPoolAdapterArtifactSecurityReceipt,
) -> Result<()> {
    if receipt.schema != ARTIFACT_SECURITY_RECEIPT_SCHEMA
        || receipt.canonicalization != ARTIFACT_SECURITY_CANONICALIZATION
        || receipt.digest_algorithm != ARTIFACT_SECURITY_DIGEST_ALGORITHM
    {
        bail!("Adapter security receipt metadata is unsupported");
    }
    identifier(&receipt.security_receipt_id, 160)?;
    digest(&receipt.security_receipt_digest)?;
    digest(&receipt.security_material_digest)?;
    let value = &receipt.security;
    for text in [
        &value.admission_id,
        &value.package_receipt_id,
        &value.scanned_by_admin_user_id,
        &value.idempotency_scope,
        &value.idempotency_key,
    ] {
        identifier(text, 200)?;
    }
    for digest_value in [
        &value.admission_digest,
        &value.source_receipt_digest,
        &value.provenance_receipt_digest,
        &value.package_receipt_digest,
        &value.archive_sha256,
        &value.package_inspection_digest,
        &value.manifest_digest,
        &value.sbom_digest,
        &value.component_inventory_digest,
        &value.license_inventory_digest,
        &value.scanned_file_inventory_digest,
        &value.scanner_rule_set_digest,
        &value.inspection_digest,
    ] {
        digest(digest_value)?;
    }
    canonical_nanos(&value.scanned_at)?;
    if value.recorded_at != value.scanned_at
        || value.confirmation != ARTIFACT_SECURITY_CONFIRMATION
        || value.scanner_rule_set_id != ARTIFACT_SECURITY_RULE_SET_ID
        || value.scanner_rule_set_digest != scanner_rule_set_digest(ARTIFACT_SECURITY_RULES)?
        || value.license_policy_id != ARTIFACT_SECURITY_LICENSE_POLICY_ID
        || value.evidence_scope != ARTIFACT_SECURITY_EVIDENCE_SCOPE
        || value.artifact_format_effect != ARTIFACT_PACKAGE_FORMAT_EFFECT
        || value.artifact_security_effect != ARTIFACT_SECURITY_EFFECT
        || value.vulnerability_intelligence_effect != ARTIFACT_SECURITY_NO_EFFECT
        || value.conformance_effect != ARTIFACT_SECURITY_NO_EFFECT
        || value.adapter_effect != ARTIFACT_SECURITY_NO_EFFECT
        || value.route_effect != ARTIFACT_SECURITY_NO_EFFECT
        || value.finding_count != 0
        || security_material_digest(value)? != receipt.security_material_digest
        || canonical_artifact_security_receipt_json_and_digest(receipt)?.1
            != receipt.security_receipt_digest
    {
        bail!("Adapter security receipt material is not exact");
    }
    Ok(())
}

fn identifier(value: &str, max: usize) -> Result<()> {
    if value.trim() != value
        || value.is_empty()
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("Adapter security identifier is invalid");
    }
    Ok(())
}

fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("Adapter security digest is invalid");
    }
    Ok(())
}

fn canonical_nanos(value: &str) -> Result<()> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("Adapter security timestamp is not canonical UTC nanoseconds");
    }
    Ok(())
}
