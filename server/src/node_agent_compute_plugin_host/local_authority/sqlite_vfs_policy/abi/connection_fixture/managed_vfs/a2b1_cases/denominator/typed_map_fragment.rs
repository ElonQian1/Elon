//! Map projection at the typed-operation outer-result seam.
//!
//! This fragment starts only after the raw-state gate admitted the expected installed type. It
//! freezes the four outcomes visible to `io_shm::map` and `run_code`: `NotPresent`, `Mapped`,
//! managed/adapter failure, and caught unwind. Caught unwind has two canonical abandonment
//! results, so the table contains five cells.
//!
//! The fragment deliberately does not project the route, callback, managed-map source path,
//! prestate, prefix mutation, terminal source universe, `SourceBranch`, `Expected`, `CaseKey`, or
//! denominator. Those axes remain behind the typed-operation provenance frontier.

use super::{
    abi_map_fragment::AbiNullWriteOutcome,
    case_key::Path,
    projection::ExpectedStatus,
    raw_state_fragment::{
        RawAbandonOutcome, RawCleanupEffect, RawPostOperationOutcome, RawSlotRetention,
        DROP_UNWIND_CUSTODY_PENDING, INSTALLED_RAW_VALUES, NO_RAW_VALUES,
    },
};

/// The four result classes observable outside the typed Map operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedTypedMapOutcomeFragment {
    NotPresent,
    Mapped,
    Failure,
    CaughtUnwind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedTypedMapDispositionFragment {
    NormalReturn,
    FallbackAfterCaughtUnwind(RawAbandonOutcome),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedTypedMapOutputFragment {
    NullRetained,
    MappedPointerWritten,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedTypedMapExitFragment {
    SqliteOkNotPresent,
    SqliteOkMapped,
    ShmMapUnavailable,
}

/// No cell in this local table decides which route, callback, managed branch or prestate produced
/// the outer result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedTypedMapProvenanceFragment {
    PendingRouteManagedPrestateAndCustody,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedTypedMapBranchFragment {
    pub(super) candidate_path: Path,
    pub(super) outcome: ReviewedTypedMapOutcomeFragment,
    pub(super) raw_post_operation: RawPostOperationOutcome,
    pub(super) disposition: ReviewedTypedMapDispositionFragment,
}

/// Only the raw slots, ABI output and return code at this outer cut are exact. This remains a
/// fragment rather than the final denominator `Expected` vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedTypedMapExpectedFragment {
    pub(super) null_write_at_entry: AbiNullWriteOutcome,
    pub(super) output_at_cut: ReviewedTypedMapOutputFragment,
    pub(super) raw_slots_at_entry: RawSlotRetention,
    pub(super) raw_slots_at_cut: RawSlotRetention,
    pub(super) cleanup: RawCleanupEffect,
    pub(super) sqlite_exit: ReviewedTypedMapExitFragment,
    pub(super) provenance: ReviewedTypedMapProvenanceFragment,
    pub(super) expected_status: ExpectedStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedTypedMapFragmentCell {
    pub(super) branch: ReviewedTypedMapBranchFragment,
    pub(super) expected: ReviewedTypedMapExpectedFragment,
}

const fn normal_cell(
    outcome: ReviewedTypedMapOutcomeFragment,
    output_at_cut: ReviewedTypedMapOutputFragment,
    sqlite_exit: ReviewedTypedMapExitFragment,
) -> ReviewedTypedMapFragmentCell {
    ReviewedTypedMapFragmentCell {
        branch: ReviewedTypedMapBranchFragment {
            candidate_path: Path::Map,
            outcome,
            raw_post_operation: RawPostOperationOutcome::AcceptedNormalReturn,
            disposition: ReviewedTypedMapDispositionFragment::NormalReturn,
        },
        expected: ReviewedTypedMapExpectedFragment {
            null_write_at_entry: AbiNullWriteOutcome::NullWritten,
            output_at_cut,
            raw_slots_at_entry: INSTALLED_RAW_VALUES,
            raw_slots_at_cut: INSTALLED_RAW_VALUES,
            cleanup: RawCleanupEffect::None,
            sqlite_exit,
            provenance: ReviewedTypedMapProvenanceFragment::PendingRouteManagedPrestateAndCustody,
            expected_status: ExpectedStatus::PendingSourceAndRedTeamReview,
        },
    }
}

const fn unwind_cell(
    abandonment: RawAbandonOutcome,
    raw_slots_at_cut: RawSlotRetention,
) -> ReviewedTypedMapFragmentCell {
    ReviewedTypedMapFragmentCell {
        branch: ReviewedTypedMapBranchFragment {
            candidate_path: Path::Map,
            outcome: ReviewedTypedMapOutcomeFragment::CaughtUnwind,
            raw_post_operation: RawPostOperationOutcome::CaughtUnwind,
            disposition: ReviewedTypedMapDispositionFragment::FallbackAfterCaughtUnwind(
                abandonment,
            ),
        },
        expected: ReviewedTypedMapExpectedFragment {
            null_write_at_entry: AbiNullWriteOutcome::NullWritten,
            output_at_cut: ReviewedTypedMapOutputFragment::NullRetained,
            raw_slots_at_entry: INSTALLED_RAW_VALUES,
            raw_slots_at_cut,
            cleanup: RawCleanupEffect::ClearSlotsThenDropInstalledEnvelope,
            sqlite_exit: ReviewedTypedMapExitFragment::ShmMapUnavailable,
            provenance: ReviewedTypedMapProvenanceFragment::PendingRouteManagedPrestateAndCustody,
            expected_status: ExpectedStatus::PendingSourceAndRedTeamReview,
        },
    }
}

/// Four outer result classes. Caught unwind expands to the two canonical installed-envelope Drop
/// outcomes, yielding five local cells without claiming a complete typed-operation source path.
pub(super) const REVIEWED_TYPED_MAP_FRAGMENTS: &[ReviewedTypedMapFragmentCell] = &[
    normal_cell(
        ReviewedTypedMapOutcomeFragment::NotPresent,
        ReviewedTypedMapOutputFragment::NullRetained,
        ReviewedTypedMapExitFragment::SqliteOkNotPresent,
    ),
    normal_cell(
        ReviewedTypedMapOutcomeFragment::Mapped,
        ReviewedTypedMapOutputFragment::MappedPointerWritten,
        ReviewedTypedMapExitFragment::SqliteOkMapped,
    ),
    normal_cell(
        ReviewedTypedMapOutcomeFragment::Failure,
        ReviewedTypedMapOutputFragment::NullRetained,
        ReviewedTypedMapExitFragment::ShmMapUnavailable,
    ),
    unwind_cell(RawAbandonOutcome::InstalledDropCompleted, NO_RAW_VALUES),
    unwind_cell(
        RawAbandonOutcome::InstalledDropUnwindCaught,
        DROP_UNWIND_CUSTODY_PENDING,
    ),
];
