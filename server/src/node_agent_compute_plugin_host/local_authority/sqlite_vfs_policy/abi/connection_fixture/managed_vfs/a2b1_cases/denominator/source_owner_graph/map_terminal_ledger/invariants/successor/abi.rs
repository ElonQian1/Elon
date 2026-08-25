use std::collections::BTreeSet;

use super::super::super::{
    model::MapSourceStepId,
    reviewed_trace::{
        AbiNullWriteOutcome, AbiOutputSlotShape, AbiScalarInvalidityShape, ReviewedTerminal,
        ReviewedTraceEndpoint, ABI_INPUT_CELLS, ABI_OUTPUT_SLOT_SHAPES,
        ABI_SCALAR_INVALIDITY_SHAPES,
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

    for cell in ABI_INPUT_CELLS {
        let expected = match (cell.scalar_invalidity, cell.output_slot) {
            (
                AbiScalarInvalidityShape::Valid,
                AbiOutputSlotShape::ValidCallbackOwnedNonAliasing,
            ) => (
                AbiNullWriteOutcome::NullWritten,
                MapSourceStepId::AbiRawDispatch,
                ReviewedTraceEndpoint::Step(MapSourceStepId::AbiRawDispatch),
            ),
            (AbiScalarInvalidityShape::Valid, AbiOutputSlotShape::AbsentNull) => (
                AbiNullWriteOutcome::NoSlotNoWrite,
                MapSourceStepId::AbiNullOutputRejected,
                ReviewedTraceEndpoint::Terminal(ReviewedTerminal::AbiUnavailableNoSlot),
            ),
            (_, AbiOutputSlotShape::ValidCallbackOwnedNonAliasing) => (
                AbiNullWriteOutcome::NullWritten,
                MapSourceStepId::AbiInputRejected,
                ReviewedTraceEndpoint::Terminal(ReviewedTerminal::AbiUnavailableNull),
            ),
            (_, AbiOutputSlotShape::AbsentNull) => (
                AbiNullWriteOutcome::NoSlotNoWrite,
                MapSourceStepId::AbiInputRejected,
                ReviewedTraceEndpoint::Terminal(ReviewedTerminal::AbiUnavailableNoSlot),
            ),
        };
        if (cell.null_write, cell.decision_step, cell.endpoint) != expected {
            return Err("Map ABI input cell has a non-exact successor or null-write outcome");
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
    Ok(())
}

fn endpoint_count(expected: ReviewedTraceEndpoint) -> usize {
    ABI_INPUT_CELLS
        .iter()
        .filter(|cell| cell.endpoint == expected)
        .count()
}
