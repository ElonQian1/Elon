//! Canonical ABI-validation fragments for the future Map denominator.
//!
//! "Canonical" is limited to the reviewed `8 x 2` ABI input quotient. These fragments stop at
//! the raw-state gate and deliberately do not construct a complete `SourceBranch`, `Expected`,
//! `CaseKey`, static denominator, or dynamic inventory.

use super::{
    case_key::{BranchGroup, InitializationPath, Path, PrefixMutation},
    projection::ExpectedStatus,
};

/// The three scalar ABI validations encoded as an exact three-bit invalidity partition.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum AbiScalarInvalidityShape {
    Valid = 0,
    Region = 1,
    RegionSize = 2,
    RegionAndRegionSize = 3,
    Extend = 4,
    RegionAndExtend = 5,
    RegionSizeAndExtend = 6,
    RegionAndRegionSizeAndExtend = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum AbiOutputSlotShape {
    /// A live, aligned, writable, callback-owned slot that cannot alias raw file state.
    ValidCallbackOwnedNonAliasing,
    AbsentNull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum AbiNullWriteOutcome {
    NullWritten,
    NoSlotNoWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedMapAbiInputFragment {
    pub(super) scalar_invalidity: AbiScalarInvalidityShape,
    pub(super) output_slot: AbiOutputSlotShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapAbiDecisionFragment {
    ScalarTupleRejected,
    NullOutputRejected,
    RawStateDispatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapAbiTerminalFragment {
    UnavailableNull,
    UnavailableNoSlot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapAbiDispositionFragment {
    PreRawTerminal(ReviewedMapAbiTerminalFragment),
    /// The ABI fragment ends at `AbiRawDispatch`; the raw gate decides all later paths.
    ContinuesAtRawStateGate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedMapAbiBranchFragment {
    pub(super) input: ReviewedMapAbiInputFragment,
    pub(super) candidate_path: Path,
    pub(super) candidate_group: BranchGroup,
    pub(super) decision: ReviewedMapAbiDecisionFragment,
    pub(super) disposition: ReviewedMapAbiDispositionFragment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapAbiExitFragment {
    Exact(ReviewedMapAbiTerminalFragment),
    PendingAfterRawStateGate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewedMapAbiDownstreamFragment {
    NotReachedByPreRawTerminal,
    PendingAfterRawStateGate,
}

/// Only axes closed at the end of this ABI fragment are exact. `expected_status` intentionally
/// remains pending because this is not the final denominator `Expected` vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedMapAbiExpectedFragment {
    pub(super) input: ReviewedMapAbiInputFragment,
    pub(super) null_write: AbiNullWriteOutcome,
    pub(super) sqlite_exit: ReviewedMapAbiExitFragment,
    pub(super) typed_operation: ReviewedMapAbiDownstreamFragment,
    pub(super) prefix_mutation_at_cut: PrefixMutation,
    pub(super) initialization_at_cut: InitializationPath,
    pub(super) expected_status: ExpectedStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReviewedMapAbiFragmentCell {
    pub(super) branch: ReviewedMapAbiBranchFragment,
    pub(super) expected: ReviewedMapAbiExpectedFragment,
}

pub(super) const ABI_SCALAR_INVALIDITY_SHAPES: &[AbiScalarInvalidityShape] = &[
    AbiScalarInvalidityShape::Valid,
    AbiScalarInvalidityShape::Region,
    AbiScalarInvalidityShape::RegionSize,
    AbiScalarInvalidityShape::RegionAndRegionSize,
    AbiScalarInvalidityShape::Extend,
    AbiScalarInvalidityShape::RegionAndExtend,
    AbiScalarInvalidityShape::RegionSizeAndExtend,
    AbiScalarInvalidityShape::RegionAndRegionSizeAndExtend,
];

pub(super) const ABI_OUTPUT_SLOT_SHAPES: &[AbiOutputSlotShape] = &[
    AbiOutputSlotShape::ValidCallbackOwnedNonAliasing,
    AbiOutputSlotShape::AbsentNull,
];

const fn cell(
    scalar_invalidity: AbiScalarInvalidityShape,
    output_slot: AbiOutputSlotShape,
    null_write: AbiNullWriteOutcome,
    decision: ReviewedMapAbiDecisionFragment,
    disposition: ReviewedMapAbiDispositionFragment,
) -> ReviewedMapAbiFragmentCell {
    let input = ReviewedMapAbiInputFragment {
        scalar_invalidity,
        output_slot,
    };
    let (sqlite_exit, downstream) = match disposition {
        ReviewedMapAbiDispositionFragment::PreRawTerminal(terminal) => (
            ReviewedMapAbiExitFragment::Exact(terminal),
            ReviewedMapAbiDownstreamFragment::NotReachedByPreRawTerminal,
        ),
        ReviewedMapAbiDispositionFragment::ContinuesAtRawStateGate => (
            ReviewedMapAbiExitFragment::PendingAfterRawStateGate,
            ReviewedMapAbiDownstreamFragment::PendingAfterRawStateGate,
        ),
    };
    ReviewedMapAbiFragmentCell {
        branch: ReviewedMapAbiBranchFragment {
            input,
            candidate_path: Path::Map,
            candidate_group: BranchGroup::AbiValidation,
            decision,
            disposition,
        },
        expected: ReviewedMapAbiExpectedFragment {
            input,
            null_write,
            sqlite_exit,
            typed_operation: downstream,
            prefix_mutation_at_cut: PrefixMutation::NotReached,
            initialization_at_cut: InitializationPath::NotReached,
            expected_status: ExpectedStatus::PendingSourceAndRedTeamReview,
        },
    }
}

const fn valid_writable_cell() -> ReviewedMapAbiFragmentCell {
    cell(
        AbiScalarInvalidityShape::Valid,
        AbiOutputSlotShape::ValidCallbackOwnedNonAliasing,
        AbiNullWriteOutcome::NullWritten,
        ReviewedMapAbiDecisionFragment::RawStateDispatch,
        ReviewedMapAbiDispositionFragment::ContinuesAtRawStateGate,
    )
}

const fn valid_absent_cell() -> ReviewedMapAbiFragmentCell {
    cell(
        AbiScalarInvalidityShape::Valid,
        AbiOutputSlotShape::AbsentNull,
        AbiNullWriteOutcome::NoSlotNoWrite,
        ReviewedMapAbiDecisionFragment::NullOutputRejected,
        ReviewedMapAbiDispositionFragment::PreRawTerminal(
            ReviewedMapAbiTerminalFragment::UnavailableNoSlot,
        ),
    )
}

const fn invalid_writable_cell(shape: AbiScalarInvalidityShape) -> ReviewedMapAbiFragmentCell {
    cell(
        shape,
        AbiOutputSlotShape::ValidCallbackOwnedNonAliasing,
        AbiNullWriteOutcome::NullWritten,
        ReviewedMapAbiDecisionFragment::ScalarTupleRejected,
        ReviewedMapAbiDispositionFragment::PreRawTerminal(
            ReviewedMapAbiTerminalFragment::UnavailableNull,
        ),
    )
}

const fn invalid_absent_cell(shape: AbiScalarInvalidityShape) -> ReviewedMapAbiFragmentCell {
    cell(
        shape,
        AbiOutputSlotShape::AbsentNull,
        AbiNullWriteOutcome::NoSlotNoWrite,
        ReviewedMapAbiDecisionFragment::ScalarTupleRejected,
        ReviewedMapAbiDispositionFragment::PreRawTerminal(
            ReviewedMapAbiTerminalFragment::UnavailableNoSlot,
        ),
    )
}

/// Exact `8 x 2` local ABI quotient: 15 pre-raw terminal cells and one raw-dispatch continuation.
pub(super) const REVIEWED_MAP_ABI_FRAGMENTS: &[ReviewedMapAbiFragmentCell] = &[
    valid_writable_cell(),
    valid_absent_cell(),
    invalid_writable_cell(AbiScalarInvalidityShape::Region),
    invalid_absent_cell(AbiScalarInvalidityShape::Region),
    invalid_writable_cell(AbiScalarInvalidityShape::RegionSize),
    invalid_absent_cell(AbiScalarInvalidityShape::RegionSize),
    invalid_writable_cell(AbiScalarInvalidityShape::RegionAndRegionSize),
    invalid_absent_cell(AbiScalarInvalidityShape::RegionAndRegionSize),
    invalid_writable_cell(AbiScalarInvalidityShape::Extend),
    invalid_absent_cell(AbiScalarInvalidityShape::Extend),
    invalid_writable_cell(AbiScalarInvalidityShape::RegionAndExtend),
    invalid_absent_cell(AbiScalarInvalidityShape::RegionAndExtend),
    invalid_writable_cell(AbiScalarInvalidityShape::RegionSizeAndExtend),
    invalid_absent_cell(AbiScalarInvalidityShape::RegionSizeAndExtend),
    invalid_writable_cell(AbiScalarInvalidityShape::RegionAndRegionSizeAndExtend),
    invalid_absent_cell(AbiScalarInvalidityShape::RegionAndRegionSizeAndExtend),
];
