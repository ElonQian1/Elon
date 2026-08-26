use std::fmt;

use anyhow::Error;

use crate::{
    node_agent_compute_plugin_host::work_admission_contract::DurableWorkAdmittedPluginSlot,
    node_agent_managed_fs::{
        ManagedLoaderAuthenticatedNegativeReceipt, ManagedLoaderFileContentLease,
        ManagedLoaderFileContentLeaseAcquisitionAttemptCustody,
        ManagedLoaderFileContentLeaseAuthenticatedNegativeReceipt, ManagedLoaderFileIdentityAnchor,
        ManagedLoaderNamespaceQueryAttemptCustody, ManagedLoaderNamespaceSession,
        ManagedLoaderParentRelativeReopenAttemptCustody,
        ManagedLoaderParentRelativeReopenAuthenticatedNegativeReceipt,
        ManagedLoaderSearchedNameGrant, ManagedLoaderSearchedNameGrantAcquisitionAttemptCustody,
        PinnedManagedFile, PinnedManagedLoaderDirectory, PinnedManagedLoaderFile,
        QuarantinedManagedLoaderFile, QuarantinedManagedLoaderSourceClose,
    },
};

use super::{
    model::LoaderTransitionAuthorityCustody,
    resolution::{
        PostLeaseSplitWindowsRunnerLoadSetPrerequisite, SealedWindowsLoaderResolutionAuthority,
        UnqueriedWindowsLoaderNamespaceGrantSet, UnqueriedWindowsRunnerLoadSetPrerequisite,
        WindowsLoaderAcquiredNameGrantCustody, WindowsLoaderPackageContentLeaseCustody,
        WindowsRunnerLoadSetBorrowPrerequisite,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowsRunnerNamespaceQueryFailureClass {
    DefinitiveRejected,
    OutcomeUncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowsRunnerContentLeaseAcquisitionFailureClass {
    DefinitiveRejected,
    OutcomeUncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowsRunnerNameGrantAcquisitionFailureClass {
    AuthenticatedRejected,
    OutcomeUncertain,
}

pub(super) enum WindowsRunnerPendingNameGrantRef {
    ImportSearch {
        searched_name_ordinal: usize,
    },
    LaunchPath {
        path_kind: super::resolution::WindowsLoaderLaunchPathKind,
        component_ordinal: usize,
    },
}

pub(super) struct WindowsRunnerNameGrantAcquisitionUnusableCustody<'root> {
    class: WindowsRunnerNameGrantAcquisitionFailureClass,
    _admitted: DurableWorkAdmittedPluginSlot<'root>,
    _resolution: SealedWindowsLoaderResolutionAuthority,
    _session: ManagedLoaderNamespaceSession,
    _acquired_grants: Vec<WindowsLoaderAcquiredNameGrantCustody>,
    _active_grant_ref: WindowsRunnerPendingNameGrantRef,
    _active_attempt: ManagedLoaderSearchedNameGrantAcquisitionAttemptCustody,
    _returned_positive: Option<ManagedLoaderSearchedNameGrant>,
    _authenticated_negative: Option<ManagedLoaderAuthenticatedNegativeReceipt>,
    _pending_grants: Vec<WindowsRunnerPendingNameGrantRef>,
}

impl<'root> WindowsRunnerNameGrantAcquisitionUnusableCustody<'root> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn reject_acquisition(
        admitted: DurableWorkAdmittedPluginSlot<'root>,
        resolution: SealedWindowsLoaderResolutionAuthority,
        session: ManagedLoaderNamespaceSession,
        acquired_grants: Vec<WindowsLoaderAcquiredNameGrantCustody>,
        active_grant_ref: WindowsRunnerPendingNameGrantRef,
        active_attempt: ManagedLoaderSearchedNameGrantAcquisitionAttemptCustody,
        returned_positive: Option<ManagedLoaderSearchedNameGrant>,
        authenticated_negative: ManagedLoaderAuthenticatedNegativeReceipt,
        pending_grants: Vec<WindowsRunnerPendingNameGrantRef>,
    ) -> Self {
        let (request_digest, query_nonce_digest) = active_attempt.request_binding();
        let class = if returned_positive.is_none()
            && active_attempt.matches_session(&session)
            && authenticated_negative.matches_query(&session, request_digest, query_nonce_digest)
        {
            WindowsRunnerNameGrantAcquisitionFailureClass::AuthenticatedRejected
        } else {
            WindowsRunnerNameGrantAcquisitionFailureClass::OutcomeUncertain
        };
        Self {
            class,
            _admitted: admitted,
            _resolution: resolution,
            _session: session,
            _acquired_grants: acquired_grants,
            _active_grant_ref: active_grant_ref,
            _active_attempt: active_attempt,
            _returned_positive: returned_positive,
            _authenticated_negative: Some(authenticated_negative),
            _pending_grants: pending_grants,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn outcome_uncertain(
        admitted: DurableWorkAdmittedPluginSlot<'root>,
        resolution: SealedWindowsLoaderResolutionAuthority,
        session: ManagedLoaderNamespaceSession,
        acquired_grants: Vec<WindowsLoaderAcquiredNameGrantCustody>,
        active_grant_ref: WindowsRunnerPendingNameGrantRef,
        active_attempt: ManagedLoaderSearchedNameGrantAcquisitionAttemptCustody,
        returned_positive: Option<ManagedLoaderSearchedNameGrant>,
        pending_grants: Vec<WindowsRunnerPendingNameGrantRef>,
    ) -> Self {
        Self {
            class: WindowsRunnerNameGrantAcquisitionFailureClass::OutcomeUncertain,
            _admitted: admitted,
            _resolution: resolution,
            _session: session,
            _acquired_grants: acquired_grants,
            _active_grant_ref: active_grant_ref,
            _active_attempt: active_attempt,
            _returned_positive: returned_positive,
            _authenticated_negative: None,
            _pending_grants: pending_grants,
        }
    }
}

/// Partial content-lease acquisition occurs only after all name grants exist and retains that
/// complete unqueried grant set, the intact admitted owner, every acquired FileId lease, the active
/// platform dispatch, and pending ordinals. It cannot collapse into namespace query failure because
/// no query-verified prerequisite exists yet.
pub(super) struct WindowsRunnerContentLeaseAcquisitionUnusableCustody<'root> {
    class: WindowsRunnerContentLeaseAcquisitionFailureClass,
    _admitted: DurableWorkAdmittedPluginSlot<'root>,
    _resolution: SealedWindowsLoaderResolutionAuthority,
    _namespace_grants: UnqueriedWindowsLoaderNamespaceGrantSet,
    _acquired_leases: Vec<WindowsLoaderPackageContentLeaseCustody>,
    _active_package_file_ordinal: usize,
    _active_attempt: ManagedLoaderFileContentLeaseAcquisitionAttemptCustody,
    _returned_positive: Option<ManagedLoaderFileContentLease>,
    _authenticated_negative: Option<ManagedLoaderFileContentLeaseAuthenticatedNegativeReceipt>,
    _pending_package_file_ordinals: Vec<usize>,
}

impl<'root> WindowsRunnerContentLeaseAcquisitionUnusableCustody<'root> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn reject_acquisition(
        admitted: DurableWorkAdmittedPluginSlot<'root>,
        resolution: SealedWindowsLoaderResolutionAuthority,
        namespace_grants: UnqueriedWindowsLoaderNamespaceGrantSet,
        acquired_leases: Vec<WindowsLoaderPackageContentLeaseCustody>,
        active_package_file_ordinal: usize,
        active_attempt: ManagedLoaderFileContentLeaseAcquisitionAttemptCustody,
        returned_positive: Option<ManagedLoaderFileContentLease>,
        authenticated_negative: ManagedLoaderFileContentLeaseAuthenticatedNegativeReceipt,
        pending_package_file_ordinals: Vec<usize>,
    ) -> Self {
        let class = if returned_positive.is_none()
            && authenticated_negative.matches_attempt(&active_attempt)
        {
            WindowsRunnerContentLeaseAcquisitionFailureClass::DefinitiveRejected
        } else {
            WindowsRunnerContentLeaseAcquisitionFailureClass::OutcomeUncertain
        };
        Self {
            class,
            _admitted: admitted,
            _resolution: resolution,
            _namespace_grants: namespace_grants,
            _acquired_leases: acquired_leases,
            _active_package_file_ordinal: active_package_file_ordinal,
            _active_attempt: active_attempt,
            _returned_positive: returned_positive,
            _authenticated_negative: Some(authenticated_negative),
            _pending_package_file_ordinals: pending_package_file_ordinals,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn outcome_uncertain(
        admitted: DurableWorkAdmittedPluginSlot<'root>,
        resolution: SealedWindowsLoaderResolutionAuthority,
        namespace_grants: UnqueriedWindowsLoaderNamespaceGrantSet,
        acquired_leases: Vec<WindowsLoaderPackageContentLeaseCustody>,
        active_package_file_ordinal: usize,
        active_attempt: ManagedLoaderFileContentLeaseAcquisitionAttemptCustody,
        returned_positive: Option<ManagedLoaderFileContentLease>,
        pending_package_file_ordinals: Vec<usize>,
    ) -> Self {
        Self {
            class: WindowsRunnerContentLeaseAcquisitionFailureClass::OutcomeUncertain,
            _admitted: admitted,
            _resolution: resolution,
            _namespace_grants: namespace_grants,
            _acquired_leases: acquired_leases,
            _active_package_file_ordinal: active_package_file_ordinal,
            _active_attempt: active_attempt,
            _returned_positive: returned_positive,
            _authenticated_negative: None,
            _pending_package_file_ordinals: pending_package_file_ordinals,
        }
    }
}

/// Pure borrowed validation runs before any name-grant or content-lease dispatch or irreversible
/// barrier. Exact owners remain only for cleanup/recovery; there is no retry extractor.
pub(super) struct WindowsRunnerBorrowOnlyNotTransitionedCustody<'root> {
    _admitted: DurableWorkAdmittedPluginSlot<'root>,
    _prerequisite: WindowsRunnerLoadSetBorrowPrerequisite,
}

impl<'root> WindowsRunnerBorrowOnlyNotTransitionedCustody<'root> {
    pub(super) fn reject_validation(
        admitted: DurableWorkAdmittedPluginSlot<'root>,
        prerequisite: WindowsRunnerLoadSetBorrowPrerequisite,
    ) -> Self {
        Self {
            _admitted: admitted,
            _prerequisite: prerequisite,
        }
    }
}

/// A dispatched namespace query always consumes the exact attempt with both input owners. Even a
/// definitive negative is non-retryable through this custody because grant/session state may have
/// advanced and the same request nonce must never be replayed.
pub(super) struct WindowsRunnerNamespaceQueryUnusableCustody<'root> {
    class: WindowsRunnerNamespaceQueryFailureClass,
    _admitted: DurableWorkAdmittedPluginSlot<'root>,
    _prerequisite: UnqueriedWindowsRunnerLoadSetPrerequisite,
    _query_attempt: ManagedLoaderNamespaceQueryAttemptCustody,
    _returned_positive: Option<ManagedLoaderNamespaceQueryReceipt>,
    _authenticated_negative: Option<ManagedLoaderAuthenticatedNegativeReceipt>,
}

impl<'root> WindowsRunnerNamespaceQueryUnusableCustody<'root> {
    pub(super) fn reject_query(
        admitted: DurableWorkAdmittedPluginSlot<'root>,
        prerequisite: UnqueriedWindowsRunnerLoadSetPrerequisite,
        query_attempt: ManagedLoaderNamespaceQueryAttemptCustody,
        returned_positive: Option<ManagedLoaderNamespaceQueryReceipt>,
        authenticated_negative: Option<ManagedLoaderAuthenticatedNegativeReceipt>,
    ) -> Self {
        let (_, _, _, request_digest, query_nonce_digest, _, _) = query_attempt.binding();
        let class = if returned_positive.is_none()
            && query_attempt.matches_session(&prerequisite.namespace.session)
            && authenticated_negative.as_ref().is_some_and(|negative| {
                negative.matches_query(
                    &prerequisite.namespace.session,
                    request_digest,
                    query_nonce_digest,
                )
            }) {
            WindowsRunnerNamespaceQueryFailureClass::DefinitiveRejected
        } else {
            WindowsRunnerNamespaceQueryFailureClass::OutcomeUncertain
        };
        Self {
            class,
            _admitted: admitted,
            _prerequisite: prerequisite,
            _query_attempt: query_attempt,
            _returned_positive: returned_positive,
            _authenticated_negative: authenticated_negative,
        }
    }
}

/// Once the first close barrier is crossed, even a still-live prerequisite cannot authorize a new
/// admission attempt. It remains physically owned only for recovery and cleanup.
pub(super) struct ConsumedWindowsRunnerLoadSetPrerequisiteCustody {
    _prerequisite: PostLeaseSplitWindowsRunnerLoadSetPrerequisite,
}

impl ConsumedWindowsRunnerLoadSetPrerequisiteCustody {
    pub(super) fn consume(prerequisite: PostLeaseSplitWindowsRunnerLoadSetPrerequisite) -> Self {
        Self {
            _prerequisite: prerequisite,
        }
    }
}

pub(super) struct PendingWindowsRunnerPackageFileCustody {
    pub(super) package_file_ordinal: usize,
    pub(super) relative_path: String,
    pub(super) file: PinnedManagedFile,
    pub(super) anchor: ManagedLoaderFileIdentityAnchor,
}

pub(super) struct TransitionedWindowsRunnerPackageFileCustody {
    pub(super) package_file_ordinal: usize,
    pub(super) relative_path: String,
    pub(super) file: PinnedManagedLoaderFile,
}

pub(super) struct QuarantinedWindowsRunnerPackageFileReplacementCustody {
    pub(super) package_file_ordinal: usize,
    pub(super) relative_path: String,
    pub(super) replacement: QuarantinedManagedLoaderFile,
}

pub(super) struct WindowsRunnerPackageFileCloseOutcomeUncertainCustody {
    pub(super) package_file_ordinal: usize,
    pub(super) relative_path: String,
    pub(super) source: QuarantinedManagedLoaderSourceClose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowsRunnerParentRelativeReopenFailureClass {
    DefinitiveRejected,
    OutcomeUncertain,
}

/// Exact anchor-only custody for failure after the irreversible close and before a replacement
/// handle can be promoted. It is distinct from pending source and rejected-replacement custody.
pub(super) struct WindowsRunnerPackageFileReopenFailureCustody {
    pub(super) package_file_ordinal: usize,
    pub(super) relative_path: String,
    class: WindowsRunnerParentRelativeReopenFailureClass,
    _attempt: ManagedLoaderParentRelativeReopenAttemptCustody,
    _authenticated_negative: Option<ManagedLoaderParentRelativeReopenAuthenticatedNegativeReceipt>,
}

impl WindowsRunnerPackageFileReopenFailureCustody {
    pub(super) fn classify(
        package_file_ordinal: usize,
        relative_path: String,
        attempt: ManagedLoaderParentRelativeReopenAttemptCustody,
        authenticated_negative: Option<
            ManagedLoaderParentRelativeReopenAuthenticatedNegativeReceipt,
        >,
    ) -> Self {
        let class = if attempt.returned_positive_is_none()
            && authenticated_negative
                .as_ref()
                .is_some_and(|negative| negative.matches_attempt(&attempt))
        {
            WindowsRunnerParentRelativeReopenFailureClass::DefinitiveRejected
        } else {
            WindowsRunnerParentRelativeReopenFailureClass::OutcomeUncertain
        };
        Self {
            package_file_ordinal,
            relative_path,
            class,
            _attempt: attempt,
            _authenticated_negative: authenticated_negative,
        }
    }
}

pub(super) struct ValidatedRetainedWindowsRunnerNamespaceDirectoryCustody {
    pub(super) directory_ordinal: usize,
    pub(super) relative_path: String,
    pub(super) directory: PinnedManagedLoaderDirectory,
}

pub(super) struct WindowsRunnerFinalFenceQueryFailureCustody {
    class: WindowsRunnerNamespaceQueryFailureClass,
    _query_attempt: ManagedLoaderNamespaceQueryAttemptCustody,
    _returned_positive: Option<ManagedLoaderNamespaceQueryReceipt>,
    _authenticated_negative: Option<ManagedLoaderAuthenticatedNegativeReceipt>,
}

impl WindowsRunnerFinalFenceQueryFailureCustody {
    pub(super) fn classify(
        session: &ManagedLoaderNamespaceSession,
        query_attempt: ManagedLoaderNamespaceQueryAttemptCustody,
        returned_positive: Option<ManagedLoaderNamespaceQueryReceipt>,
        authenticated_negative: Option<ManagedLoaderAuthenticatedNegativeReceipt>,
    ) -> Self {
        let (_, _, _, request_digest, query_nonce_digest, _, _) = query_attempt.binding();
        let class = if returned_positive.is_none()
            && query_attempt.matches_session(session)
            && authenticated_negative.as_ref().is_some_and(|negative| {
                negative.matches_query(session, request_digest, query_nonce_digest)
            }) {
            WindowsRunnerNamespaceQueryFailureClass::DefinitiveRejected
        } else {
            WindowsRunnerNamespaceQueryFailureClass::OutcomeUncertain
        };
        Self {
            class,
            _query_attempt: query_attempt,
            _returned_positive: returned_positive,
            _authenticated_negative: authenticated_negative,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowsRunnerPostBarrierTransitionPhase {
    SourceHandleClose,
    ParentRelativeReopen,
    ReplacementIdentity,
    ReplacementHash,
    HandleDerivedPath,
    FinalFenceQuery,
}

/// Every post-barrier object remains indexed to its extraction-plan path and ordinal. This is
/// quarantine/recovery custody, never an admission or scalar retry permit.
#[must_use = "outcome-uncertain loader custody must be explicitly recovered or cleaned up"]
pub(super) struct WindowsRunnerLoadSetOutcomeUncertainCustody<'root> {
    pub(super) authority: LoaderTransitionAuthorityCustody<'root>,
    pub(super) prerequisite: ConsumedWindowsRunnerLoadSetPrerequisiteCustody,
    pub(super) package_root_directory: PinnedManagedLoaderDirectory,
    pub(super) pending_files: Vec<PendingWindowsRunnerPackageFileCustody>,
    pub(super) transitioned_files: Vec<TransitionedWindowsRunnerPackageFileCustody>,
    pub(super) close_outcome_uncertain:
        Option<WindowsRunnerPackageFileCloseOutcomeUncertainCustody>,
    pub(super) parent_relative_reopen_failure: Option<WindowsRunnerPackageFileReopenFailureCustody>,
    pub(super) quarantined_replacement:
        Option<QuarantinedWindowsRunnerPackageFileReplacementCustody>,
    pub(super) namespace_directories: Vec<ValidatedRetainedWindowsRunnerNamespaceDirectoryCustody>,
    pub(super) final_fence_query_failure: Option<WindowsRunnerFinalFenceQueryFailureCustody>,
    pub(super) transition_schedule: Vec<usize>,
    pub(super) next_transition_schedule_index: usize,
    pub(super) runner_ordinal: usize,
}

pub(super) enum WindowsRunnerLoadSetTransitionFailure<'root> {
    NameGrantAcquisitionUnusable {
        error: Error,
        custody: WindowsRunnerNameGrantAcquisitionUnusableCustody<'root>,
    },
    ContentLeaseAcquisitionUnusable {
        error: Error,
        custody: WindowsRunnerContentLeaseAcquisitionUnusableCustody<'root>,
    },
    BorrowOnlyNotTransitioned {
        error: Error,
        custody: WindowsRunnerBorrowOnlyNotTransitionedCustody<'root>,
    },
    NamespaceQueryUnusable {
        error: Error,
        custody: WindowsRunnerNamespaceQueryUnusableCustody<'root>,
    },
    PostBarrierOutcomeUncertain {
        phase: WindowsRunnerPostBarrierTransitionPhase,
        error: Error,
        custody: WindowsRunnerLoadSetOutcomeUncertainCustody<'root>,
    },
}

impl fmt::Debug for WindowsRunnerLoadSetOutcomeUncertainCustody<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowsRunnerLoadSetOutcomeUncertainCustody")
            .field("authority", &self.authority)
            .field("pending_file_count", &self.pending_files.len())
            .field("transitioned_file_count", &self.transitioned_files.len())
            .field(
                "close_outcome_uncertain",
                &self.close_outcome_uncertain.is_some(),
            )
            .field(
                "parent_relative_reopen_failure",
                &self.parent_relative_reopen_failure.is_some(),
            )
            .field(
                "quarantined_replacement",
                &self.quarantined_replacement.is_some(),
            )
            .field(
                "next_transition_schedule_index",
                &self.next_transition_schedule_index,
            )
            .field(
                "final_fence_query_outcome",
                &self
                    .final_fence_query_failure
                    .as_ref()
                    .map(|failure| failure.class),
            )
            .finish()
    }
}

impl fmt::Debug for WindowsRunnerLoadSetTransitionFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NameGrantAcquisitionUnusable { error, custody } => formatter
                .debug_struct("WindowsRunnerLoadSetTransitionFailure")
                .field("classification", &"name_grant_acquisition_unusable")
                .field("outcome", &custody.class)
                .field("error", error)
                .field("custody", &"<partial-name-grant-acquisition-quarantined>")
                .finish(),
            Self::ContentLeaseAcquisitionUnusable { error, custody } => formatter
                .debug_struct("WindowsRunnerLoadSetTransitionFailure")
                .field("classification", &"content_lease_acquisition_unusable")
                .field("outcome", &custody.class)
                .field("error", error)
                .field("custody", &"<partial-fileid-lease-acquisition-quarantined>")
                .finish(),
            Self::BorrowOnlyNotTransitioned { error, .. } => formatter
                .debug_struct("WindowsRunnerLoadSetTransitionFailure")
                .field("classification", &"borrow_only_not_transitioned")
                .field("error", error)
                .field("custody", &"<exact-input-owners-no-retry-extractor>")
                .finish(),
            Self::NamespaceQueryUnusable { error, custody } => formatter
                .debug_struct("WindowsRunnerLoadSetTransitionFailure")
                .field("classification", &"namespace_query_unusable")
                .field("outcome", &custody.class)
                .field("error", error)
                .field("custody", &"<inputs-and-exact-query-attempt-quarantined>")
                .finish(),
            Self::PostBarrierOutcomeUncertain {
                phase,
                error,
                custody,
            } => formatter
                .debug_struct("WindowsRunnerLoadSetTransitionFailure")
                .field("classification", &"post_barrier_outcome_uncertain")
                .field("phase", phase)
                .field("error", error)
                .field("custody", custody)
                .finish(),
        }
    }
}
