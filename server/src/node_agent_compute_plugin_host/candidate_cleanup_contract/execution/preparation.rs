use anyhow::{bail, Error, Result};

use super::{
    ordered_files, ordered_staging_directories, CandidateCleanupExecutionState,
    PendingCleanupDirectory, PendingCleanupFile,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::AuthorizedCandidateCleanup, manifest_validation::is_sha256,
};

pub(in crate::node_agent_compute_plugin_host) fn prepare_candidate_cleanup_execution_state<
    'root,
>(
    mut authorized: AuthorizedCandidateCleanup<'root>,
) -> std::result::Result<CandidateCleanupExecutionState, (Error, AuthorizedCandidateCleanup<'root>)>
{
    if let Err(error) = validate_before_execution(&mut authorized) {
        return Err((error, authorized));
    }
    let (candidate_parent_anchor, candidate_directory, staging_parent) = match authorized
        .quarantined()
        .staged()
        .archive()
        .pin_cleanup_ancestors()
    {
        Ok(ancestors) => ancestors,
        Err(error) => return Err((error, authorized)),
    };

    let cancellation_guard = authorized
        .quarantined()
        .staged()
        .archive()
        .snapshot_cancellation_guard();
    let (quarantined, authorization_receipt) = authorized.into_parts();
    let (staged, quarantine_receipt) = quarantined.into_parts();
    let (archive, staging_receipt, staging_recovery_key) = staged.into_parts();
    let parts = archive.into_cleanup_parts();
    let candidate_token_digest = parts.evidence.evidence.candidate_token_digest.clone();
    let candidate_root = format!("compute-plugin/candidates/{candidate_token_digest}");
    let staging_run_digest = parts.evidence.evidence.staging_run_digest.clone();
    let staging_root = format!("{candidate_root}/staging/{staging_run_digest}");
    let extraction_evidence_digest = parts.evidence.evidence_digest.clone();
    let seal_file_digest = parts.seal_evidence.file_digest.clone();
    let seal_identity_digest = parts.seal_evidence.file_identity_digest.clone();
    let (download_artifacts, downloads) = parts.verified.into_cleanup_parts();
    let (downloads_directory, root_lock_lease) = downloads.into_cleanup_parts();
    let staging_run = parts.staging.into_cleanup_directory();

    let staging_files = parts
        .files
        .into_iter()
        .map(|(path, digest, file)| PendingCleanupFile {
            object_kind: "staging_file",
            logical_path: format!("{staging_root}/{path}"),
            content_digest: digest,
            expected_identity_digest: file.identity_digest().to_string(),
            file,
        })
        .collect::<Vec<_>>();

    let staging_directories = parts
        .directories
        .into_iter()
        .map(|(path, directory)| PendingCleanupDirectory {
            object_kind: "staging_directory",
            logical_path: format!("{staging_root}/{path}"),
            directory,
        })
        .collect::<Vec<_>>();

    let download_files = download_artifacts
        .into_iter()
        .map(|artifact| PendingCleanupFile {
            object_kind: "download_file",
            logical_path: artifact.logical_path,
            content_digest: artifact.expected_digest,
            expected_identity_digest: artifact.file.identity_digest().to_string(),
            file: artifact.file,
        })
        .collect::<Vec<_>>();

    Ok(CandidateCleanupExecutionState {
        authorization_receipt,
        quarantine_receipt,
        staging_receipt,
        staging_recovery_key,
        cancellation_guard,
        extraction_evidence_digest,
        root_lock_lease,
        candidate_parent_anchor,
        execution_plan_digest: None,
        staging_files: ordered_files(staging_files),
        seal: Some(PendingCleanupFile {
            object_kind: "staging_seal",
            logical_path: format!("{staging_root}/.elon-staging-seal.json"),
            content_digest: seal_file_digest,
            expected_identity_digest: seal_identity_digest,
            file: parts.seal,
        }),
        staging_directories: ordered_staging_directories(staging_directories),
        staging_run: Some(PendingCleanupDirectory {
            object_kind: "staging_run_directory",
            logical_path: staging_root,
            directory: staging_run,
        }),
        staging_parent: Some(PendingCleanupDirectory {
            object_kind: "staging_parent_directory",
            logical_path: format!("{candidate_root}/staging"),
            directory: staging_parent,
        }),
        download_files: ordered_files(download_files),
        downloads_directory: Some(PendingCleanupDirectory {
            object_kind: "downloads_directory",
            logical_path: format!("{candidate_root}/downloads"),
            directory: downloads_directory,
        }),
        candidate_directory: Some(PendingCleanupDirectory {
            object_kind: "candidate_directory",
            logical_path: candidate_root,
            directory: candidate_directory,
        }),
        completed_steps: Vec::new(),
    })
}

fn validate_before_execution(authorized: &mut AuthorizedCandidateCleanup<'_>) -> Result<()> {
    authorized.quarantined.revalidate_retained_content()?;
    authorized
        .quarantined
        .staged()
        .archive()
        .validate_cleanup_custody()?;
    let receipt = authorized.receipt();
    if receipt.receipt().slot_phase_before() != "failed" || !is_sha256(receipt.receipt_digest()) {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_EXECUTION_AUTHORIZATION_CHANGED");
    }
    Ok(())
}
