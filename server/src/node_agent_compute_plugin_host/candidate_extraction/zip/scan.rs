use std::io::{Read, Seek};

use anyhow::{bail, Context, Result};
use zip::{CompressionMethod, ZipArchive};

use super::super::{
    plan_compute_plugin_archive_extraction, ComputePluginArchiveEntryKind,
    ComputePluginArchiveEntryObservation, ValidatedComputePluginArchiveExtractionPlan,
    MAX_EXTRACTION_ENTRIES,
};
use crate::{
    node_agent_compute_plugin_host::{
        candidate_verification_contract::VerifiedComputePluginCandidateArtifactSet,
        manifest_validation::ValidatedComputePluginManifest,
        plugin_manifest::{
            COMPUTE_PLUGIN_ARCHIVE_FORMAT_ZIP, COMPUTE_PLUGIN_PACKAGE_MEDIA_TYPE_ZIP,
        },
    },
    node_agent_managed_fs::ManagedFileReadCursor,
};

/// Parses only the package artifact that already belongs to a durably verified candidate set.
/// This scanner never calls `ZipArchive::extract` and never receives a writable file capability.
pub(in crate::node_agent_compute_plugin_host) fn scan_verified_compute_plugin_zip_archive(
    verified: &mut VerifiedComputePluginCandidateArtifactSet,
    manifest: &ValidatedComputePluginManifest,
    item_index: usize,
) -> Result<ValidatedComputePluginArchiveExtractionPlan> {
    let package = &manifest.manifest().package;
    if package.media_type != COMPUTE_PLUGIN_PACKAGE_MEDIA_TYPE_ZIP
        || package.archive_format != COMPUTE_PLUGIN_ARCHIVE_FORMAT_ZIP
    {
        bail!("COMPUTE_PLUGIN_ARCHIVE_FORMAT_UNSUPPORTED");
    }
    let expected_len =
        u64::try_from(package.package_size_bytes).context("COMPUTE_PLUGIN_ARCHIVE_PACKAGE_SIZE")?;
    let observations = verified.with_verified_package_file(
        item_index,
        &package.package_digest,
        expected_len,
        |file, cancellation| {
            file.with_read_cursor(
                expected_len,
                || cancellation.ensure_current(),
                scan_zip_central_directory,
            )
        },
    )?;
    plan_compute_plugin_archive_extraction(manifest, observations)
}

fn scan_zip_central_directory(
    reader: &mut ManagedFileReadCursor<'_>,
) -> Result<Vec<ComputePluginArchiveEntryObservation>> {
    let mut archive = open_validated_zip_archive(reader)?;
    let mut observations = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .with_context(|| format!("COMPUTE_PLUGIN_ZIP_ENTRY_OPEN:{index}"))?;
        observations.push(observe_zip_entry(&entry)?);
    }
    Ok(observations)
}

pub(super) fn open_validated_zip_archive<R: Read + Seek>(reader: R) -> Result<ZipArchive<R>> {
    let mut archive = ZipArchive::new(reader).context("COMPUTE_PLUGIN_ZIP_OPEN")?;
    if archive.is_empty() || archive.len() > MAX_EXTRACTION_ENTRIES {
        bail!("COMPUTE_PLUGIN_ARCHIVE_ENTRY_LIMIT");
    }
    if archive.offset() != 0 {
        bail!("COMPUTE_PLUGIN_ZIP_PREFIX_DATA");
    }
    if archive
        .has_overlapping_files()
        .context("COMPUTE_PLUGIN_ZIP_OVERLAP_CHECK")?
    {
        bail!("COMPUTE_PLUGIN_ZIP_OVERLAPPING_DATA");
    }
    Ok(archive)
}

pub(super) fn observe_zip_entry<R: Read + Seek>(
    entry: &zip::read::ZipFile<'_, R>,
) -> Result<ComputePluginArchiveEntryObservation> {
    if entry.encrypted()
        || entry.is_symlink()
        || !matches!(
            entry.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        )
    {
        bail!("COMPUTE_PLUGIN_ZIP_ENTRY_UNSUPPORTED");
    }
    let raw_name =
        std::str::from_utf8(entry.name_raw()).context("COMPUTE_PLUGIN_ZIP_ENTRY_NAME_ENCODING")?;
    if raw_name != entry.name() {
        bail!("COMPUTE_PLUGIN_ZIP_ENTRY_NAME_AMBIGUOUS");
    }
    let (relative_path, entry_kind) = classify_entry(raw_name, entry)?;
    Ok(ComputePluginArchiveEntryObservation {
        relative_path,
        entry_kind,
        declared_size_bytes: i64::try_from(entry.size())
            .context("COMPUTE_PLUGIN_ZIP_ENTRY_SIZE")?,
    })
}

fn classify_entry<R: Read + Seek>(
    raw_name: &str,
    entry: &zip::read::ZipFile<'_, R>,
) -> Result<(String, ComputePluginArchiveEntryKind)> {
    let unix_kind = entry.unix_mode().map(|mode| mode & 0o170000).unwrap_or(0);
    if entry.is_dir() {
        if !raw_name.ends_with('/') || entry.size() != 0 || !matches!(unix_kind, 0 | 0o040000) {
            bail!("COMPUTE_PLUGIN_ZIP_DIRECTORY_INVALID");
        }
        let path = raw_name
            .strip_suffix('/')
            .filter(|path| !path.is_empty())
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_ZIP_ROOT_ENTRY"))?;
        return Ok((path.to_string(), ComputePluginArchiveEntryKind::Directory));
    }
    if !entry.is_file() || raw_name.ends_with('/') || !matches!(unix_kind, 0 | 0o100000) {
        bail!("COMPUTE_PLUGIN_ZIP_SPECIAL_ENTRY");
    }
    Ok((
        raw_name.to_string(),
        ComputePluginArchiveEntryKind::RegularFile,
    ))
}
