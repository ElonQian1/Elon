use std::collections::{BTreeSet, HashMap, HashSet};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use super::{
    identity::ComputePluginReleaseRef,
    manifest_validation::{is_normalized_relative_path, is_sha256, ValidatedComputePluginManifest},
    plugin_manifest::{
        COMPUTE_PLUGIN_DIGEST_ALGORITHM, COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION,
        COMPUTE_PLUGIN_MAX_PACKAGE_FILES,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

mod zip;

pub(in crate::node_agent_compute_plugin_host) use zip::{
    extract_verified_compute_plugin_zip_archive, scan_verified_compute_plugin_zip_archive,
    ComputePluginArchiveExtractionFailure, ExtractedComputePluginCandidateArchive,
};

const COMPUTE_PLUGIN_EXTRACTION_PLAN_SCHEMA: &str =
    "elon.compute_plugin.archive_extraction_plan.v1";
const HASHED_COMPUTE_PLUGIN_EXTRACTION_PLAN_SCHEMA: &str =
    "elon.compute_plugin.hashed_archive_extraction_plan.v1";
const MAX_EXTRACTION_DIRECTORIES: usize = 8_192;
const MAX_EXTRACTION_ENTRIES: usize = COMPUTE_PLUGIN_MAX_PACKAGE_FILES + MAX_EXTRACTION_DIRECTORIES;
const MAX_PORTABLE_PATH_SEGMENT_BYTES: usize = 255;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginArchiveEntryKind {
    RegularFile,
    Directory,
    Symlink,
    Hardlink,
    Device,
    Fifo,
    Socket,
    Other,
}

/// Metadata emitted by a bounded archive scanner. This is untrusted input and never proves that
/// bytes were extracted or persisted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginArchiveEntryObservation {
    pub relative_path: String,
    pub entry_kind: ComputePluginArchiveEntryKind,
    pub declared_size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginArchiveExtractionFile {
    pub relative_path: String,
    pub expected_digest: String,
    pub expected_size_bytes: i64,
    pub executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginArchiveExtractionPlan {
    pub schema: String,
    pub release: ComputePluginReleaseRef,
    pub publisher_key_fingerprint: String,
    pub package_media_type: String,
    pub archive_format: String,
    pub package_digest: String,
    pub unpacked_size_bytes: i64,
    pub observed_archive_entry_count: i64,
    pub observed_explicit_directory_count: i64,
    pub directories: Vec<String>,
    pub files: Vec<ComputePluginArchiveExtractionFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginArchiveExtractionPlan {
    pub schema: String,
    pub plan: ComputePluginArchiveExtractionPlan,
    pub canonicalization: String,
    pub plan_digest_algorithm: String,
    pub plan_digest: String,
}

/// A deterministic allow-list for a future handle-relative extractor. It is not evidence that an
/// archive parser is safe, a directory was created, or any extracted file digest was verified.
pub(in crate::node_agent_compute_plugin_host) struct ValidatedComputePluginArchiveExtractionPlan {
    envelope: HashedComputePluginArchiveExtractionPlan,
}

impl ValidatedComputePluginArchiveExtractionPlan {
    pub(in crate::node_agent_compute_plugin_host) fn envelope(
        &self,
    ) -> &HashedComputePluginArchiveExtractionPlan {
        &self.envelope
    }
}

impl std::fmt::Debug for ValidatedComputePluginArchiveExtractionPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ValidatedComputePluginArchiveExtractionPlan")
            .field("plugin_id", &self.envelope.plan.release.plugin_id)
            .field("plugin_version", &self.envelope.plan.release.plugin_version)
            .field("target_id", &self.envelope.plan.release.target_id)
            .field("directory_count", &self.envelope.plan.directories.len())
            .field("file_count", &self.envelope.plan.files.len())
            .field(
                "unpacked_size_bytes",
                &self.envelope.plan.unpacked_size_bytes,
            )
            .field("plan_digest", &"<redacted>")
            .finish()
    }
}

pub(in crate::node_agent_compute_plugin_host) fn plan_compute_plugin_archive_extraction(
    manifest: &ValidatedComputePluginManifest,
    observed_entries: Vec<ComputePluginArchiveEntryObservation>,
) -> Result<ValidatedComputePluginArchiveExtractionPlan> {
    if observed_entries.is_empty() || observed_entries.len() > MAX_EXTRACTION_ENTRIES {
        bail!("COMPUTE_PLUGIN_ARCHIVE_ENTRY_LIMIT");
    }
    let package = &manifest.manifest().package;
    if !is_sha256(manifest.verification_key_fingerprint())
        || package.files.is_empty()
        || package.files.len() > COMPUTE_PLUGIN_MAX_PACKAGE_FILES
    {
        bail!("COMPUTE_PLUGIN_EXTRACTION_MANIFEST_INVALID");
    }

    let expected_directories = collect_expected_directories(manifest)?;
    let expected_files: HashMap<&str, _> = package
        .files
        .iter()
        .map(|file| (file.relative_path.as_str(), file))
        .collect();
    let mut observed_paths = HashSet::with_capacity(observed_entries.len());
    let mut portable_paths = HashSet::with_capacity(observed_entries.len());
    let mut observed_files = HashSet::with_capacity(package.files.len());
    let mut explicit_directory_count = 0_i64;
    let mut observed_unpacked_bytes = 0_i64;

    for entry in &observed_entries {
        let portable_key = portable_extraction_path_key(&entry.relative_path)?;
        if !observed_paths.insert(entry.relative_path.as_str())
            || !portable_paths.insert(portable_key)
            || entry.declared_size_bytes < 0
        {
            bail!("COMPUTE_PLUGIN_ARCHIVE_ENTRY_DUPLICATE_OR_INVALID");
        }
        match entry.entry_kind {
            ComputePluginArchiveEntryKind::RegularFile => {
                let Some(expected) = expected_files.get(entry.relative_path.as_str()) else {
                    bail!("COMPUTE_PLUGIN_ARCHIVE_EXTRA_FILE");
                };
                if entry.declared_size_bytes != expected.size_bytes
                    || !observed_files.insert(entry.relative_path.as_str())
                {
                    bail!("COMPUTE_PLUGIN_ARCHIVE_FILE_SHAPE_MISMATCH");
                }
                observed_unpacked_bytes = observed_unpacked_bytes
                    .checked_add(entry.declared_size_bytes)
                    .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_ARCHIVE_SIZE_OVERFLOW"))?;
            }
            ComputePluginArchiveEntryKind::Directory => {
                if entry.declared_size_bytes != 0
                    || !expected_directories.contains(entry.relative_path.as_str())
                {
                    bail!("COMPUTE_PLUGIN_ARCHIVE_EXTRA_DIRECTORY");
                }
                explicit_directory_count = explicit_directory_count
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_ARCHIVE_COUNT_OVERFLOW"))?;
            }
            _ => bail!("COMPUTE_PLUGIN_ARCHIVE_UNSAFE_ENTRY_KIND"),
        }
    }

    if observed_files.len() != package.files.len()
        || observed_unpacked_bytes != package.unpacked_size_bytes
    {
        bail!("COMPUTE_PLUGIN_ARCHIVE_MANIFEST_CLOSURE_MISMATCH");
    }

    let plan = ComputePluginArchiveExtractionPlan {
        schema: COMPUTE_PLUGIN_EXTRACTION_PLAN_SCHEMA.to_string(),
        release: manifest.release_ref(),
        publisher_key_fingerprint: manifest.verification_key_fingerprint().to_string(),
        package_media_type: package.media_type.clone(),
        archive_format: package.archive_format.clone(),
        package_digest: package.package_digest.clone(),
        unpacked_size_bytes: package.unpacked_size_bytes,
        observed_archive_entry_count: i64::try_from(observed_entries.len())
            .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_ARCHIVE_COUNT_OVERFLOW"))?,
        observed_explicit_directory_count: explicit_directory_count,
        directories: expected_directories.into_iter().collect(),
        files: package
            .files
            .iter()
            .map(|file| ComputePluginArchiveExtractionFile {
                relative_path: file.relative_path.clone(),
                expected_digest: file.digest.clone(),
                expected_size_bytes: file.size_bytes,
                executable: file.executable,
            })
            .collect(),
    };
    let plan_digest = jcs_sha256_hex(&plan)?;
    Ok(ValidatedComputePluginArchiveExtractionPlan {
        envelope: HashedComputePluginArchiveExtractionPlan {
            schema: HASHED_COMPUTE_PLUGIN_EXTRACTION_PLAN_SCHEMA.to_string(),
            plan,
            canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
            plan_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
            plan_digest,
        },
    })
}

fn collect_expected_directories(
    manifest: &ValidatedComputePluginManifest,
) -> Result<BTreeSet<String>> {
    let mut directories = BTreeSet::new();
    let mut portable_paths = HashSet::new();
    for file in &manifest.manifest().package.files {
        let file_key = portable_extraction_path_key(&file.relative_path)?;
        if !portable_paths.insert(file_key) {
            bail!("COMPUTE_PLUGIN_MANIFEST_PORTABLE_PATH_COLLISION");
        }
        let mut parent = String::new();
        let segments: Vec<_> = file.relative_path.split('/').collect();
        for segment in segments.iter().take(segments.len().saturating_sub(1)) {
            if !parent.is_empty() {
                parent.push('/');
            }
            parent.push_str(segment);
            let parent_key = portable_extraction_path_key(&parent)?;
            if portable_paths.insert(parent_key) {
                directories.insert(parent.clone());
                if directories.len() > MAX_EXTRACTION_DIRECTORIES {
                    bail!("COMPUTE_PLUGIN_EXTRACTION_DIRECTORY_LIMIT");
                }
            } else if !directories.contains(&parent) {
                bail!("COMPUTE_PLUGIN_MANIFEST_FILE_DIRECTORY_COLLISION");
            }
        }
    }
    Ok(directories)
}

fn portable_extraction_path_key(value: &str) -> Result<String> {
    if !is_normalized_relative_path(value)
        || !value.is_ascii()
        || value.split('/').any(|segment| {
            segment.len() > MAX_PORTABLE_PATH_SEGMENT_BYTES
                || segment.ends_with('.')
                || segment.ends_with(' ')
                || segment
                    .bytes()
                    .any(|byte| matches!(byte, b'<' | b'>' | b'"' | b'|' | b'?' | b'*'))
                || is_windows_reserved_segment(segment)
        })
    {
        bail!("COMPUTE_PLUGIN_ARCHIVE_PATH_NOT_PORTABLE");
    }
    Ok(value.to_ascii_lowercase())
}

fn is_windows_reserved_segment(segment: &str) -> bool {
    let stem = segment
        .split_once('.')
        .map_or(segment, |(stem, _extension)| stem)
        .to_ascii_uppercase();
    matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || stem
        .strip_prefix("COM")
        .is_some_and(|suffix| matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"))
        || stem.strip_prefix("LPT").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
}
