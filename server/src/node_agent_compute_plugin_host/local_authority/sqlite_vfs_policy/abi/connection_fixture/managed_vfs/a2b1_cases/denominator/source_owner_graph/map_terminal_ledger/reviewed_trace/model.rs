use super::super::super::super::abi_map_fragment::{
    AbiNullWriteOutcome, AbiOutputSlotShape, AbiScalarInvalidityShape,
};
pub(in super::super) use super::super::super::super::raw_state_fragment::{
    RawAbandonOutcome, RawSlotRetention, DROP_UNWIND_CUSTODY_PENDING,
    FOREIGN_METHODS_AND_OPAQUE_STATE, INSTALLED_RAW_VALUES, METHODS_VALUE_ONLY, NO_RAW_VALUES,
    OPAQUE_STATE_VALUE,
};
use super::super::super::model::SourceEffect;
use super::super::model::{MapExit, MapSourceStepId, MapSourceStepId::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super) enum ReviewedTerminal {
    AbiUnavailableNull,
    AbiUnavailableNoSlot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super) enum ReviewedOpenFrontier {
    TypedMapOperation,
    RawFallbackCustodyAndRouteProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super) enum ReviewedFrontierIngress {
    ExpectedTypedState,
    /// Only raw rejections materialized before the typed-operation frontier.
    PrefixRawRejectionAfterAbandon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ReviewedOpenFrontierRecord {
    pub(in super::super) frontier: ReviewedOpenFrontier,
    pub(in super::super) ingress: ReviewedFrontierIngress,
    pub(in super::super) known_exit: Option<MapExit>,
    pub(in super::super) custody_unresolved: bool,
    pub(in super::super) route_projection_unresolved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super) enum ReviewedTraceEndpoint {
    Step(MapSourceStepId),
    Terminal(ReviewedTerminal),
    OpenFrontier(ReviewedOpenFrontier),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct AbiInputCell {
    pub(in super::super) scalar_invalidity: AbiScalarInvalidityShape,
    pub(in super::super) output_slot: AbiOutputSlotShape,
    pub(in super::super) null_write: AbiNullWriteOutcome,
    pub(in super::super) decision_step: MapSourceStepId,
    pub(in super::super) endpoint: ReviewedTraceEndpoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super) enum RawStateOutcome {
    AcceptedNormalReturn,
    NullFile,
    Uninstalled,
    ForeignMethodsNullTable,
    ForeignMethodsForeignTable,
    StateMissing,
    TypeMismatch,
    /// A Rust panic caught while unwinding; UB and aborting panics are outside this case.
    CaughtUnwind,
}

// `Occupied` is install-only and cannot be returned by `with_installed_state`; aborting panics,
// forged envelopes and other UB premises likewise do not enter this protected-call outcome set.

/// The nine reviewed source cases. The foreign methods-table rejection is intentionally crossed
/// with null/non-null state because those two prestates retain different raw slots. `Occupied`
/// belongs to installation, cannot be returned by this Map raw-state read gate, and is excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super) enum RawStateCase {
    AcceptedAfterTypedOperation,
    NullFile,
    Uninstalled,
    ForeignMethodsNullTableStatePresent,
    ForeignMethodsForeignTableStateNull,
    ForeignMethodsForeignTableStatePresent,
    StateMissingInertTableStateNull,
    TypeMismatchInstalled,
    CaughtUnwindFromTypedOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super) enum RawStateTraceDisposition {
    PrefixSuccessor(ReviewedTraceEndpoint),
    /// The source step is observable only after entering this unresolved operation.
    BeyondOpenFrontier(ReviewedOpenFrontier),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct RawStateOutcomeRecord {
    pub(in super::super) case: RawStateCase,
    pub(in super::super) outcome: RawStateOutcome,
    pub(in super::super) step: MapSourceStepId,
    pub(in super::super) slots: RawSlotRetention,
    pub(in super::super) trace: RawStateTraceDisposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super) enum RawAbandonCauseDisposition {
    PrefixSuccessor,
    BeyondOpenFrontier(ReviewedOpenFrontier),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super) struct RawAbandonCause {
    pub(in super::super) case: RawStateCase,
    pub(in super::super) disposition: RawAbandonCauseDisposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct RawAbandonOutcomeRecord {
    pub(in super::super) outcome: RawAbandonOutcome,
    /// Prefix causes materialize successor edges. Beyond-frontier causes preserve the exact
    /// source association without pretending that the unresolved operation has been traced.
    pub(in super::super) causes: &'static [RawAbandonCause],
    pub(in super::super) step: MapSourceStepId,
    pub(in super::super) effect: SourceEffect,
    pub(in super::super) slots: RawSlotRetention,
    pub(in super::super) prefix_successor: ReviewedTraceEndpoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super) enum ReviewedTraceCondition {
    Unconditional,
    AbiInvalidOutputWritable,
    AbiInvalidOutputAbsent,
    AbiValidOutputWritable,
    AbiValidOutputAbsent,
    RawExpectedTypeEntry,
    RawState(RawStateCase),
    RawAbandon(RawAbandonOutcome),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super) enum ReviewedTraceRelation {
    Continuation,
    ConditionalBranch,
    Abandon,
    Cleanup,
    ResultProjection,
    OpenFrontier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ReviewedSuccessorEdge {
    pub(in super::super) from: ReviewedTraceEndpoint,
    pub(in super::super) to: ReviewedTraceEndpoint,
    pub(in super::super) condition: ReviewedTraceCondition,
    pub(in super::super) relation: ReviewedTraceRelation,
    pub(in super::super) effect: SourceEffect,
    pub(in super::super) raw_slots: Option<RawSlotRetention>,
}

const fn raw_state(
    case: RawStateCase,
    outcome: RawStateOutcome,
    step: MapSourceStepId,
    slots: RawSlotRetention,
    trace: RawStateTraceDisposition,
) -> RawStateOutcomeRecord {
    RawStateOutcomeRecord {
        case,
        outcome,
        step,
        slots,
        trace,
    }
}

macro_rules! prefix_raw_state {
    ($case:ident, $outcome:ident, $step:ident, $slots:ident => $successor:ident) => {
        raw_state(
            RawStateCase::$case,
            RawStateOutcome::$outcome,
            $step,
            $slots,
            RawStateTraceDisposition::PrefixSuccessor(ReviewedTraceEndpoint::Step($successor)),
        )
    };
}

pub(in super::super) const RAW_STATE_OUTCOMES: &[RawStateOutcomeRecord] = &[
    raw_state(
        RawStateCase::AcceptedAfterTypedOperation,
        RawStateOutcome::AcceptedNormalReturn,
        RawStateAccepted,
        INSTALLED_RAW_VALUES,
        RawStateTraceDisposition::BeyondOpenFrontier(ReviewedOpenFrontier::TypedMapOperation),
    ),
    prefix_raw_state!(NullFile, NullFile, RawStateNullFile, NO_RAW_VALUES => RawAbandonNullFileRejected),
    prefix_raw_state!(Uninstalled, Uninstalled, RawStateUninstalled, NO_RAW_VALUES => RawAbandonEmpty),
    prefix_raw_state!(ForeignMethodsNullTableStatePresent, ForeignMethodsNullTable, RawStateForeignMethodsNullTable, OPAQUE_STATE_VALUE => RawAbandonForeignMethodsNullTableRejected),
    prefix_raw_state!(ForeignMethodsForeignTableStateNull, ForeignMethodsForeignTable, RawStateForeignMethodsForeignTable, METHODS_VALUE_ONLY => RawAbandonForeignMethodsForeignTableRejected),
    prefix_raw_state!(ForeignMethodsForeignTableStatePresent, ForeignMethodsForeignTable, RawStateForeignMethodsForeignTable, FOREIGN_METHODS_AND_OPAQUE_STATE => RawAbandonForeignMethodsForeignTableRejected),
    prefix_raw_state!(StateMissingInertTableStateNull, StateMissing, RawStateMissing, METHODS_VALUE_ONLY => RawAbandonStateMissingRejected),
    prefix_raw_state!(TypeMismatchInstalled, TypeMismatch, RawStateTypeMismatch, INSTALLED_RAW_VALUES => RawAbandonInstalled),
    raw_state(
        RawStateCase::CaughtUnwindFromTypedOperation,
        RawStateOutcome::CaughtUnwind,
        RawStateCaughtPanic,
        INSTALLED_RAW_VALUES,
        RawStateTraceDisposition::BeyondOpenFrontier(ReviewedOpenFrontier::TypedMapOperation),
    ),
];

const fn prefix_abandon_cause(case: RawStateCase) -> RawAbandonCause {
    RawAbandonCause {
        case,
        disposition: RawAbandonCauseDisposition::PrefixSuccessor,
    }
}

const fn beyond_frontier_abandon_cause(
    case: RawStateCase,
    frontier: ReviewedOpenFrontier,
) -> RawAbandonCause {
    RawAbandonCause {
        case,
        disposition: RawAbandonCauseDisposition::BeyondOpenFrontier(frontier),
    }
}

const fn raw_abandon(
    outcome: RawAbandonOutcome,
    causes: &'static [RawAbandonCause],
    step: MapSourceStepId,
    effect: SourceEffect,
    slots: RawSlotRetention,
) -> RawAbandonOutcomeRecord {
    RawAbandonOutcomeRecord {
        outcome,
        causes,
        step,
        effect,
        slots,
        prefix_successor: ReviewedTraceEndpoint::Step(RawFallbackProjection),
    }
}

macro_rules! raw_abandon_outcome {
    ($outcome:ident, [$($cause:ident),+], $step:ident, $effect:ident, $slots:ident) => {
        raw_abandon(
            RawAbandonOutcome::$outcome,
            &[$(prefix_abandon_cause(RawStateCase::$cause)),+],
            $step,
            SourceEffect::$effect,
            $slots,
        )
    };
}

pub(in super::super) const RAW_ABANDON_OUTCOMES: &[RawAbandonOutcomeRecord] = &[
    raw_abandon_outcome!(Empty, [Uninstalled], RawAbandonEmpty, None, NO_RAW_VALUES),
    raw_abandon(
        RawAbandonOutcome::InstalledDropCompleted,
        &[
            prefix_abandon_cause(RawStateCase::TypeMismatchInstalled),
            beyond_frontier_abandon_cause(
                RawStateCase::CaughtUnwindFromTypedOperation,
                ReviewedOpenFrontier::TypedMapOperation,
            ),
        ],
        RawAbandonInstalled,
        SourceEffect::Cleanup,
        NO_RAW_VALUES,
    ),
    raw_abandon(
        RawAbandonOutcome::InstalledDropUnwindCaught,
        &[
            prefix_abandon_cause(RawStateCase::TypeMismatchInstalled),
            beyond_frontier_abandon_cause(
                RawStateCase::CaughtUnwindFromTypedOperation,
                ReviewedOpenFrontier::TypedMapOperation,
            ),
        ],
        RawAbandonInstalled,
        SourceEffect::Cleanup,
        DROP_UNWIND_CUSTODY_PENDING,
    ),
    raw_abandon_outcome!(
        NullFileRejected,
        [NullFile],
        RawAbandonNullFileRejected,
        None,
        NO_RAW_VALUES
    ),
    raw_abandon_outcome!(
        ForeignMethodsNullTableRejected,
        [ForeignMethodsNullTableStatePresent],
        RawAbandonForeignMethodsNullTableRejected,
        None,
        OPAQUE_STATE_VALUE
    ),
    raw_abandon_outcome!(
        ForeignMethodsForeignTableStateNullRejected,
        [ForeignMethodsForeignTableStateNull],
        RawAbandonForeignMethodsForeignTableRejected,
        None,
        METHODS_VALUE_ONLY
    ),
    raw_abandon_outcome!(
        ForeignMethodsForeignTableStatePresentRejected,
        [ForeignMethodsForeignTableStatePresent],
        RawAbandonForeignMethodsForeignTableRejected,
        None,
        FOREIGN_METHODS_AND_OPAQUE_STATE
    ),
    raw_abandon_outcome!(
        StateMissingRejected,
        [StateMissingInertTableStateNull],
        RawAbandonStateMissingRejected,
        None,
        METHODS_VALUE_ONLY
    ),
];
