use std::collections::BTreeSet;

use super::super::{
    map,
    model::{
        MapBoundaryReviewStatus, MapExit, MapReviewGate, MapSourceStep, MapSourceStepId,
        MapSuccessFamily, MapSuccessFamilyRecord, MAP_EXTEND, MAP_OBSERVE,
    },
    scope::{
        DEEPEST_TYPED_BOUNDARY, OPEN_SOURCE_REVIEW_BOUNDARIES, PENDING_BOUNDARIES, REVIEW_GATES,
    },
};

pub(super) fn validate(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    let ids = steps.iter().map(|step| step.id).collect::<BTreeSet<_>>();
    validate_gates(&ids)?;
    validate_pending_boundaries(&ids)?;
    validate_open_source_review_boundaries()?;
    validate_success_family_candidates(&ids, map::SUCCESS_FAMILY_CANDIDATES)?;
    if DEEPEST_TYPED_BOUNDARY.is_empty() {
        return Err("Map review ledger has no declared deepest typed boundary");
    }
    Ok(())
}

fn validate_open_source_review_boundaries() -> Result<(), &'static str> {
    let gates = OPEN_SOURCE_REVIEW_BOUNDARIES
        .iter()
        .map(|boundary| boundary.gate)
        .collect::<BTreeSet<_>>();
    let expected = [
        MapReviewGate::AbiInputShapeSplit,
        MapReviewGate::PlatformCfgAndControllerInternals,
        MapReviewGate::PrefixMutationAndInitializationCrossProduct,
        MapReviewGate::ManagedDefensiveLeafExpansion,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if gates != expected || OPEN_SOURCE_REVIEW_BOUNDARIES.len() != 5 {
        return Err("Map open source-review boundary set is not exact");
    }
    Ok(())
}

fn validate_gates(ids: &BTreeSet<MapSourceStepId>) -> Result<(), &'static str> {
    let actual = REVIEW_GATES
        .iter()
        .map(|record| record.gate)
        .collect::<BTreeSet<_>>();
    let expected = [
        MapReviewGate::AbiInputShapeSplit,
        MapReviewGate::RawRejectionVsPanicSplit,
        MapReviewGate::RawStateExactFixtureExclusion,
        MapReviewGate::RouteAndPromotionExactFixtureExclusion,
        MapReviewGate::TypedPlatformOutcomeExpansion,
        MapReviewGate::PrefixMutationAndInitializationCrossProduct,
        MapReviewGate::SymbolicRegionLoopAndFaultOccurrence,
        MapReviewGate::CallbackAndCustodyProjectionClosure,
        MapReviewGate::DynamicTerminalRewriteObservation,
        MapReviewGate::PlatformCfgAndControllerInternals,
        MapReviewGate::ManagedDefensiveLeafExpansion,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if actual != expected || actual.len() != REVIEW_GATES.len() {
        return Err("Map review gate set is not exact");
    }
    for gate in REVIEW_GATES {
        if gate.witnesses.is_empty() || gate.witnesses.iter().any(|id| !ids.contains(id)) {
            return Err("Map review gate has an empty or detached witness set");
        }
    }
    Ok(())
}

fn validate_pending_boundaries(ids: &BTreeSet<MapSourceStepId>) -> Result<(), &'static str> {
    let nodes = PENDING_BOUNDARIES
        .iter()
        .map(|record| record.node)
        .collect::<BTreeSet<_>>();
    if nodes.len() != PENDING_BOUNDARIES.len() || PENDING_BOUNDARIES.len() != 10 {
        return Err("Map pending graph-boundary closure set is not exact");
    }
    let statuses = PENDING_BOUNDARIES
        .iter()
        .map(|record| record.status)
        .collect::<BTreeSet<_>>();
    let required = [
        MapBoundaryReviewStatus::AnchoredButGraphPending,
        MapBoundaryReviewStatus::BudgetOwnerGraphGap,
        MapBoundaryReviewStatus::FileSizeGrowGraphConflated,
        MapBoundaryReviewStatus::CrossLedgerStateWitnessPending,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if statuses != required {
        return Err("Map pending graph-boundary statuses lost a known closure gap");
    }
    for boundary in PENDING_BOUNDARIES {
        if boundary.witnesses.is_empty() || boundary.witnesses.iter().any(|id| !ids.contains(id)) {
            return Err("Map pending graph boundary has an empty or detached witness set");
        }
    }
    Ok(())
}

fn validate_success_family_candidates(
    ids: &BTreeSet<MapSourceStepId>,
    families: &[MapSuccessFamilyRecord],
) -> Result<(), &'static str> {
    let actual = families
        .iter()
        .map(|family| family.family)
        .collect::<BTreeSet<_>>();
    let expected = [
        MapSuccessFamily::ExtendColdCreate,
        MapSuccessFamily::ExtendWarmCreate,
        MapSuccessFamily::ExtendReuse,
        MapSuccessFamily::ObserveWarmCreate,
        MapSuccessFamily::ObserveReuse,
        MapSuccessFamily::ObserveNotPresent,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if actual != expected || actual.len() != families.len() {
        return Err("Map review ledger does not preserve the six authority success candidates");
    }
    for family in families {
        if family.witnesses.is_empty() || family.witnesses.iter().any(|id| !ids.contains(id)) {
            return Err("Map success family has an empty or detached witness set");
        }
        match family.family {
            MapSuccessFamily::ObserveNotPresent
                if family.ops != MAP_OBSERVE || family.exit != MapExit::AbiOkNotPresent =>
            {
                return Err("Observe NotPresent has the wrong operation or ABI exit")
            }
            MapSuccessFamily::ObserveWarmCreate | MapSuccessFamily::ObserveReuse
                if family.ops != MAP_OBSERVE || family.exit != MapExit::AbiOkMapped =>
            {
                return Err("Observe mapped family has the wrong operation or ABI exit")
            }
            MapSuccessFamily::ExtendColdCreate
            | MapSuccessFamily::ExtendWarmCreate
            | MapSuccessFamily::ExtendReuse
                if family.ops != MAP_EXTEND || family.exit != MapExit::AbiOkMapped =>
            {
                return Err("Extend mapped family has the wrong operation or ABI exit")
            }
            _ => {}
        }
        let required_projection = match family.exit {
            MapExit::AbiOkMapped => [
                MapSourceStepId::ManagedMapped,
                MapSourceStepId::AdapterMapped,
                MapSourceStepId::AbiMappedProjection,
            ],
            MapExit::AbiOkNotPresent => [
                MapSourceStepId::ObserveNotPresent,
                MapSourceStepId::AdapterNotPresent,
                MapSourceStepId::AbiNotPresentProjection,
            ],
            _ => return Err("Map success candidate uses a non-success ABI exit"),
        };
        if required_projection
            .iter()
            .any(|required| !family.witnesses.contains(required))
        {
            return Err(
                "Map success candidate is detached from its managed/adapter/ABI projection",
            );
        }
        let _symbolic = family.unresolved_multiplicity;
        if !family.prestate_partition_pending {
            return Err("Map success candidate incorrectly claims a closed prestate partition");
        }
    }
    Ok(())
}
