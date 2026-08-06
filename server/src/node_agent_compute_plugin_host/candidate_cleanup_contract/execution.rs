use std::{collections::VecDeque, error::Error as StdError, fmt};

use anyhow::{bail, Error, Result};

use super::AuthorizedCandidateCleanup;
use crate::node_agent_compute_plugin_host::{
    local_authority::{
        HashedComputePluginCandidateCleanupAuthorizationReceipt,
        HashedComputePluginCandidateHealthQuarantineReceipt,
        HashedComputePluginCandidateStagingReceipt,
    },
    manifest_validation::is_sha256,
    root_lock::ComputePluginRootLockLease,
};

mod delete_steps;
mod evidence;

use delete_steps::{
    delete_directory, delete_file, delete_optional_directory, ordered_files,
    ordered_staging_directories, PendingCleanupDirectory, PendingCleanupFile,
};
use evidence::{build_hashed_execution_evidence, ComputePluginCandidateCleanupStepEvidence};
pub(in crate::node_agent_compute_plugin_host) use evidence::{
    ComputePluginCandidateCleanupExecutionEvidence,
    HashedComputePluginCandidateCleanupExecutionEvidence,
};

#[must_use = "partial cleanup state must be resumed or retained for operator recovery"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupExecutionState {
    authorization_receipt: HashedComputePluginCandidateCleanupAuthorizationReceipt,
    quarantine_receipt: HashedComputePluginCandidateHealthQuarantineReceipt,
    staging_receipt: HashedComputePluginCandidateStagingReceipt,
    extraction_evidence_digest: String,
    root_lock_lease: ComputePluginRootLockLease,
    staging_files: VecDeque<PendingCleanupFile>,
    seal: Option<PendingCleanupFile>,
    staging_directories: VecDeque<PendingCleanupDirectory>,
    staging_run: Option<PendingCleanupDirectory>,
    staging_parent: Option<PendingCleanupDirectory>,
    download_files: VecDeque<PendingCleanupFile>,
    downloads_directory: Option<PendingCleanupDirectory>,
    candidate_directory: Option<PendingCleanupDirectory>,
    completed_steps: Vec<ComputePluginCandidateCleanupStepEvidence>,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupExecutionPreparationFailure<
    'root,
> {
    error: Error,
    authorized: AuthorizedCandidateCleanup<'root>,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupExecutionFailure {
    error: Error,
    state: CandidateCleanupExecutionState,
}

/// All known candidate objects accepted delete disposition while the root lock remained held.
/// This is not yet a durable directory flush or a completion Store receipt.
#[must_use = "physical cleanup evidence must be committed by the future completion Store"]
pub(in crate::node_agent_compute_plugin_host) struct PhysicallyExecutedCandidateCleanup {
    authorization_receipt: HashedComputePluginCandidateCleanupAuthorizationReceipt,
    quarantine_receipt: HashedComputePluginCandidateHealthQuarantineReceipt,
    staging_receipt: HashedComputePluginCandidateStagingReceipt,
    root_lock_lease: ComputePluginRootLockLease,
    evidence: HashedComputePluginCandidateCleanupExecutionEvidence,
}

pub(in crate::node_agent_compute_plugin_host) fn prepare_candidate_cleanup_execution<'root>(
    mut authorized: AuthorizedCandidateCleanup<'root>,
) -> std::result::Result<
    CandidateCleanupExecutionState,
    CandidateCleanupExecutionPreparationFailure<'root>,
> {
    if let Err(error) = validate_before_execution(&mut authorized) {
        return Err(CandidateCleanupExecutionPreparationFailure { error, authorized });
    }
    let (candidate_directory, staging_parent) = match authorized
        .quarantined()
        .staged()
        .archive()
        .pin_cleanup_ancestors()
    {
        Ok(ancestors) => ancestors,
        Err(error) => {
            return Err(CandidateCleanupExecutionPreparationFailure { error, authorized })
        }
    };

    let (quarantined, authorization_receipt) = authorized.into_parts();
    let (staged, quarantine_receipt) = quarantined.into_parts();
    let (archive, staging_receipt, _staging_recovery_key) = staged.into_parts();
    let parts = archive.into_cleanup_parts();
    let candidate_token_digest = parts.evidence.evidence.candidate_token_digest.clone();
    let staging_run_digest = parts.evidence.evidence.staging_run_digest.clone();
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
            logical_path: format!("staging/{staging_run_digest}/{path}"),
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
            logical_path: format!("staging/{staging_run_digest}/{path}"),
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
        extraction_evidence_digest,
        root_lock_lease,
        staging_files: ordered_files(staging_files),
        seal: Some(PendingCleanupFile {
            object_kind: "staging_seal",
            logical_path: format!("staging/{staging_run_digest}/.elon-staging-seal.json"),
            content_digest: seal_file_digest,
            expected_identity_digest: seal_identity_digest,
            file: parts.seal,
        }),
        staging_directories: ordered_staging_directories(staging_directories),
        staging_run: Some(PendingCleanupDirectory {
            object_kind: "staging_run_directory",
            logical_path: format!("staging/{staging_run_digest}"),
            directory: staging_run,
        }),
        staging_parent: Some(PendingCleanupDirectory {
            object_kind: "staging_parent_directory",
            logical_path: "staging".to_string(),
            directory: staging_parent,
        }),
        download_files: ordered_files(download_files),
        downloads_directory: Some(PendingCleanupDirectory {
            object_kind: "downloads_directory",
            logical_path: "downloads".to_string(),
            directory: downloads_directory,
        }),
        candidate_directory: Some(PendingCleanupDirectory {
            object_kind: "candidate_directory",
            logical_path: format!("candidate/{candidate_token_digest}"),
            directory: candidate_directory,
        }),
        completed_steps: Vec::new(),
    })
}

pub(in crate::node_agent_compute_plugin_host) fn resume_candidate_cleanup_execution(
    mut state: CandidateCleanupExecutionState,
) -> std::result::Result<PhysicallyExecutedCandidateCleanup, CandidateCleanupExecutionFailure> {
    while let Some(file) = state.staging_files.pop_front() {
        if let Err((error, retained)) = delete_file(&mut state.completed_steps, file) {
            state.staging_files.push_front(retained);
            return Err(execution_failure(error, state));
        }
    }
    if let Some(file) = state.seal.take() {
        if let Err((error, retained)) = delete_file(&mut state.completed_steps, file) {
            state.seal = Some(retained);
            return Err(execution_failure(error, state));
        }
    }
    while let Some(directory) = state.staging_directories.pop_front() {
        if let Err((error, retained)) = delete_directory(&mut state.completed_steps, directory) {
            state.staging_directories.push_front(retained);
            return Err(execution_failure(error, state));
        }
    }
    if let Err(failure) =
        delete_optional_directory(&mut state.completed_steps, &mut state.staging_run)
    {
        return Err(execution_failure(failure, state));
    }
    if let Err(failure) =
        delete_optional_directory(&mut state.completed_steps, &mut state.staging_parent)
    {
        return Err(execution_failure(failure, state));
    }
    while let Some(file) = state.download_files.pop_front() {
        if let Err((error, retained)) = delete_file(&mut state.completed_steps, file) {
            state.download_files.push_front(retained);
            return Err(execution_failure(error, state));
        }
    }
    if let Err(failure) =
        delete_optional_directory(&mut state.completed_steps, &mut state.downloads_directory)
    {
        return Err(execution_failure(failure, state));
    }
    if let Err(failure) =
        delete_optional_directory(&mut state.completed_steps, &mut state.candidate_directory)
    {
        return Err(execution_failure(failure, state));
    }
    finish_execution(state).map_err(|(error, state)| execution_failure(error, state))
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

fn finish_execution(
    state: CandidateCleanupExecutionState,
) -> std::result::Result<PhysicallyExecutedCandidateCleanup, (Error, CandidateCleanupExecutionState)>
{
    if !state_is_empty(&state) {
        return Err((
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_EXECUTION_INCOMPLETE"),
            state,
        ));
    }
    let authorization = state.authorization_receipt.receipt();
    let hashed = match build_hashed_execution_evidence(
        authorization.cleanup_id().to_string(),
        state.authorization_receipt.receipt_digest().to_string(),
        authorization.candidate_token_digest().to_string(),
        state.quarantine_receipt.receipt_digest().to_string(),
        state.staging_receipt.receipt_digest().to_string(),
        state.extraction_evidence_digest.clone(),
        state.completed_steps.clone(),
    ) {
        Ok(hashed) => hashed,
        Err(error) => return Err((error, state)),
    };
    Ok(PhysicallyExecutedCandidateCleanup {
        authorization_receipt: state.authorization_receipt,
        quarantine_receipt: state.quarantine_receipt,
        staging_receipt: state.staging_receipt,
        root_lock_lease: state.root_lock_lease,
        evidence: hashed,
    })
}

fn state_is_empty(state: &CandidateCleanupExecutionState) -> bool {
    state.staging_files.is_empty()
        && state.seal.is_none()
        && state.staging_directories.is_empty()
        && state.staging_run.is_none()
        && state.staging_parent.is_none()
        && state.download_files.is_empty()
        && state.downloads_directory.is_none()
        && state.candidate_directory.is_none()
}

fn execution_failure(
    error: Error,
    state: CandidateCleanupExecutionState,
) -> CandidateCleanupExecutionFailure {
    CandidateCleanupExecutionFailure { error, state }
}

impl CandidateCleanupExecutionState {
    pub(in crate::node_agent_compute_plugin_host) fn completed_step_count(&self) -> usize {
        self.completed_steps.len()
    }
}

impl CandidateCleanupExecutionFailure {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateCleanupExecutionState) {
        (self.error, self.state)
    }
}

impl<'root> CandidateCleanupExecutionPreparationFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, AuthorizedCandidateCleanup<'root>) {
        (self.error, self.authorized)
    }
}

impl fmt::Display for CandidateCleanupExecutionPreparationFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl fmt::Debug for CandidateCleanupExecutionPreparationFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupExecutionPreparationFailure")
            .field("error", &self.error)
            .field("authorized", &"<retained-handles>")
            .finish()
    }
}

impl StdError for CandidateCleanupExecutionPreparationFailure<'_> {}

impl PhysicallyExecutedCandidateCleanup {
    pub(in crate::node_agent_compute_plugin_host) fn evidence(
        &self,
    ) -> &HashedComputePluginCandidateCleanupExecutionEvidence {
        &self.evidence
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        HashedComputePluginCandidateCleanupAuthorizationReceipt,
        HashedComputePluginCandidateHealthQuarantineReceipt,
        HashedComputePluginCandidateStagingReceipt,
        ComputePluginRootLockLease,
        HashedComputePluginCandidateCleanupExecutionEvidence,
    ) {
        (
            self.authorization_receipt,
            self.quarantine_receipt,
            self.staging_receipt,
            self.root_lock_lease,
            self.evidence,
        )
    }
}

impl fmt::Display for CandidateCleanupExecutionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl fmt::Debug for CandidateCleanupExecutionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupExecutionFailure")
            .field("error", &self.error)
            .field("completed_steps", &self.state.completed_steps.len())
            .finish_non_exhaustive()
    }
}

impl StdError for CandidateCleanupExecutionFailure {}

impl fmt::Debug for CandidateCleanupExecutionState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupExecutionState")
            .field("completed_steps", &self.completed_steps.len())
            .field("pending_staging_files", &self.staging_files.len())
            .field(
                "pending_staging_directories",
                &self.staging_directories.len(),
            )
            .field("pending_download_files", &self.download_files.len())
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for PhysicallyExecutedCandidateCleanup {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PhysicallyExecutedCandidateCleanup")
            .field("evidence", &self.evidence)
            .field("root_lock", &"<retained>")
            .finish_non_exhaustive()
    }
}
