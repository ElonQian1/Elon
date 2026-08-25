use std::collections::BTreeSet;

use super::super::super::super::model::SourceEffect;
use super::super::super::{
    model::{
        MapExit, MapMultiplicity, MapPendingReason, MapPhase, MapPhaseProjection, MapRetention,
        MapSourceStep, MapSourceStepId, MapStepKind, MapTerminalTemplate, MapTiming,
    },
    reviewed_trace::{
        RawCustodyRetention, RawSlotRetention, ReviewedSuccessorEdge, ReviewedTraceCondition,
        ReviewedTraceEndpoint, ReviewedTraceRelation,
    },
};

pub(super) type EdgeKey = (
    ReviewedTraceEndpoint,
    ReviewedTraceEndpoint,
    ReviewedTraceCondition,
    ReviewedTraceRelation,
    SourceEffect,
    Option<RawSlotRetention>,
);

pub(super) const NO_RAW_VALUES: RawSlotRetention = slots(false, false, RawCustodyRetention::None);
pub(super) const INSTALLED_RAW_VALUES: RawSlotRetention =
    slots(true, true, RawCustodyRetention::InstalledEnvelope);
pub(super) const OPAQUE_STATE_VALUE: RawSlotRetention =
    slots(false, true, RawCustodyRetention::OpaqueStatePossible);
pub(super) const METHODS_VALUE_ONLY: RawSlotRetention =
    slots(true, false, RawCustodyRetention::None);
pub(super) const FOREIGN_METHODS_AND_OPAQUE_STATE: RawSlotRetention =
    slots(true, true, RawCustodyRetention::OpaqueStatePossible);
pub(super) const DROP_UNWIND_CUSTODY_PENDING: RawSlotRetention =
    slots(false, false, RawCustodyRetention::DropUnwindPending);

const fn slots(
    methods_value_retained: bool,
    state_value_retained: bool,
    custody: RawCustodyRetention,
) -> RawSlotRetention {
    RawSlotRetention {
        methods_value_retained,
        state_value_retained,
        custody,
    }
}

pub(super) fn edge_key(successor: &ReviewedSuccessorEdge) -> EdgeKey {
    edge(
        successor.from,
        successor.to,
        successor.condition,
        successor.relation,
        successor.effect,
        successor.raw_slots,
    )
}

pub(super) const fn edge(
    from: ReviewedTraceEndpoint,
    to: ReviewedTraceEndpoint,
    condition: ReviewedTraceCondition,
    relation: ReviewedTraceRelation,
    effect: SourceEffect,
    raw_slots: Option<RawSlotRetention>,
) -> EdgeKey {
    (from, to, condition, relation, effect, raw_slots)
}

pub(super) fn require_edge(
    edges: &BTreeSet<EdgeKey>,
    expected: EdgeKey,
) -> Result<(), &'static str> {
    if !edges.contains(&expected) {
        return Err("Map ABI/raw reviewed successor edge set is not exact");
    }
    Ok(())
}

pub(super) fn find_step(
    steps: &[MapSourceStep],
    id: MapSourceStepId,
) -> Result<&MapSourceStep, &'static str> {
    steps
        .iter()
        .find(|step| step.id == id)
        .ok_or("Map ABI/raw invariant references an absent source step")
}

pub(super) fn require_kind(
    steps: &[MapSourceStep],
    id: MapSourceStepId,
    predicate: impl FnOnce(MapStepKind) -> bool,
) -> Result<(), &'static str> {
    if !predicate(find_step(steps, id)?.kind) {
        return Err("Map ABI/raw source step has the wrong disposition");
    }
    Ok(())
}

pub(super) fn require_raw_pending(
    steps: &[MapSourceStep],
    id: MapSourceStepId,
    retention: MapRetention,
) -> Result<(), &'static str> {
    let expected = MapTerminalTemplate {
        cause: MapPhase::RawStateGate,
        returned_terminal: MapPhase::RawStateGate,
        stored_poison: MapPhaseProjection::Pending,
        route_marker: MapPhaseProjection::Pending,
        timing: MapTiming::LocalDeterministic,
        exit: MapExit::AbiUnavailableNull,
        retention,
        multiplicity: MapMultiplicity::OncePerMapCall,
    };
    require_kind(steps, id, |kind| {
        matches!(
            kind,
            MapStepKind::Pending {
                terminal: Some(template),
                reason: MapPendingReason::RawFallbackCustodyProjection,
            } if template == expected
        )
    })
}
