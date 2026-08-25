use super::super::super::model::SourceEffect;
use super::super::model::{MapExit, MapSourceStepId, MapSourceStepId::*};
use super::model::*;
use super::{AbiNullWriteOutcome, AbiOutputSlotShape, AbiScalarInvalidityShape};

const fn abi_cell(
    scalar_invalidity: AbiScalarInvalidityShape,
    output_slot: AbiOutputSlotShape,
    null_write: AbiNullWriteOutcome,
    decision_step: super::super::model::MapSourceStepId,
    endpoint: ReviewedTraceEndpoint,
) -> AbiInputCell {
    AbiInputCell {
        scalar_invalidity,
        output_slot,
        null_write,
        decision_step,
        endpoint,
    }
}

const fn valid_writable_cell() -> AbiInputCell {
    abi_cell(
        AbiScalarInvalidityShape::Valid,
        AbiOutputSlotShape::ValidCallbackOwnedNonAliasing,
        AbiNullWriteOutcome::NullWritten,
        AbiRawDispatch,
        ReviewedTraceEndpoint::Step(AbiRawDispatch),
    )
}

const fn valid_absent_cell() -> AbiInputCell {
    abi_cell(
        AbiScalarInvalidityShape::Valid,
        AbiOutputSlotShape::AbsentNull,
        AbiNullWriteOutcome::NoSlotNoWrite,
        AbiNullOutputRejected,
        ReviewedTraceEndpoint::Terminal(ReviewedTerminal::AbiUnavailableNoSlot),
    )
}

const fn invalid_writable_cell(shape: AbiScalarInvalidityShape) -> AbiInputCell {
    abi_cell(
        shape,
        AbiOutputSlotShape::ValidCallbackOwnedNonAliasing,
        AbiNullWriteOutcome::NullWritten,
        AbiInputRejected,
        ReviewedTraceEndpoint::Terminal(ReviewedTerminal::AbiUnavailableNull),
    )
}

const fn invalid_absent_cell(shape: AbiScalarInvalidityShape) -> AbiInputCell {
    abi_cell(
        shape,
        AbiOutputSlotShape::AbsentNull,
        AbiNullWriteOutcome::NoSlotNoWrite,
        AbiInputRejected,
        ReviewedTraceEndpoint::Terminal(ReviewedTerminal::AbiUnavailableNoSlot),
    )
}

/// The complete 8 scalar-invalidity masks crossed with the two reviewed output-slot shapes.
pub(in super::super) const ABI_INPUT_CELLS: &[AbiInputCell] = &[
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

const fn step_edge(
    from: MapSourceStepId,
    to: ReviewedTraceEndpoint,
    condition: ReviewedTraceCondition,
    relation: ReviewedTraceRelation,
    effect: SourceEffect,
    raw_slots: Option<RawSlotRetention>,
) -> ReviewedSuccessorEdge {
    ReviewedSuccessorEdge {
        from: ReviewedTraceEndpoint::Step(from),
        to,
        condition,
        relation,
        effect,
        raw_slots,
    }
}

const fn conditional(
    from: MapSourceStepId,
    to: MapSourceStepId,
    condition: ReviewedTraceCondition,
) -> ReviewedSuccessorEdge {
    step_edge(
        from,
        ReviewedTraceEndpoint::Step(to),
        condition,
        ReviewedTraceRelation::ConditionalBranch,
        SourceEffect::None,
        None,
    )
}

const fn conditional_with_effect(
    from: MapSourceStepId,
    to: MapSourceStepId,
    condition: ReviewedTraceCondition,
    effect: SourceEffect,
) -> ReviewedSuccessorEdge {
    step_edge(
        from,
        ReviewedTraceEndpoint::Step(to),
        condition,
        ReviewedTraceRelation::ConditionalBranch,
        effect,
        None,
    )
}

const fn raw_state_branch(
    from: MapSourceStepId,
    to: MapSourceStepId,
    case: RawStateCase,
    slots: RawSlotRetention,
) -> ReviewedSuccessorEdge {
    step_edge(
        from,
        ReviewedTraceEndpoint::Step(to),
        ReviewedTraceCondition::RawState(case),
        ReviewedTraceRelation::ConditionalBranch,
        SourceEffect::None,
        Some(slots),
    )
}

const fn abandon_case(
    from: MapSourceStepId,
    to: MapSourceStepId,
    case: RawStateCase,
    slots: RawSlotRetention,
) -> ReviewedSuccessorEdge {
    step_edge(
        from,
        ReviewedTraceEndpoint::Step(to),
        ReviewedTraceCondition::RawState(case),
        ReviewedTraceRelation::Abandon,
        SourceEffect::None,
        Some(slots),
    )
}

const fn raw_projection(
    from: MapSourceStepId,
    outcome: RawAbandonOutcome,
    relation: ReviewedTraceRelation,
    effect: SourceEffect,
    slots: RawSlotRetention,
) -> ReviewedSuccessorEdge {
    step_edge(
        from,
        ReviewedTraceEndpoint::Step(RawFallbackProjection),
        ReviewedTraceCondition::RawAbandon(outcome),
        relation,
        effect,
        Some(slots),
    )
}

const fn terminal_projection(
    from: MapSourceStepId,
    terminal: ReviewedTerminal,
    condition: ReviewedTraceCondition,
) -> ReviewedSuccessorEdge {
    step_edge(
        from,
        ReviewedTraceEndpoint::Terminal(terminal),
        condition,
        ReviewedTraceRelation::ResultProjection,
        SourceEffect::None,
        None,
    )
}

const fn open_frontier(
    from: MapSourceStepId,
    frontier: ReviewedOpenFrontier,
) -> ReviewedSuccessorEdge {
    step_edge(
        from,
        ReviewedTraceEndpoint::OpenFrontier(frontier),
        ReviewedTraceCondition::Unconditional,
        ReviewedTraceRelation::OpenFrontier,
        SourceEffect::None,
        None,
    )
}

macro_rules! raw_branch_edge {
    ($step:ident, $case:ident, $slots:ident) => {
        raw_state_branch(AbiRawDispatch, $step, RawStateCase::$case, $slots)
    };
}

macro_rules! abandon_edge {
    ($from:ident => $to:ident, $case:ident, $slots:ident) => {
        abandon_case($from, $to, RawStateCase::$case, $slots)
    };
}

macro_rules! projection_edge {
    ($from:ident, $outcome:ident, $relation:ident, $effect:ident, $slots:ident) => {
        raw_projection(
            $from,
            RawAbandonOutcome::$outcome,
            ReviewedTraceRelation::$relation,
            SourceEffect::$effect,
            $slots,
        )
    };
}

pub(in super::super) const SUCCESSOR_EDGES: &[ReviewedSuccessorEdge] = &[
    conditional_with_effect(
        AbiNullFirst,
        AbiInputRejected,
        ReviewedTraceCondition::AbiInvalidOutputWritable,
        SourceEffect::OutputNull,
    ),
    conditional(
        AbiNullFirst,
        AbiInputRejected,
        ReviewedTraceCondition::AbiInvalidOutputAbsent,
    ),
    step_edge(
        AbiNullFirst,
        ReviewedTraceEndpoint::Step(AbiRawDispatch),
        ReviewedTraceCondition::AbiValidOutputWritable,
        ReviewedTraceRelation::Continuation,
        SourceEffect::OutputNull,
        None,
    ),
    conditional(
        AbiNullFirst,
        AbiNullOutputRejected,
        ReviewedTraceCondition::AbiValidOutputAbsent,
    ),
    terminal_projection(
        AbiInputRejected,
        ReviewedTerminal::AbiUnavailableNull,
        ReviewedTraceCondition::AbiInvalidOutputWritable,
    ),
    terminal_projection(
        AbiInputRejected,
        ReviewedTerminal::AbiUnavailableNoSlot,
        ReviewedTraceCondition::AbiInvalidOutputAbsent,
    ),
    terminal_projection(
        AbiNullOutputRejected,
        ReviewedTerminal::AbiUnavailableNoSlot,
        ReviewedTraceCondition::AbiValidOutputAbsent,
    ),
    step_edge(
        AbiRawDispatch,
        ReviewedTraceEndpoint::OpenFrontier(ReviewedOpenFrontier::TypedMapOperation),
        ReviewedTraceCondition::RawExpectedTypeEntry,
        ReviewedTraceRelation::OpenFrontier,
        SourceEffect::None,
        Some(INSTALLED_RAW_VALUES),
    ),
    raw_branch_edge!(RawStateNullFile, NullFile, NO_RAW_VALUES),
    raw_branch_edge!(RawStateUninstalled, Uninstalled, NO_RAW_VALUES),
    raw_branch_edge!(
        RawStateForeignMethodsNullTable,
        ForeignMethodsNullTableStatePresent,
        OPAQUE_STATE_VALUE
    ),
    raw_branch_edge!(
        RawStateForeignMethodsForeignTable,
        ForeignMethodsForeignTableStateNull,
        METHODS_VALUE_ONLY
    ),
    raw_branch_edge!(
        RawStateForeignMethodsForeignTable,
        ForeignMethodsForeignTableStatePresent,
        FOREIGN_METHODS_AND_OPAQUE_STATE
    ),
    raw_branch_edge!(
        RawStateMissing,
        StateMissingInertTableStateNull,
        METHODS_VALUE_ONLY
    ),
    raw_branch_edge!(
        RawStateTypeMismatch,
        TypeMismatchInstalled,
        INSTALLED_RAW_VALUES
    ),
    abandon_edge!(RawStateNullFile => RawAbandonNullFileRejected, NullFile, NO_RAW_VALUES),
    abandon_edge!(RawStateUninstalled => RawAbandonEmpty, Uninstalled, NO_RAW_VALUES),
    abandon_edge!(RawStateForeignMethodsNullTable => RawAbandonForeignMethodsNullTableRejected, ForeignMethodsNullTableStatePresent, OPAQUE_STATE_VALUE),
    abandon_edge!(RawStateForeignMethodsForeignTable => RawAbandonForeignMethodsForeignTableRejected, ForeignMethodsForeignTableStateNull, METHODS_VALUE_ONLY),
    abandon_edge!(RawStateForeignMethodsForeignTable => RawAbandonForeignMethodsForeignTableRejected, ForeignMethodsForeignTableStatePresent, FOREIGN_METHODS_AND_OPAQUE_STATE),
    abandon_edge!(RawStateMissing => RawAbandonStateMissingRejected, StateMissingInertTableStateNull, METHODS_VALUE_ONLY),
    abandon_edge!(RawStateTypeMismatch => RawAbandonInstalled, TypeMismatchInstalled, INSTALLED_RAW_VALUES),
    projection_edge!(
        RawAbandonEmpty,
        Empty,
        ResultProjection,
        None,
        NO_RAW_VALUES
    ),
    projection_edge!(
        RawAbandonInstalled,
        InstalledDropCompleted,
        Cleanup,
        Cleanup,
        NO_RAW_VALUES
    ),
    projection_edge!(
        RawAbandonInstalled,
        InstalledDropUnwindCaught,
        Cleanup,
        Cleanup,
        DROP_UNWIND_CUSTODY_PENDING
    ),
    projection_edge!(
        RawAbandonNullFileRejected,
        NullFileRejected,
        ResultProjection,
        None,
        NO_RAW_VALUES
    ),
    projection_edge!(
        RawAbandonForeignMethodsNullTableRejected,
        ForeignMethodsNullTableRejected,
        ResultProjection,
        None,
        OPAQUE_STATE_VALUE
    ),
    projection_edge!(
        RawAbandonForeignMethodsForeignTableRejected,
        ForeignMethodsForeignTableStateNullRejected,
        ResultProjection,
        None,
        METHODS_VALUE_ONLY
    ),
    projection_edge!(
        RawAbandonForeignMethodsForeignTableRejected,
        ForeignMethodsForeignTableStatePresentRejected,
        ResultProjection,
        None,
        FOREIGN_METHODS_AND_OPAQUE_STATE
    ),
    projection_edge!(
        RawAbandonStateMissingRejected,
        StateMissingRejected,
        ResultProjection,
        None,
        METHODS_VALUE_ONLY
    ),
    open_frontier(
        RawFallbackProjection,
        ReviewedOpenFrontier::RawFallbackCustodyAndRouteProjection,
    ),
];

pub(in super::super) const TERMINALS: &[ReviewedTerminal] = &[
    ReviewedTerminal::AbiUnavailableNull,
    ReviewedTerminal::AbiUnavailableNoSlot,
];

pub(in super::super) const OPEN_FRONTIERS: &[ReviewedOpenFrontier] = &[
    ReviewedOpenFrontier::TypedMapOperation,
    ReviewedOpenFrontier::RawFallbackCustodyAndRouteProjection,
];

pub(in super::super) const OPEN_FRONTIER_RECORDS: &[ReviewedOpenFrontierRecord] = &[
    ReviewedOpenFrontierRecord {
        frontier: ReviewedOpenFrontier::TypedMapOperation,
        ingress: ReviewedFrontierIngress::ExpectedTypedState,
        known_exit: None,
        custody_unresolved: true,
        route_projection_unresolved: true,
    },
    ReviewedOpenFrontierRecord {
        frontier: ReviewedOpenFrontier::RawFallbackCustodyAndRouteProjection,
        ingress: ReviewedFrontierIngress::PrefixRawRejectionAfterAbandon,
        known_exit: Some(MapExit::AbiUnavailableNull),
        custody_unresolved: true,
        route_projection_unresolved: true,
    },
];
