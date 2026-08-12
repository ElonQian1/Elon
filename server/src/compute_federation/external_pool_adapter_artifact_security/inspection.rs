use std::{
    collections::BTreeSet,
    io::{Read, Seek, SeekFrom},
};

use anyhow::{bail, Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use super::{
    canonical::{
        canonical_sbom, component_inventory_digest, license_inventory_digest,
        scanned_file_inventory_digest, scanner_rule_set_digest, security_inspection_digest,
    },
    types::*,
    validation::{validate_artifact_security_inspection, validate_sbom},
};
use crate::compute_federation::external_pool_adapter_artifact_package::{
    ExternalPoolAdapterArtifactManifestFile, InspectedExternalPoolAdapterArtifactPackage,
    ARTIFACT_PACKAGE_MANIFEST_PATH,
};

#[derive(Serialize)]
struct ScannedFile<'a> {
    path: &'a str,
    sha256: String,
    size_bytes: u64,
}

pub(crate) fn scan_external_pool_adapter_artifact_security(
    mut inspected: InspectedExternalPoolAdapterArtifactPackage,
    expected: &ExternalPoolAdapterArtifactSecurityExpected,
) -> Result<ScannedExternalPoolAdapterArtifactSecurity> {
    let package = inspected.inspection().clone();
    if package.archive_sha256 != expected.archive_sha256
        || package.archive_size_bytes != expected.archive_size_bytes
        || package.manifest != expected.manifest
        || package.manifest_digest != expected.manifest_digest
        || package.inspection_digest != expected.package_inspection_digest
    {
        bail!("reinspected package conflicts with V232 authority");
    }

    let reader = inspected.artifact_reader();
    reader.seek(SeekFrom::Start(0))?;
    let mut archive =
        ZipArchive::new(reader).context("reopen Adapter Artifact ZIP for static scan")?;
    let mut sbom_bytes = None;
    let mut scanned = Vec::new();
    for file in &package.manifest.files {
        let mut entry = archive.by_name(&file.path)?;
        let mut bytes = Vec::with_capacity(file.size_bytes as usize);
        entry
            .by_ref()
            .take(file.size_bytes + 1)
            .read_to_end(&mut bytes)?;
        require_exact_file(file, &bytes)?;
        if file.path == ARTIFACT_SBOM_PATH {
            if file.role != "resource" || bytes.len() as u64 > MAX_ARTIFACT_SBOM_BYTES {
                bail!("Adapter SBOM is not a bounded resource");
            }
            sbom_bytes = Some(bytes.clone());
        } else {
            scan_bytes(&file.path, &bytes)?;
        }
        scanned.push(ScannedFile {
            path: &file.path,
            sha256: hex::encode(Sha256::digest(&bytes)),
            size_bytes: bytes.len() as u64,
        });
    }
    let sbom_bytes = sbom_bytes.ok_or_else(|| anyhow::anyhow!("Adapter SBOM is missing"))?;
    let sbom: ExternalPoolAdapterArtifactSbom =
        serde_json::from_slice(&sbom_bytes).context("decode strict Adapter SBOM JSON")?;
    validate_sbom(&sbom, &package.manifest)?;
    let (sbom_canonical_json, sbom_digest) = canonical_sbom(&sbom)?;
    if sbom_canonical_json.as_bytes() != sbom_bytes {
        bail!("Adapter SBOM bytes are not canonical JCS JSON");
    }

    let licenses = sbom
        .components
        .iter()
        .map(|item| item.license_spdx_id.as_str())
        .collect::<BTreeSet<_>>();
    let component_count = sbom.components.len() as u64;
    let mut inspection = ExternalPoolAdapterArtifactSecurityInspection {
        archive_sha256: expected.archive_sha256.clone(),
        archive_size_bytes: expected.archive_size_bytes,
        package_receipt_digest: expected.package_receipt_digest.clone(),
        package_inspection_digest: expected.package_inspection_digest.clone(),
        manifest_digest: expected.manifest_digest.clone(),
        sbom_canonical_json,
        sbom_digest,
        component_inventory_digest: component_inventory_digest(&sbom.components)?,
        component_count,
        license_inventory_digest: license_inventory_digest(&licenses)?,
        license_count: licenses.len() as u64,
        scanned_file_inventory_digest: scanned_file_inventory_digest(&scanned)?,
        scanned_file_count: scanned.len() as u64,
        scanner_rule_set_id: ARTIFACT_SECURITY_RULE_SET_ID.to_string(),
        scanner_rule_set_digest: scanner_rule_set_digest(ARTIFACT_SECURITY_RULES)?,
        finding_count: 0,
        inspection_digest: String::new(),
    };
    inspection.inspection_digest = security_inspection_digest(&inspection)?;
    validate_artifact_security_inspection(&inspection, expected)?;
    drop(archive);
    Ok(split_scanned(inspected, inspection))
}

fn require_exact_file(file: &ExternalPoolAdapterArtifactManifestFile, bytes: &[u8]) -> Result<()> {
    if bytes.len() as u64 != file.size_bytes || hex::encode(Sha256::digest(bytes)) != file.sha256 {
        bail!("Adapter file changed during static scan");
    }
    Ok(())
}

fn scan_bytes(path: &str, bytes: &[u8]) -> Result<()> {
    for marker in [
        b"-----BEGIN PRIVATE KEY-----".as_slice(),
        b"-----BEGIN RSA PRIVATE KEY-----".as_slice(),
        b"-----BEGIN OPENSSH PRIVATE KEY-----".as_slice(),
    ] {
        if contains(bytes, marker) {
            bail!("Adapter static safety rules rejected embedded private key material in {path}");
        }
    }
    if contains(bytes, b"PK\x03\x04") {
        bail!("Adapter static safety rules rejected a nested ZIP payload in {path}");
    }
    if bytes.windows(20).any(|window| {
        window.starts_with(b"AKIA")
            && window[4..]
                .iter()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
    }) || [b"ghp_".as_slice(), b"github_pat_".as_slice()]
        .into_iter()
        .any(|prefix| contains(bytes, prefix))
    {
        bail!("Adapter static safety rules rejected embedded access-token material in {path}");
    }
    Ok(())
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}
