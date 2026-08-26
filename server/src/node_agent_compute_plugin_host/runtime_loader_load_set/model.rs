use std::{path::Path, time::Instant};

use crate::{
    node_agent_compute_plugin_host::{
        candidate_extraction::{
            ComputePluginStagingSealEvidence, HashedComputePluginExtractedArchiveEvidence,
            ValidatedComputePluginArchiveExtractionPlan,
        },
        candidate_promotion_contract::CandidatePromotionReceiptPair,
        candidate_staging_contract::ComputePluginCandidateStagingRecoveryKey,
        candidate_verification_contract::VerifiedComputePluginCandidateArtifactSet,
        fetch_file::PinnedComputePluginRoot,
        identity::ComputePluginReleaseRef,
        local_authority::{
            HashedComputePluginCandidateHealthReceipt, HashedComputePluginCandidateStagingReceipt,
        },
        root_lock::ComputePluginRootLockLease,
        trusted_time::ComputePluginTrustedTimeObservation,
        work_admission_contract::ComputePluginWorkAdmissionReceiptPair,
    },
    node_agent_managed_fs::{
        ManagedLoaderAuthenticatedNegativeReceipt, ManagedLoaderNamespaceQueryAttemptCustody,
        ManagedLoaderNamespaceQueryReceipt, PinnedManagedFile, PinnedManagedLoaderDirectory,
        PinnedManagedLoaderFile,
    },
};

use super::resolution::SealedWindowsRunnerLoadSetAuthority;

/// All non-package-file authority formerly nested inside `DurableWorkAdmittedPluginSlot`.
/// Package files and namespace directories move into the image; raw downloads, Store receipts,
/// time barriers, staging root/lock, and the share-none seal remain here.
#[must_use = "loader-transition authority must remain in successor or recovery custody"]
pub(super) struct LoaderTransitionAuthorityCustody<'root> {
    pub(super) authenticated_launch_lineage:
        super::launch_path_discovery::QueryVerifiedWindowsRunnerLaunchLineage,
    pub(super) work_admission_receipts: ComputePluginWorkAdmissionReceiptPair,
    pub(super) work_admission_trusted_time: ComputePluginTrustedTimeObservation,
    pub(super) work_admission_revalidated_at: Instant,
    pub(super) promotion_receipts: CandidatePromotionReceiptPair,
    pub(super) promotion_trusted_time: ComputePluginTrustedTimeObservation,
    pub(super) promotion_revalidated_at: Instant,
    pub(super) health_receipt: HashedComputePluginCandidateHealthReceipt,
    pub(super) staging_receipt: HashedComputePluginCandidateStagingReceipt,
    pub(super) staging_recovery_key: ComputePluginCandidateStagingRecoveryKey,
    pub(super) extraction_plan: ValidatedComputePluginArchiveExtractionPlan,
    pub(super) extraction_evidence: HashedComputePluginExtractedArchiveEvidence,
    pub(super) verified_artifacts: VerifiedComputePluginCandidateArtifactSet,
    pub(super) staging_root: &'root PinnedComputePluginRoot,
    /// Owned lease minted by that exact root before the admitted graph is split. Unlike the borrow,
    /// it remains alive even if an unconfirmed process graph must be deliberately leaked.
    pub(super) _staging_root_lock_lease: ComputePluginRootLockLease,
    pub(super) staging_relative_root: String,
    pub(super) staging_run_digest: String,
    pub(super) staging_seal: PinnedManagedFile,
    pub(super) staging_seal_evidence: ComputePluginStagingSealEvidence,
    pub(super) extraction_completed_at: Instant,
}

/// One exact package-file ordinal from the retained extraction plan. Every ordinal must appear
/// exactly once in a successful image; the Runner is selected by ordinal rather than duplicated.
pub(super) struct WindowsLoaderPackageFileCustody {
    pub(super) package_file_ordinal: usize,
    pub(super) relative_path: String,
    pub(super) file: PinnedManagedLoaderFile,
}

/// One exact namespace-directory ordinal from the retained extraction plan. The working
/// directory is selected from this set by ordinal and is never a second detached handle owner.
pub(super) struct WindowsLoaderNamespaceDirectoryCustody {
    pub(super) directory_ordinal: usize,
    pub(super) relative_path: String,
    pub(super) directory: PinnedManagedLoaderDirectory,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowsLoaderWorkingDirectoryLocation {
    PackageRoot,
    PlanDirectory { directory_ordinal: usize },
}

/// Opaque full-package custody prepared for the process owner. The runner is one exact ordinal in
/// the ordered package set rather than a second file owner detached from admission.
#[must_use = "sealed Runner image must remain attached to its admitted successor"]
pub(in crate::node_agent_compute_plugin_host) struct SealedComputePluginRunnerImage {
    pub(super) load_set_authority: SealedWindowsRunnerLoadSetAuthority,
    pub(super) package_files: Vec<WindowsLoaderPackageFileCustody>,
    pub(super) runner_ordinal: usize,
    pub(super) package_root_directory: PinnedManagedLoaderDirectory,
    pub(super) namespace_directories: Vec<WindowsLoaderNamespaceDirectoryCustody>,
    pub(super) working_directory_location: WindowsLoaderWorkingDirectoryLocation,
    pub(super) installation_id_digest: String,
    pub(super) root_identity_digest: String,
    pub(super) working_directory_identity_digest: String,
    pub(super) plugin_id: String,
    pub(super) slot_ref: String,
    pub(super) release: ComputePluginReleaseRef,
    pub(super) relative_path: String,
    pub(super) digest: String,
    pub(super) size_bytes: u64,
    pub(super) file_identity_digest: String,
}

/// The only success shape acceptable to process custody. It replaces—not accompanies—the old
/// share-none admitted owner.
#[must_use = "loader-locked admission must be consumed by launch security and process custody"]
pub(in crate::node_agent_compute_plugin_host) struct LoaderLockedWorkAdmittedPluginSlot<'root> {
    pub(super) authority: LoaderTransitionAuthorityCustody<'root>,
    pub(super) image: SealedComputePluginRunnerImage,
}

impl LoaderLockedWorkAdmittedPluginSlot<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn receipts(
        &self,
    ) -> &ComputePluginWorkAdmissionReceiptPair {
        &self.authority.work_admission_receipts
    }

    pub(in crate::node_agent_compute_plugin_host) fn image(
        &self,
    ) -> &SealedComputePluginRunnerImage {
        &self.image
    }
}

impl SealedComputePluginRunnerImage {
    pub(in crate::node_agent_compute_plugin_host) fn application_path(&self) -> Option<&Path> {
        self.runner_file()
            .map(PinnedManagedLoaderFile::handle_derived_canonical_path)
    }

    pub(in crate::node_agent_compute_plugin_host) fn working_directory_path(
        &self,
    ) -> Option<&Path> {
        self.working_directory()
            .map(PinnedManagedLoaderDirectory::handle_derived_canonical_path)
    }

    pub(super) fn runner_file(&self) -> Option<&PinnedManagedLoaderFile> {
        self.package_files
            .iter()
            .find(|entry| entry.package_file_ordinal == self.runner_ordinal)
            .map(|entry| &entry.file)
    }

    pub(super) fn working_directory(&self) -> Option<&PinnedManagedLoaderDirectory> {
        match self.working_directory_location {
            WindowsLoaderWorkingDirectoryLocation::PackageRoot => {
                Some(&self.package_root_directory)
            }
            WindowsLoaderWorkingDirectoryLocation::PlanDirectory { directory_ordinal } => self
                .namespace_directories
                .iter()
                .find(|entry| entry.directory_ordinal == directory_ordinal)
                .map(|entry| &entry.directory),
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn package_file_count(&self) -> usize {
        self.package_files.len()
    }

    pub(in crate::node_agent_compute_plugin_host) fn namespace_directory_count(&self) -> usize {
        self.namespace_directories.len()
    }

    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        &self.installation_id_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn root_identity_digest(&self) -> &str {
        &self.root_identity_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn working_directory_identity_digest(
        &self,
    ) -> &str {
        &self.working_directory_identity_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn retained_working_directory_matches(
        &self,
    ) -> bool {
        self.working_directory().is_some_and(|directory| {
            directory.matches_sealed_identity(&self.working_directory_identity_digest)
                && directory.matches_root_identity(&self.root_identity_digest)
        })
    }

    pub(in crate::node_agent_compute_plugin_host) fn retained_runner_matches(&self) -> bool {
        self.runner_file().is_some_and(|runner| {
            runner.is_executable_image()
                && runner.matches_root_identity(&self.root_identity_digest)
                && runner.matches_sealed_observation(
                    &self.digest,
                    self.size_bytes,
                    &self.file_identity_digest,
                )
        })
    }

    pub(in crate::node_agent_compute_plugin_host) fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn slot_ref(&self) -> &str {
        &self.slot_ref
    }

    pub(in crate::node_agent_compute_plugin_host) fn release(&self) -> &ComputePluginReleaseRef {
        &self.release
    }

    pub(in crate::node_agent_compute_plugin_host) fn relative_path(&self) -> &str {
        &self.relative_path
    }

    pub(in crate::node_agent_compute_plugin_host) fn digest(&self) -> &str {
        &self.digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn file_identity_digest(&self) -> &str {
        &self.file_identity_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn startup_import_resolution_profile_digest(
        &self,
    ) -> &str {
        &self.load_set_authority.resolution.resolution_profile_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn startup_import_namespace_authority_digest(
        &self,
    ) -> &str {
        &self.load_set_authority.namespace.namespace_authority_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn namespace_fence_generation_set_digest(
        &self,
    ) -> &str {
        &self
            .load_set_authority
            .namespace
            .prerequisite
            .fence_generation_set_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn known_dll_os_build_identity_digest(
        &self,
    ) -> &str {
        &self
            .load_set_authority
            .resolution
            .known_dll_authority
            .os_build_identity_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn known_dll_section_generation_digest(
        &self,
    ) -> &str {
        &self
            .load_set_authority
            .resolution
            .known_dll_authority
            .section_namespace_generation_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn known_dll_object_manager_identity_digest(
        &self,
    ) -> &str {
        &self
            .load_set_authority
            .resolution
            .known_dll_authority
            .object_manager_directory_identity_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn known_dll_section_binding_set_digest(
        &self,
    ) -> &str {
        &self
            .load_set_authority
            .resolution
            .known_dll_authority
            .section_binding_set_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn api_set_schema_identity_digest(&self) -> &str {
        &self
            .load_set_authority
            .resolution
            .api_set_authority
            .schema_identity_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn api_set_os_build_identity_digest(
        &self,
    ) -> &str {
        &self
            .load_set_authority
            .resolution
            .api_set_authority
            .os_build_identity_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn api_set_contract_host_binding_set_digest(
        &self,
    ) -> &str {
        &self
            .load_set_authority
            .resolution
            .api_set_authority
            .contract_host_binding_set_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn activation_context_identity_digest(
        &self,
    ) -> &str {
        &self
            .load_set_authority
            .resolution
            .side_by_side_authority
            .activation_context_identity_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn side_by_side_manifest_set_digest(
        &self,
    ) -> &str {
        &self
            .load_set_authority
            .resolution
            .side_by_side_authority
            .manifest_set_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn side_by_side_assembly_binding_set_digest(
        &self,
    ) -> &str {
        &self
            .load_set_authority
            .resolution
            .side_by_side_authority
            .assembly_binding_set_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn system_component_image_set_digest(
        &self,
    ) -> &str {
        &self
            .load_set_authority
            .resolution
            .system_module_images
            .component_image_set_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn package_content_lease_set_digest(
        &self,
    ) -> &str {
        &self
            .load_set_authority
            .resolution
            .package_content_lease_set_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn immutable_content_lease_set_digest(
        &self,
    ) -> &str {
        &self
            .load_set_authority
            .resolution
            .immutable_content_lease_set_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn launch_context_selector_digest(&self) -> &str {
        &self
            .load_set_authority
            .resolution
            .launch_context_selector_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn process_machine_context_digest(&self) -> &str {
        &self
            .load_set_authority
            .resolution
            .process_machine_context_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn namespace_session_binding(
        &self,
    ) -> (&str, u64, &str) {
        self.load_set_authority
            .namespace
            .prerequisite
            .session
            .binding()
    }

    pub(in crate::node_agent_compute_plugin_host) fn namespace_attempt_matches_session(
        &self,
        attempt: &ManagedLoaderNamespaceQueryAttemptCustody,
    ) -> bool {
        attempt.matches_session(&self.load_set_authority.namespace.prerequisite.session)
    }

    pub(in crate::node_agent_compute_plugin_host) fn namespace_receipt_matches_session(
        &self,
        receipt: &ManagedLoaderNamespaceQueryReceipt,
    ) -> bool {
        receipt.matches_session(&self.load_set_authority.namespace.prerequisite.session)
    }

    pub(in crate::node_agent_compute_plugin_host) fn namespace_negative_matches_query(
        &self,
        negative: &ManagedLoaderAuthenticatedNegativeReceipt,
        request_digest: &str,
        query_nonce_digest: &str,
    ) -> bool {
        negative.matches_query(
            &self.load_set_authority.namespace.prerequisite.session,
            request_digest,
            query_nonce_digest,
        )
    }

    pub(in crate::node_agent_compute_plugin_host) fn final_namespace_query_generation(
        &self,
    ) -> u64 {
        self.load_set_authority
            .namespace
            .final_query_receipt
            .binding()
            .2
    }

    pub(in crate::node_agent_compute_plugin_host) fn final_namespace_query_request_digest(
        &self,
    ) -> &str {
        self.load_set_authority
            .namespace
            .final_query_receipt
            .binding()
            .5
    }

    pub(in crate::node_agent_compute_plugin_host) fn final_namespace_query_nonce_digest(
        &self,
    ) -> &str {
        self.load_set_authority
            .namespace
            .final_query_receipt
            .binding()
            .6
    }
}
