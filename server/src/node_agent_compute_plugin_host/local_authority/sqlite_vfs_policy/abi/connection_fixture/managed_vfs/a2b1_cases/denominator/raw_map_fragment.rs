//! Map projection of the source-neutral raw-state fragment.
//!
//! This local quotient starts only after the ABI fragment's valid-scalar, writable-output
//! continuation has already written null. It records eight rejection/abandon continuations and
//! one expected-type continuation. None is a terminal denominator case: fallback custody/route
//! projection and the typed Map operation remain open frontiers.
//!
//! The source-neutral raw fragment owns pointer and serialization premises. This projection does
//! not admit dangling, forged, unaligned, uninitialized or concurrently accessed file storage,
//! and it does not turn aborting panic or undefined behavior into finite cells.

use super::{
    abi_map_fragment::AbiNullWriteOutcome,
    case_key::{InitializationPath, Path, PrefixMutation},
    projection::ExpectedStatus,
    raw_state_fragment::{
        RawAbandonOutcome, RawAdmissionShape, RawCleanupEffect, RawSlotRetention,
        DROP_UNWIND_CUSTODY_PENDING, FOREIGN_METHODS_AND_OPAQUE_STATE, INSTALLED_RAW_VALUES,
        METHODS_VALUE_ONLY, NO_RAW_VALUES, OPAQUE_STATE_VALUE,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapRawDecisionFragment {
    AdmissionRejected,
    ExpectedTypeInstalled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapRawDispositionFragment {
    ContinuesAfterAbandon(RawAbandonOutcome),
    ContinuesAtTypedMapOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapRawCutFragment {
    AfterAbandon,
    TypedMapOperationEntry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedMapRawBranchFragment {
    pub(super) admission: RawAdmissionShape,
    pub(super) candidate_path: Path,
    pub(super) decision: ReviewedMapRawDecisionFragment,
    pub(super) disposition: ReviewedMapRawDispositionFragment,
    pub(super) cut: ReviewedMapRawCutFragment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapRawExitFragment {
    KnownUnavailableNullAfterRawFallback,
    PendingAfterTypedMapOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapRawTypedOperationFragment {
    NotReachedByRawRejection,
    PendingAfterTypedMapOperation,
}

/// Only facts fixed at this local cut are exact. This is deliberately not the final `Expected`
/// vector, and `expected_status` remains pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedMapRawExpectedFragment {
    pub(super) null_write: AbiNullWriteOutcome,
    pub(super) raw_slots_at_gate: RawSlotRetention,
    pub(super) raw_slots_at_cut: RawSlotRetention,
    pub(super) cleanup: RawCleanupEffect,
    pub(super) sqlite_exit: ReviewedMapRawExitFragment,
    pub(super) typed_operation: ReviewedMapRawTypedOperationFragment,
    pub(super) prefix_mutation_at_cut: PrefixMutation,
    pub(super) initialization_at_cut: InitializationPath,
    pub(super) expected_status: ExpectedStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedMapRawFragmentCell {
    pub(super) branch: ReviewedMapRawBranchFragment,
    pub(super) expected: ReviewedMapRawExpectedFragment,
}

const fn fallback_cell(
    admission: RawAdmissionShape,
    outcome: RawAbandonOutcome,
    raw_slots_at_gate: RawSlotRetention,
    raw_slots_at_cut: RawSlotRetention,
    cleanup: RawCleanupEffect,
) -> ReviewedMapRawFragmentCell {
    ReviewedMapRawFragmentCell {
        branch: ReviewedMapRawBranchFragment {
            admission,
            candidate_path: Path::Map,
            decision: ReviewedMapRawDecisionFragment::AdmissionRejected,
            disposition: ReviewedMapRawDispositionFragment::ContinuesAfterAbandon(outcome),
            cut: ReviewedMapRawCutFragment::AfterAbandon,
        },
        expected: ReviewedMapRawExpectedFragment {
            null_write: AbiNullWriteOutcome::NullWritten,
            raw_slots_at_gate,
            raw_slots_at_cut,
            cleanup,
            sqlite_exit: ReviewedMapRawExitFragment::KnownUnavailableNullAfterRawFallback,
            typed_operation: ReviewedMapRawTypedOperationFragment::NotReachedByRawRejection,
            prefix_mutation_at_cut: PrefixMutation::NotReached,
            initialization_at_cut: InitializationPath::NotReached,
            expected_status: ExpectedStatus::PendingSourceAndRedTeamReview,
        },
    }
}

const fn typed_operation_cell() -> ReviewedMapRawFragmentCell {
    ReviewedMapRawFragmentCell {
        branch: ReviewedMapRawBranchFragment {
            admission: RawAdmissionShape::ExactMethodsInstalledExpectedType,
            candidate_path: Path::Map,
            decision: ReviewedMapRawDecisionFragment::ExpectedTypeInstalled,
            disposition: ReviewedMapRawDispositionFragment::ContinuesAtTypedMapOperation,
            cut: ReviewedMapRawCutFragment::TypedMapOperationEntry,
        },
        expected: ReviewedMapRawExpectedFragment {
            null_write: AbiNullWriteOutcome::NullWritten,
            raw_slots_at_gate: INSTALLED_RAW_VALUES,
            raw_slots_at_cut: INSTALLED_RAW_VALUES,
            cleanup: RawCleanupEffect::None,
            sqlite_exit: ReviewedMapRawExitFragment::PendingAfterTypedMapOperation,
            typed_operation: ReviewedMapRawTypedOperationFragment::PendingAfterTypedMapOperation,
            prefix_mutation_at_cut: PrefixMutation::NotReached,
            initialization_at_cut: InitializationPath::NotReached,
            expected_status: ExpectedStatus::PendingSourceAndRedTeamReview,
        },
    }
}

/// Nine local continuations from the sole valid-scalar, writable-output ABI cell: eight fallback
/// continuations after raw abandonment and one continuation into the typed-operation frontier.
pub(super) const REVIEWED_MAP_RAW_FRAGMENTS: &[ReviewedMapRawFragmentCell] = &[
    fallback_cell(
        RawAdmissionShape::NullFile,
        RawAbandonOutcome::NullFileRejected,
        NO_RAW_VALUES,
        NO_RAW_VALUES,
        RawCleanupEffect::None,
    ),
    fallback_cell(
        RawAdmissionShape::MethodsNullStateNull,
        RawAbandonOutcome::Empty,
        NO_RAW_VALUES,
        NO_RAW_VALUES,
        RawCleanupEffect::None,
    ),
    fallback_cell(
        RawAdmissionShape::MethodsNullStatePresent,
        RawAbandonOutcome::ForeignMethodsNullTableRejected,
        OPAQUE_STATE_VALUE,
        OPAQUE_STATE_VALUE,
        RawCleanupEffect::None,
    ),
    fallback_cell(
        RawAdmissionShape::ForeignMethodsStateNull,
        RawAbandonOutcome::ForeignMethodsForeignTableStateNullRejected,
        METHODS_VALUE_ONLY,
        METHODS_VALUE_ONLY,
        RawCleanupEffect::None,
    ),
    fallback_cell(
        RawAdmissionShape::ForeignMethodsStatePresent,
        RawAbandonOutcome::ForeignMethodsForeignTableStatePresentRejected,
        FOREIGN_METHODS_AND_OPAQUE_STATE,
        FOREIGN_METHODS_AND_OPAQUE_STATE,
        RawCleanupEffect::None,
    ),
    fallback_cell(
        RawAdmissionShape::ExactMethodsStateNull,
        RawAbandonOutcome::StateMissingRejected,
        METHODS_VALUE_ONLY,
        METHODS_VALUE_ONLY,
        RawCleanupEffect::None,
    ),
    fallback_cell(
        RawAdmissionShape::ExactMethodsInstalledWrongType,
        RawAbandonOutcome::InstalledDropCompleted,
        INSTALLED_RAW_VALUES,
        NO_RAW_VALUES,
        RawCleanupEffect::ClearSlotsThenDropInstalledEnvelope,
    ),
    fallback_cell(
        RawAdmissionShape::ExactMethodsInstalledWrongType,
        RawAbandonOutcome::InstalledDropUnwindCaught,
        INSTALLED_RAW_VALUES,
        DROP_UNWIND_CUSTODY_PENDING,
        RawCleanupEffect::ClearSlotsThenDropInstalledEnvelope,
    ),
    typed_operation_cell(),
];
