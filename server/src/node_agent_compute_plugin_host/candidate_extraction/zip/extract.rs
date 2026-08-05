use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use anyhow::{bail, Context, Error, Result};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{
    scan::{
        observe_zip_entry, open_validated_zip_archive, scan_verified_compute_plugin_zip_archive,
    },
    types::{
        ComputePluginArchiveExtractionFailure, ComputePluginExtractedArchiveEvidence,
        ComputePluginExtractedFileEvidence, ExtractedComputePluginCandidateArchive,
        HashedComputePluginExtractedArchiveEvidence, EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA,
        HASHED_EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA,
    },
};
use crate::{
    node_agent_compute_plugin_host::{
        candidate_extraction::{
            ComputePluginArchiveEntryKind, ComputePluginArchiveExtractionFile,
            ValidatedComputePluginArchiveExtractionPlan,
        },
        candidate_verification_contract::VerifiedComputePluginCandidateArtifactSet,
        fetch_contract::ComputePluginFetchCancellationGuard,
        fetch_file::{
            prepare_compute_plugin_candidate_staging, PinnedComputePluginRoot,
            PreparedComputePluginCandidateStaging,
        },
        manifest_validation::ValidatedComputePluginManifest,
        signed_artifact_verification::jcs_sha256_hex,
    },
    node_agent_managed_fs::{ManagedFileReadCursor, PinnedManagedDirectory, PinnedManagedFile},
};

const STAGING_RUN_ID_DOMAIN: &[u8] = b"ELON_COMPUTE_PLUGIN_STAGING_RUN_V1";

pub(in crate::node_agent_compute_plugin_host) fn extract_verified_compute_plugin_zip_archive<
    'root,
>(
    mut verified: VerifiedComputePluginCandidateArtifactSet,
    manifest: &ValidatedComputePluginManifest,
    item_index: usize,
    root: &'root PinnedComputePluginRoot,
) -> std::result::Result<
    ExtractedComputePluginCandidateArchive<'root>,
    ComputePluginArchiveExtractionFailure,
> {
    if verified.installation_id_digest() != root.installation_id_digest() {
        return Err(extraction_failure(
            anyhow::anyhow!("COMPUTE_PLUGIN_EXTRACTION_INSTALLATION_CHANGED"),
            verified,
            None,
            false,
        ));
    }
    let plan = match scan_verified_compute_plugin_zip_archive(&mut verified, manifest, item_index) {
        Ok(plan) => plan,
        Err(error) => return Err(extraction_failure(error, verified, None, false)),
    };
    let staging_run_digest = new_staging_run_digest(
        verified.candidate_token_digest(),
        &plan.envelope().plan_digest,
    );
    let staging = match prepare_compute_plugin_candidate_staging(
        root,
        verified.candidate_token_digest(),
        &staging_run_digest,
    ) {
        Ok(staging) => staging,
        Err(failure) => {
            let filesystem_mutated = failure.filesystem_mutated();
            return Err(extraction_failure(
                failure.into_error(),
                verified,
                Some(staging_run_digest),
                filesystem_mutated,
            ));
        }
    };

    let extracted = match write_zip_to_staging(&mut verified, manifest, item_index, &plan, &staging)
    {
        Ok(extracted) => extracted,
        Err(error) => {
            return Err(extraction_failure(
                error,
                verified,
                Some(staging_run_digest),
                true,
            ));
        }
    };
    let evidence_digest = match jcs_sha256_hex(&extracted.evidence) {
        Ok(digest) => digest,
        Err(error) => {
            return Err(extraction_failure(
                error,
                verified,
                Some(staging_run_digest),
                true,
            ));
        }
    };
    Ok(ExtractedComputePluginCandidateArchive {
        plan,
        evidence: HashedComputePluginExtractedArchiveEvidence {
            schema: HASHED_EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA.to_string(),
            evidence: extracted.evidence,
            canonicalization: "RFC8785-JCS".to_string(),
            digest_algorithm: "sha256".to_string(),
            evidence_digest,
        },
        verified,
        staging,
        directories: extracted.directories,
        files: extracted.files,
        completed_at: extracted.completed_at,
    })
}

struct ExtractedArchiveParts {
    evidence: ComputePluginExtractedArchiveEvidence,
    directories: Vec<PinnedManagedDirectory>,
    files: Vec<PinnedManagedFile>,
    completed_at: Instant,
}

fn write_zip_to_staging(
    verified: &mut VerifiedComputePluginCandidateArtifactSet,
    manifest: &ValidatedComputePluginManifest,
    item_index: usize,
    plan: &ValidatedComputePluginArchiveExtractionPlan,
    staging: &PreparedComputePluginCandidateStaging<'_>,
) -> Result<ExtractedArchiveParts> {
    let mut directories = Vec::with_capacity(plan.envelope().plan.directories.len());
    for relative in &plan.envelope().plan.directories {
        directories.push(staging.prepare_directory(relative)?);
    }

    let package = &manifest.manifest().package;
    let expected_len =
        u64::try_from(package.package_size_bytes).context("COMPUTE_PLUGIN_ARCHIVE_PACKAGE_SIZE")?;
    let (files, file_evidence, completed_at) = verified.with_verified_package_file(
        item_index,
        &package.package_digest,
        expected_len,
        |file, cancellation| {
            let reader_cancellation = cancellation.clone();
            file.with_read_cursor(
                expected_len,
                || reader_cancellation.ensure_current(),
                |reader| extract_zip_entries(reader, plan, staging, &cancellation),
            )
        },
    )?;
    Ok(ExtractedArchiveParts {
        evidence: ComputePluginExtractedArchiveEvidence {
            schema: EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA.to_string(),
            installation_id_digest: verified.installation_id_digest().to_string(),
            root_identity_digest: staging.root_identity_digest().to_string(),
            candidate_token_digest: verified.candidate_token_digest().to_string(),
            staging_run_digest: staging.staging_run_digest().to_string(),
            extraction_plan_digest: plan.envelope().plan_digest.clone(),
            extracted_file_count: i64::try_from(file_evidence.len())
                .context("COMPUTE_PLUGIN_EXTRACTED_FILE_COUNT")?,
            extracted_bytes: plan.envelope().plan.unpacked_size_bytes,
            files: file_evidence,
        },
        directories,
        files,
        completed_at,
    })
}

fn extract_zip_entries(
    reader: &mut ManagedFileReadCursor<'_>,
    plan: &ValidatedComputePluginArchiveExtractionPlan,
    staging: &PreparedComputePluginCandidateStaging<'_>,
    cancellation: &ComputePluginFetchCancellationGuard,
) -> Result<(
    Vec<PinnedManagedFile>,
    Vec<ComputePluginExtractedFileEvidence>,
    Instant,
)> {
    let mut archive = open_validated_zip_archive(reader)?;
    let expected_files: HashMap<&str, (usize, &ComputePluginArchiveExtractionFile)> = plan
        .envelope()
        .plan
        .files
        .iter()
        .enumerate()
        .map(|(index, file)| (file.relative_path.as_str(), (index, file)))
        .collect();
    let expected_directories: HashSet<_> = plan
        .envelope()
        .plan
        .directories
        .iter()
        .map(String::as_str)
        .collect();
    let mut outputs: Vec<Option<(PinnedManagedFile, ComputePluginExtractedFileEvidence)>> =
        (0..expected_files.len()).map(|_| None).collect();
    let mut completed_at: Option<Instant> = None;

    for index in 0..archive.len() {
        cancellation.ensure_current()?;
        let mut entry = archive
            .by_index(index)
            .with_context(|| format!("COMPUTE_PLUGIN_ZIP_ENTRY_REOPEN:{index}"))?;
        let observation = observe_zip_entry(&entry)?;
        match observation.entry_kind {
            ComputePluginArchiveEntryKind::Directory => {
                if !expected_directories.contains(observation.relative_path.as_str()) {
                    bail!("COMPUTE_PLUGIN_ZIP_DIRECTORY_PLAN_CHANGED");
                }
            }
            ComputePluginArchiveEntryKind::RegularFile => {
                let (expected_index, expected) = expected_files
                    .get(observation.relative_path.as_str())
                    .copied()
                    .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_ZIP_FILE_PLAN_CHANGED"))?;
                if observation.declared_size_bytes != expected.expected_size_bytes
                    || outputs[expected_index].is_some()
                {
                    bail!("COMPUTE_PLUGIN_ZIP_FILE_PLAN_DUPLICATE");
                }
                let mut output = staging.create_new_file(&expected.relative_path)?;
                let expected_size = u64::try_from(expected.expected_size_bytes)
                    .context("COMPUTE_PLUGIN_EXTRACTED_FILE_SIZE")?;
                let copied = output.copy_reader_sync_hash_and_revalidate(
                    &mut entry,
                    expected_size,
                    || cancellation.ensure_current(),
                )?;
                if copied.digest() != expected.expected_digest {
                    bail!("COMPUTE_PLUGIN_EXTRACTED_FILE_DIGEST_MISMATCH");
                }
                completed_at = Some(
                    completed_at
                        .map(|current| current.max(copied.completed_at()))
                        .unwrap_or_else(|| copied.completed_at()),
                );
                let evidence = ComputePluginExtractedFileEvidence {
                    relative_path: expected.relative_path.clone(),
                    digest: copied.digest().to_string(),
                    size_bytes: expected.expected_size_bytes,
                    file_identity_digest: output.identity_digest().to_string(),
                };
                outputs[expected_index] = Some((output, evidence));
            }
            _ => bail!("COMPUTE_PLUGIN_ZIP_ENTRY_KIND_CHANGED"),
        }
    }
    if outputs.iter().any(Option::is_none) {
        bail!("COMPUTE_PLUGIN_EXTRACTED_FILE_MISSING");
    }
    let completed_at = completed_at
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_EXTRACTION_COMPLETION_MISSING"))?;
    let mut files = Vec::with_capacity(outputs.len());
    let mut evidence = Vec::with_capacity(outputs.len());
    for output in outputs.into_iter().flatten() {
        files.push(output.0);
        evidence.push(output.1);
    }
    Ok((files, evidence, completed_at))
}

fn new_staging_run_digest(candidate_token_digest: &str, plan_digest: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(STAGING_RUN_ID_DOMAIN);
    digest.update([0]);
    digest.update(candidate_token_digest.as_bytes());
    digest.update([0]);
    digest.update(plan_digest.as_bytes());
    digest.update([0]);
    digest.update(Uuid::new_v4().as_bytes());
    hex::encode(digest.finalize())
}

fn extraction_failure(
    error: Error,
    verified: VerifiedComputePluginCandidateArtifactSet,
    staging_run_digest: Option<String>,
    filesystem_mutated: bool,
) -> ComputePluginArchiveExtractionFailure {
    ComputePluginArchiveExtractionFailure {
        error,
        verified,
        staging_run_digest,
        filesystem_mutated,
    }
}
