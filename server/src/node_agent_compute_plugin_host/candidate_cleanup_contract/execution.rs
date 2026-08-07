use std::{collections::VecDeque, error::Error as StdError, fmt, time::Instant};

use anyhow::{bail, Error, Result};

use super::{
    topology::builder::CandidateCleanupTopologyObjectInput, SealedCandidateCleanupTopology,
};
use crate::node_agent_compute_plugin_host::{
    candidate_staging_contract::ComputePluginCandidateStagingRecoveryKey,
    fetch_contract::ComputePluginFetchCancellationGuard,
    local_authority::{
        HashedComputePluginCandidateCleanupAuthorizationReceipt,
        HashedComputePluginCandidateHealthQuarantineReceipt,
        HashedComputePluginCandidateStagingReceipt,
    },
    root_lock::ComputePluginRootLockLease,
};
use crate::node_agent_managed_fs::PinnedManagedDirectory;

mod delete_steps;
mod evidence;
mod preparation;
mod strong_step;

pub(in crate::node_agent_compute_plugin_host) use preparation::prepare_candidate_cleanup_execution_state;

use delete_steps::{
    delete_directory, delete_file, delete_optional_directory, ordered_files,
    ordered_staging_directories, PendingCleanupDirectory, PendingCleanupFile,
};
use evidence::{build_hashed_execution_evidence, ComputePluginCandidateCleanupStepEvidence};
pub(in crate::node_agent_compute_plugin_host) use evidence::{
    validate_hashed_execution_evidence, ComputePluginCandidateCleanupExecutionEvidence,
    HashedComputePluginCandidateCleanupExecutionEvidence,
};
pub(super) use strong_step::{
    retry_candidate_cleanup_delete_disposition, set_candidate_cleanup_delete_disposition,
};
pub(in crate::node_agent_compute_plugin_host) use strong_step::{
    CandidateCleanupDispositionFailure, CandidateCleanupDispositionFailureCustody,
    CandidateCleanupDispositionFailurePhase, CandidateCleanupDispositionRejectedCustody,
    CandidateCleanupDispositionRetryCustody, PhysicallyDisposedCandidateCleanupObject,
};

#[must_use = "partial cleanup state must be resumed or retained for operator recovery"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupExecutionState {
    authorization_receipt: HashedComputePluginCandidateCleanupAuthorizationReceipt,
    quarantine_receipt: HashedComputePluginCandidateHealthQuarantineReceipt,
    staging_receipt: HashedComputePluginCandidateStagingReceipt,
    staging_recovery_key: ComputePluginCandidateStagingRecoveryKey,
    cancellation_guard: ComputePluginFetchCancellationGuard,
    extraction_evidence_digest: String,
    root_lock_lease: ComputePluginRootLockLease,
    candidate_parent_anchor: PinnedManagedDirectory,
    execution_plan_digest: Option<String>,
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
    staging_recovery_key: ComputePluginCandidateStagingRecoveryKey,
    cancellation_guard: ComputePluginFetchCancellationGuard,
    root_lock_lease: ComputePluginRootLockLease,
    candidate_parent_anchor: PinnedManagedDirectory,
    execution_plan_digest: String,
    evidence: HashedComputePluginCandidateCleanupExecutionEvidence,
    physical_completed_at: Instant,
}

pub(super) fn prepare_candidate_cleanup_execution(
    sealed: SealedCandidateCleanupTopology,
) -> CandidateCleanupExecutionState {
    let (mut state, plan) = sealed.into_parts();
    state.execution_plan_digest = Some(plan.plan_digest().to_string());
    state
}

pub(super) fn resume_candidate_cleanup_execution(
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
    let execution_plan_digest = match state.execution_plan_digest.clone() {
        Some(digest) => digest,
        None => {
            return Err((
                anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PLAN_NOT_SEALED"),
                state,
            ))
        }
    };
    Ok(PhysicallyExecutedCandidateCleanup {
        authorization_receipt: state.authorization_receipt,
        quarantine_receipt: state.quarantine_receipt,
        staging_receipt: state.staging_receipt,
        staging_recovery_key: state.staging_recovery_key,
        cancellation_guard: state.cancellation_guard,
        root_lock_lease: state.root_lock_lease,
        candidate_parent_anchor: state.candidate_parent_anchor,
        execution_plan_digest,
        evidence: hashed,
        physical_completed_at: Instant::now(),
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

    pub(super) fn execution_plan_digest(&self) -> Option<&str> {
        self.execution_plan_digest.as_deref()
    }

    pub(in crate::node_agent_compute_plugin_host) fn cancellation_guard(
        &self,
    ) -> &ComputePluginFetchCancellationGuard {
        &self.cancellation_guard
    }

    pub(in crate::node_agent_compute_plugin_host) fn authorization_receipt(
        &self,
    ) -> &HashedComputePluginCandidateCleanupAuthorizationReceipt {
        &self.authorization_receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn staging_recovery_key(
        &self,
    ) -> &ComputePluginCandidateStagingRecoveryKey {
        &self.staging_recovery_key
    }

    pub(in crate::node_agent_compute_plugin_host) fn candidate_parent_anchor_identity_digest(
        &self,
    ) -> Result<&str> {
        let binding = self
            .candidate_parent_anchor
            .object_binding()
            .ok_or_else(|| {
                anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_ANCHOR_BINDING_MISSING")
            })?;
        if !binding.is_directory() {
            bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_ANCHOR_BINDING_CHANGED");
        }
        Ok(binding.identity_digest())
    }

    pub(in crate::node_agent_compute_plugin_host) fn topology_objects(
        &self,
    ) -> Result<Vec<CandidateCleanupTopologyObjectInput>> {
        self.staging_files
            .iter()
            .map(PendingCleanupFile::topology_input)
            .chain(self.seal.iter().map(PendingCleanupFile::topology_input))
            .chain(
                self.staging_directories
                    .iter()
                    .map(PendingCleanupDirectory::topology_input),
            )
            .chain(
                self.staging_run
                    .iter()
                    .map(PendingCleanupDirectory::topology_input),
            )
            .chain(
                self.staging_parent
                    .iter()
                    .map(PendingCleanupDirectory::topology_input),
            )
            .chain(
                self.download_files
                    .iter()
                    .map(PendingCleanupFile::topology_input),
            )
            .chain(
                self.downloads_directory
                    .iter()
                    .map(PendingCleanupDirectory::topology_input),
            )
            .chain(
                self.candidate_directory
                    .iter()
                    .map(PendingCleanupDirectory::topology_input),
            )
            .collect()
    }
}

impl CandidateCleanupExecutionFailure {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateCleanupExecutionState) {
        (self.error, self.state)
    }
}

impl PhysicallyExecutedCandidateCleanup {
    pub(in crate::node_agent_compute_plugin_host) fn authorization_receipt(
        &self,
    ) -> &HashedComputePluginCandidateCleanupAuthorizationReceipt {
        &self.authorization_receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn quarantine_receipt(
        &self,
    ) -> &HashedComputePluginCandidateHealthQuarantineReceipt {
        &self.quarantine_receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn staging_receipt(
        &self,
    ) -> &HashedComputePluginCandidateStagingReceipt {
        &self.staging_receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn staging_recovery_key(
        &self,
    ) -> &ComputePluginCandidateStagingRecoveryKey {
        &self.staging_recovery_key
    }

    pub(in crate::node_agent_compute_plugin_host) fn cancellation_guard(
        &self,
    ) -> &ComputePluginFetchCancellationGuard {
        &self.cancellation_guard
    }

    pub(in crate::node_agent_compute_plugin_host) fn physical_completed_at(&self) -> Instant {
        self.physical_completed_at
    }

    pub(in crate::node_agent_compute_plugin_host) fn evidence(
        &self,
    ) -> &HashedComputePluginCandidateCleanupExecutionEvidence {
        &self.evidence
    }

    pub(in crate::node_agent_compute_plugin_host) fn execution_plan_digest(&self) -> &str {
        &self.execution_plan_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        HashedComputePluginCandidateCleanupAuthorizationReceipt,
        HashedComputePluginCandidateHealthQuarantineReceipt,
        HashedComputePluginCandidateStagingReceipt,
        ComputePluginCandidateStagingRecoveryKey,
        ComputePluginFetchCancellationGuard,
        ComputePluginRootLockLease,
        PinnedManagedDirectory,
        String,
        HashedComputePluginCandidateCleanupExecutionEvidence,
        Instant,
    ) {
        (
            self.authorization_receipt,
            self.quarantine_receipt,
            self.staging_receipt,
            self.staging_recovery_key,
            self.cancellation_guard,
            self.root_lock_lease,
            self.candidate_parent_anchor,
            self.execution_plan_digest,
            self.evidence,
            self.physical_completed_at,
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
