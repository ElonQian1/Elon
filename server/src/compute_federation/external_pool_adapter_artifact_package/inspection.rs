use std::{collections::BTreeMap, io::Read};

use anyhow::{bail, Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use zip::{CompressionMethod, ZipArchive};

use super::{
    canonical::{canonical_manifest, inventory_digest},
    types::*,
    validation::{validate_artifact_package_inspection, validate_relative_path},
};
use crate::compute_federation::external_pool_adapter_artifact_source::CurrentQuarantinedExternalPoolAdapterArtifactBytes;

#[derive(Serialize)]
struct ObservedEntry {
    path: String,
    sha256: String,
    size_bytes: u64,
}

pub(crate) fn inspect_external_pool_adapter_artifact_package(
    mut artifact: CurrentQuarantinedExternalPoolAdapterArtifactBytes,
    expected: &ExternalPoolAdapterArtifactPackageExpected<'_>,
) -> Result<InspectedExternalPoolAdapterArtifactPackage> {
    if artifact.content_address_digest() != expected.artifact_sha256
        || artifact.artifact_size_bytes() != expected.artifact_size_bytes
    {
        bail!("verified CAS handle conflicts with package inspection authority");
    }
    let inspection = inspect_reader(artifact.reader(), expected)?;
    Ok(InspectedExternalPoolAdapterArtifactPackage {
        artifact,
        inspection,
    })
}

fn inspect_reader<R: Read + std::io::Seek>(
    reader: R,
    expected: &ExternalPoolAdapterArtifactPackageExpected<'_>,
) -> Result<ExternalPoolAdapterArtifactPackageInspection> {
    let mut archive = ZipArchive::new(reader).context("open Adapter Artifact ZIP")?;
    if !archive.comment().is_empty() {
        bail!("Adapter Artifact ZIP archive comments are not permitted");
    }
    if archive.len() < 2 || archive.len() > MAX_ARTIFACT_PACKAGE_ENTRIES + 1 {
        bail!("Adapter Artifact ZIP entry count is outside the bounded contract");
    }

    let mut observed = BTreeMap::new();
    let mut manifest_bytes = None;
    let mut total_uncompressed_bytes = 0_u64;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let raw_name = std::str::from_utf8(file.name_raw())?.to_string();
        if raw_name != file.name()
            || file.enclosed_name().is_none()
            || file.is_dir()
            || file.is_symlink()
            || !file.is_file()
            || file.encrypted()
            || !matches!(
                file.compression(),
                CompressionMethod::Stored | CompressionMethod::Deflated
            )
            || !file.comment().is_empty()
        {
            bail!("Adapter Artifact ZIP contains an unsafe or unsupported entry");
        }
        validate_relative_path(&raw_name)?;
        let folded = raw_name.to_ascii_lowercase();
        if observed
            .values()
            .any(|entry: &ObservedEntry| entry.path.to_ascii_lowercase() == folded)
            || (manifest_bytes.is_some() && raw_name == ARTIFACT_PACKAGE_MANIFEST_PATH)
            || (raw_name.eq_ignore_ascii_case(ARTIFACT_PACKAGE_MANIFEST_PATH)
                && raw_name != ARTIFACT_PACKAGE_MANIFEST_PATH)
        {
            bail!("Adapter Artifact ZIP contains duplicate or case-conflicting paths");
        }
        let maximum = if raw_name == ARTIFACT_PACKAGE_MANIFEST_PATH {
            MAX_ARTIFACT_PACKAGE_MANIFEST_BYTES
        } else {
            MAX_ARTIFACT_PACKAGE_ENTRY_BYTES
        };
        if file.size() == 0 || file.size() > maximum {
            bail!("Adapter Artifact ZIP entry size is outside the bounded contract");
        }
        if file.compressed_size() == 0
            || file.size()
                > file
                    .compressed_size()
                    .saturating_mul(200)
                    .saturating_add(1_048_576)
        {
            bail!("Adapter Artifact ZIP entry expansion ratio is unsafe");
        }
        total_uncompressed_bytes = total_uncompressed_bytes
            .checked_add(file.size())
            .ok_or_else(|| anyhow::anyhow!("Adapter Artifact ZIP size overflow"))?;
        if total_uncompressed_bytes > MAX_ARTIFACT_PACKAGE_UNCOMPRESSED_BYTES {
            bail!("Adapter Artifact ZIP expands beyond the bounded contract");
        }
        let mut bytes = Vec::with_capacity(file.size() as usize);
        file.by_ref().take(maximum + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 != file.size() {
            bail!("Adapter Artifact ZIP entry length drifted while reading");
        }
        if raw_name == ARTIFACT_PACKAGE_MANIFEST_PATH {
            manifest_bytes = Some(bytes);
        } else {
            observed.insert(
                raw_name.clone(),
                ObservedEntry {
                    path: raw_name,
                    sha256: hex::encode(Sha256::digest(&bytes)),
                    size_bytes: bytes.len() as u64,
                },
            );
        }
    }

    let manifest_bytes =
        manifest_bytes.ok_or_else(|| anyhow::anyhow!("Adapter manifest is missing"))?;
    let manifest: ExternalPoolAdapterArtifactManifest =
        serde_json::from_slice(&manifest_bytes).context("decode strict Adapter manifest JSON")?;
    let (manifest_canonical_json, manifest_digest) = canonical_manifest(&manifest)?;
    if manifest_canonical_json.as_bytes() != manifest_bytes {
        bail!("Adapter manifest bytes are not canonical JCS JSON");
    }
    if manifest.files.len() != observed.len() {
        bail!("Adapter manifest does not declare the exact ZIP file set");
    }
    for declared in &manifest.files {
        let Some(entry) = observed.get(&declared.path) else {
            bail!("Adapter manifest declares a file absent from the ZIP");
        };
        if entry.sha256 != declared.sha256 || entry.size_bytes != declared.size_bytes {
            bail!("Adapter manifest file digest or length does not match ZIP bytes");
        }
    }
    let inventory: Vec<_> = observed.into_values().collect();
    let mut inspection = ExternalPoolAdapterArtifactPackageInspection {
        archive_sha256: expected.artifact_sha256.to_string(),
        archive_size_bytes: expected.artifact_size_bytes,
        manifest,
        manifest_canonical_json,
        manifest_digest,
        entry_inventory_digest: inventory_digest(&inventory)?,
        entry_count: inventory.len() as u64,
        total_uncompressed_bytes,
        inspection_digest: String::new(),
    };
    inspection.inspection_digest = super::canonical::package_inspection_digest(&inspection)?;
    validate_artifact_package_inspection(&inspection, expected)?;
    Ok(inspection)
}
