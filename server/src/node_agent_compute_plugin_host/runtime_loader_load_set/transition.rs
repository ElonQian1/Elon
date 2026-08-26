use anyhow::Error;

use crate::{
    node_agent_compute_plugin_host::{
        candidate_extraction::ExtractedComputePluginLoaderTransitionParts,
        work_admission_contract::DurableWorkAdmittedPluginSlot,
    },
    node_agent_managed_fs::{
        ManagedLoaderFileContentLease, PinnedManagedDirectory,
        PinnedManagedExtractionLoaderDirectory, PinnedManagedFile,
    },
};

use super::{
    failure::{
        PendingWindowsRunnerPackageFileCustody,
        ValidatedRetainedWindowsRunnerNamespaceDirectoryCustody,
    },
    model::LoaderTransitionAuthorityCustody,
    resolution::{
        PostLeaseSplitWindowsRunnerLoadSetPrerequisite, SealedWindowsRunnerLoadSetPrerequisite,
    },
};

/// Raw consuming split after the query-verified input is already sealed. Package files and leases
/// remain nested in their original owners; this shape cannot enter the barrier.
pub(super) struct RawDestructuredLoaderTransitionCustody<'root> {
    pub(super) authority: LoaderTransitionAuthorityCustody<'root>,
    pub(super) prerequisite: SealedWindowsRunnerLoadSetPrerequisite,
    pub(super) package_root_directory: PinnedManagedExtractionLoaderDirectory,
    pub(super) namespace_directories: Vec<PinnedManagedDirectory>,
    pub(super) package_files: Vec<PinnedManagedFile>,
}

pub(super) struct ValidatedPreBarrierPackageFileCustody {
    pub(super) package_file_ordinal: usize,
    pub(super) relative_path: String,
    pub(super) file: PinnedManagedFile,
    pub(super) content_lease: ManagedLoaderFileContentLease,
}

/// Full indexed owner graph after ordinal/path/FileId/digest/cardinality validation but before the
/// first close. Each file and its unique content lease are one element, never parallel vectors.
#[must_use = "destructured loader authority must enter successor or uncertain custody"]
pub(super) struct DestructuredLoaderTransitionCustody<'root> {
    pub(super) authority: LoaderTransitionAuthorityCustody<'root>,
    pub(super) prerequisite: PostLeaseSplitWindowsRunnerLoadSetPrerequisite,
    pub(super) package_root_directory: PinnedManagedExtractionLoaderDirectory,
    pub(super) namespace_directories: Vec<PinnedManagedDirectory>,
    pub(super) package_files: Vec<ValidatedPreBarrierPackageFileCustody>,
}

pub(super) struct WindowsRunnerLoaderOwnerGraphIndexFailure<'root> {
    pub(super) error: Error,
    pub(super) custody: RawDestructuredLoaderTransitionCustody<'root>,
}

/// No implementation exists. A future backend must validate and consume the raw split into the
/// exact indexed graph without truncating, cloning, or dropping an unmatched file or lease.
pub(super) trait WindowsRunnerLoaderOwnerGraphIndexer {
    fn index_verified_owner_graph<'root>(
        self,
        raw: RawDestructuredLoaderTransitionCustody<'root>,
    ) -> std::result::Result<
        DestructuredLoaderTransitionCustody<'root>,
        WindowsRunnerLoaderOwnerGraphIndexFailure<'root>,
    >;
}

/// Only this shape may enter the first file-close barrier. All fallible borrowed observations and
/// namespace queries have completed, original directory handles have moved infallibly into typed
/// retained wrappers, and the explicit Runner-last ordinal schedule is fixed. No producer exists.
#[must_use = "barrier-ready custody must enter success or outcome-uncertain ownership"]
pub(super) struct BarrierReadyLoaderTransitionCustody<'root> {
    pub(super) authority: LoaderTransitionAuthorityCustody<'root>,
    pub(super) prerequisite: PostLeaseSplitWindowsRunnerLoadSetPrerequisite,
    pub(super) package_root_directory: crate::node_agent_managed_fs::PinnedManagedLoaderDirectory,
    pub(super) namespace_directories: Vec<ValidatedRetainedWindowsRunnerNamespaceDirectoryCustody>,
    pub(super) pending_files: Vec<PendingWindowsRunnerPackageFileCustody>,
    pub(super) transition_schedule: Vec<usize>,
    pub(super) runner_ordinal: usize,
}

/// Purpose-specific consuming seam proving that every nested authority component can move into one
/// successor without cleanup projection or scalar reconstruction. A future producer may call this
/// only after all fallible borrow-only checks and namespace-fence acquisition have succeeded.
pub(super) fn destructure_query_verified_owners<'root>(
    admitted: DurableWorkAdmittedPluginSlot<'root>,
    prerequisite: SealedWindowsRunnerLoadSetPrerequisite,
) -> RawDestructuredLoaderTransitionCustody<'root> {
    let (work_revalidated, work_admission_receipts) = admitted.into_loader_transition_parts();
    let (installed, work_admission_trusted_time, work_admission_revalidated_at) =
        work_revalidated.into_loader_transition_parts();
    let (promotion_revalidated, promotion_receipts) = installed.into_parts();
    let (publication, promotion_trusted_time, promotion_revalidated_at) =
        promotion_revalidated.into_loader_transition_parts();
    let (staged, health_receipt) = publication.into_parts();
    let (archive, staging_receipt, staging_recovery_key) = staged.into_parts();
    let ExtractedComputePluginLoaderTransitionParts {
        plan,
        evidence,
        verified,
        staging,
        directories,
        files,
        seal,
        seal_evidence,
        completed_at,
    } = archive.into_loader_transition_parts();
    let staging = staging.into_loader_transition_parts();
    let staging_root_lock_lease = staging.root.root_lock_lease();
    RawDestructuredLoaderTransitionCustody {
        authority: LoaderTransitionAuthorityCustody {
            work_admission_receipts,
            work_admission_trusted_time,
            work_admission_revalidated_at,
            promotion_receipts,
            promotion_trusted_time,
            promotion_revalidated_at,
            health_receipt,
            staging_receipt,
            staging_recovery_key,
            extraction_plan: plan,
            extraction_evidence: evidence,
            verified_artifacts: verified,
            staging_root: staging.root,
            _staging_root_lock_lease: staging_root_lock_lease,
            staging_relative_root: staging.relative_root,
            staging_run_digest: staging.staging_run_digest,
            staging_seal: seal,
            staging_seal_evidence: seal_evidence,
            extraction_completed_at: completed_at,
        },
        prerequisite,
        package_root_directory: staging.package_root,
        namespace_directories: directories,
        package_files: files,
    }
}
