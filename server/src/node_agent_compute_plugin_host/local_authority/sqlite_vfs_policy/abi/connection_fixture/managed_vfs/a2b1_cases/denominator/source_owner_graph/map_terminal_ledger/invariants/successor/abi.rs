use std::collections::BTreeSet;

use super::super::super::super::super::{
    case_key::{BranchGroup, InitializationPath, Path, PrefixMutation},
    projection::ExpectedStatus,
};
use super::super::super::{
    model::MapSourceStepId,
    reviewed_trace::{
        AbiNullWriteOutcome, AbiOutputSlotShape, AbiScalarInvalidityShape,
        ReviewedMapAbiDecisionFragment, ReviewedMapAbiDispositionFragment,
        ReviewedMapAbiDownstreamFragment, ReviewedMapAbiExitFragment,
        ReviewedMapAbiTerminalFragment, ReviewedTerminal, ReviewedTraceEndpoint, ABI_INPUT_CELLS,
        ABI_OUTPUT_SLOT_SHAPES, ABI_SCALAR_INVALIDITY_SHAPES, REVIEWED_MAP_ABI_FRAGMENTS,
    },
};

pub(super) fn validate() -> Result<(), &'static str> {
    let expected_shapes = [
        AbiScalarInvalidityShape::Valid,
        AbiScalarInvalidityShape::Region,
        AbiScalarInvalidityShape::RegionSize,
        AbiScalarInvalidityShape::RegionAndRegionSize,
        AbiScalarInvalidityShape::Extend,
        AbiScalarInvalidityShape::RegionAndExtend,
        AbiScalarInvalidityShape::RegionSizeAndExtend,
        AbiScalarInvalidityShape::RegionAndRegionSizeAndExtend,
    ];
    let actual_shapes = ABI_SCALAR_INVALIDITY_SHAPES
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if actual_shapes != expected_shapes.into_iter().collect()
        || actual_shapes.len() != ABI_SCALAR_INVALIDITY_SHAPES.len()
        || !ABI_SCALAR_INVALIDITY_SHAPES
            .iter()
            .enumerate()
            .all(|(mask, shape)| usize::from(*shape as u8) == mask)
    {
        return Err("Map ABI scalar-invalidity mask set is not exact");
    }

    let expected_slots = [
        AbiOutputSlotShape::ValidCallbackOwnedNonAliasing,
        AbiOutputSlotShape::AbsentNull,
    ];
    let actual_slots = ABI_OUTPUT_SLOT_SHAPES
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if actual_slots != expected_slots.into_iter().collect()
        || actual_slots.len() != ABI_OUTPUT_SLOT_SHAPES.len()
    {
        return Err("Map ABI output-slot premise set is not exact");
    }

    let cells = ABI_INPUT_CELLS
        .iter()
        .map(|cell| (cell.scalar_invalidity, cell.output_slot))
        .collect::<BTreeSet<_>>();
    let expected_cells = expected_shapes
        .into_iter()
        .flat_map(|shape| expected_slots.into_iter().map(move |slot| (shape, slot)))
        .collect::<BTreeSet<_>>();
    if cells != expected_cells || cells.len() != ABI_INPUT_CELLS.len() || cells.len() != 16 {
        return Err("Map ABI input ledger is not the exact 8-by-2 partition");
    }
    let fragments = REVIEWED_MAP_ABI_FRAGMENTS
        .iter()
        .map(|fragment| {
            (
                fragment.branch.input.scalar_invalidity,
                fragment.branch.input.output_slot,
            )
        })
        .collect::<BTreeSet<_>>();
    if fragments != expected_cells
        || fragments.len() != REVIEWED_MAP_ABI_FRAGMENTS.len()
        || fragments.len() != 16
    {
        return Err("Map ABI denominator fragment is not the exact 8-by-2 partition");
    }

    for cell in ABI_INPUT_CELLS {
        let expected = match (cell.scalar_invalidity, cell.output_slot) {
            (
                AbiScalarInvalidityShape::Valid,
                AbiOutputSlotShape::ValidCallbackOwnedNonAliasing,
            ) => (
                AbiNullWriteOutcome::NullWritten,
                MapSourceStepId::AbiRawDispatch,
                ReviewedTraceEndpoint::Step(MapSourceStepId::AbiRawDispatch),
                ReviewedMapAbiDecisionFragment::RawStateDispatch,
                ReviewedMapAbiDispositionFragment::ContinuesAtRawStateGate,
                ReviewedMapAbiExitFragment::PendingAfterRawStateGate,
                ReviewedMapAbiDownstreamFragment::PendingAfterRawStateGate,
            ),
            (AbiScalarInvalidityShape::Valid, AbiOutputSlotShape::AbsentNull) => (
                AbiNullWriteOutcome::NoSlotNoWrite,
                MapSourceStepId::AbiNullOutputRejected,
                ReviewedTraceEndpoint::Terminal(ReviewedTerminal::AbiUnavailableNoSlot),
                ReviewedMapAbiDecisionFragment::NullOutputRejected,
                ReviewedMapAbiDispositionFragment::PreRawTerminal(
                    ReviewedMapAbiTerminalFragment::UnavailableNoSlot,
                ),
                ReviewedMapAbiExitFragment::Exact(
                    ReviewedMapAbiTerminalFragment::UnavailableNoSlot,
                ),
                ReviewedMapAbiDownstreamFragment::NotReachedByPreRawTerminal,
            ),
            (_, AbiOutputSlotShape::ValidCallbackOwnedNonAliasing) => (
                AbiNullWriteOutcome::NullWritten,
                MapSourceStepId::AbiInputRejected,
                ReviewedTraceEndpoint::Terminal(ReviewedTerminal::AbiUnavailableNull),
                ReviewedMapAbiDecisionFragment::ScalarTupleRejected,
                ReviewedMapAbiDispositionFragment::PreRawTerminal(
                    ReviewedMapAbiTerminalFragment::UnavailableNull,
                ),
                ReviewedMapAbiExitFragment::Exact(ReviewedMapAbiTerminalFragment::UnavailableNull),
                ReviewedMapAbiDownstreamFragment::NotReachedByPreRawTerminal,
            ),
            (_, AbiOutputSlotShape::AbsentNull) => (
                AbiNullWriteOutcome::NoSlotNoWrite,
                MapSourceStepId::AbiInputRejected,
                ReviewedTraceEndpoint::Terminal(ReviewedTerminal::AbiUnavailableNoSlot),
                ReviewedMapAbiDecisionFragment::ScalarTupleRejected,
                ReviewedMapAbiDispositionFragment::PreRawTerminal(
                    ReviewedMapAbiTerminalFragment::UnavailableNoSlot,
                ),
                ReviewedMapAbiExitFragment::Exact(
                    ReviewedMapAbiTerminalFragment::UnavailableNoSlot,
                ),
                ReviewedMapAbiDownstreamFragment::NotReachedByPreRawTerminal,
            ),
        };
        if (cell.null_write, cell.decision_step, cell.endpoint)
            != (expected.0, expected.1, expected.2)
        {
            return Err("Map ABI input cell has a non-exact successor or null-write outcome");
        }
        let Some(fragment) = REVIEWED_MAP_ABI_FRAGMENTS.iter().find(|fragment| {
            fragment.branch.input.scalar_invalidity == cell.scalar_invalidity
                && fragment.branch.input.output_slot == cell.output_slot
        }) else {
            return Err("Map ABI source cell has no denominator fragment");
        };
        if fragment.branch.input != fragment.expected.input
            || fragment.branch.candidate_path != Path::Map
            || fragment.branch.candidate_group != BranchGroup::AbiValidation
            || fragment.branch.decision != expected.3
            || fragment.branch.disposition != expected.4
            || fragment.expected.null_write != expected.0
            || fragment.expected.sqlite_exit != expected.5
            || fragment.expected.typed_operation != expected.6
            || fragment.expected.prefix_mutation_at_cut != PrefixMutation::NotReached
            || fragment.expected.initialization_at_cut != InitializationPath::NotReached
            || fragment.expected.expected_status != ExpectedStatus::PendingSourceAndRedTeamReview
        {
            return Err("Map ABI denominator fragment changed a closed or pending local axis");
        }
    }

    let unavailable_null = endpoint_count(ReviewedTraceEndpoint::Terminal(
        ReviewedTerminal::AbiUnavailableNull,
    ));
    let unavailable_no_slot = endpoint_count(ReviewedTraceEndpoint::Terminal(
        ReviewedTerminal::AbiUnavailableNoSlot,
    ));
    let raw_dispatch = endpoint_count(ReviewedTraceEndpoint::Step(MapSourceStepId::AbiRawDispatch));
    if (unavailable_null, unavailable_no_slot, raw_dispatch) != (7, 8, 1) {
        return Err("Map ABI 16-cell terminal and raw-dispatch counts are not exact");
    }
    let terminal_fragments = REVIEWED_MAP_ABI_FRAGMENTS
        .iter()
        .filter(|fragment| {
            matches!(
                fragment.branch.disposition,
                ReviewedMapAbiDispositionFragment::PreRawTerminal(_)
            )
        })
        .count();
    if (
        terminal_fragments,
        REVIEWED_MAP_ABI_FRAGMENTS.len() - terminal_fragments,
    ) != (15, 1)
    {
        return Err("Map ABI denominator fragment is not 15 terminals plus one raw continuation");
    }
    Ok(())
}

fn endpoint_count(expected: ReviewedTraceEndpoint) -> usize {
    ABI_INPUT_CELLS
        .iter()
        .filter(|cell| cell.endpoint == expected)
        .count()
}
