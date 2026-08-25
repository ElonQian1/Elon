use super::super::super::model::{PathOp, SourceEffect};
use super::super::model::{
    MapExit, MapMultiplicity, MapPhase, MapPhaseProjection, MapRetention, MapSourceStep,
    MapSourceStepId, MapStepKind, MapTerminalTemplate, MapTiming, MapValueFlow, MAP_EXTEND,
    MAP_OBSERVE,
};

pub(super) fn validate(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    let null_writers = steps
        .iter()
        .filter(|step| step.effect == SourceEffect::OutputNull)
        .map(|step| step.id)
        .collect::<std::collections::BTreeSet<_>>();
    if null_writers != [MapSourceStepId::AbiNullFirst].into_iter().collect() {
        return Err("Map review ledger has an unexpected ABI output-null writer");
    }
    let pointer_writers = steps
        .iter()
        .filter(|step| step.effect == SourceEffect::OutputPointer)
        .map(|step| step.id)
        .collect::<std::collections::BTreeSet<_>>();
    if pointer_writers != [MapSourceStepId::AbiMappedProjection].into_iter().collect() {
        return Err("Map review ledger has an unexpected ABI output-pointer writer");
    }
    let value_flow = steps
        .iter()
        .filter(|step| step.value_flow != MapValueFlow::None)
        .map(|step| (step.id, step.value_flow))
        .collect::<std::collections::BTreeMap<_, _>>();
    let expected_value_flow = [
        (
            MapSourceStepId::AbiNullFirst,
            MapValueFlow::AbiNullWriteConditional,
        ),
        (
            MapSourceStepId::AbiNullOutputRejected,
            MapValueFlow::OutputSlotAbsent,
        ),
        (
            MapSourceStepId::ManagedMapped,
            MapValueFlow::TypedPointerCreated,
        ),
        (
            MapSourceStepId::AdapterMapped,
            MapValueFlow::TypedPointerCarried,
        ),
        (
            MapSourceStepId::AbiMappedProjection,
            MapValueFlow::AbiPointerWritten,
        ),
    ]
    .into_iter()
    .collect::<std::collections::BTreeMap<_, _>>();
    if value_flow != expected_value_flow {
        return Err("Map review ledger conflates typed pointer flow with the ABI output write");
    }

    for step in steps {
        let template = match step.kind {
            MapStepKind::Terminal(template)
            | MapStepKind::CleanupRewrite(template)
            | MapStepKind::Pending {
                terminal: Some(template),
                ..
            } => Some(template),
            _ => None,
        };
        if let Some(template) = template {
            if template.exit == MapExit::AbiOkMapped
                && step.id != MapSourceStepId::AbiMappedProjection
            {
                return Err("only the audited ABI projection may terminate with a mapped pointer");
            }
            if template.exit == MapExit::AbiOkNotPresent
                && step.id != MapSourceStepId::AbiNotPresentProjection
            {
                return Err("only the audited ABI projection may terminate as NotPresent");
            }
            if matches!(step.kind, MapStepKind::CleanupRewrite(_))
                && !matches!(
                    template.returned_terminal,
                    MapPhase::DmsExclusiveRelease | MapPhase::MappingClose | MapPhase::FileClose
                )
            {
                return Err("Map cleanup rewrite targets a non-cleanup terminal phase");
            }
        }
    }

    for id in [
        MapSourceStepId::ObserveNotPresent,
        MapSourceStepId::AdapterNotPresent,
        MapSourceStepId::AbiNotPresentProjection,
    ] {
        require_ops(steps, id, MAP_OBSERVE)?;
    }
    for id in [
        MapSourceStepId::FileGrowFaultBefore,
        MapSourceStepId::FileGrowNativeFailure,
        MapSourceStepId::FileGrowFaultAfterKnown,
        MapSourceStepId::FileGrowFaultAfterUncertain,
    ] {
        require_ops(steps, id, MAP_EXTEND)?;
    }
    for (id, effect) in [
        (
            MapSourceStepId::CoordinatorMutexPoisoned,
            SourceEffect::Poison,
        ),
        (MapSourceStepId::ExactOpenNativeFailure, SourceEffect::None),
        (
            MapSourceStepId::FileSizeAfterSelectorRejected,
            SourceEffect::None,
        ),
        (MapSourceStepId::ManagedInactive, SourceEffect::None),
        (
            MapSourceStepId::MappingCreateAfterMatchLostExcluded,
            SourceEffect::Poison,
        ),
        (MapSourceStepId::RegionLoopContinues, SourceEffect::None),
        (MapSourceStepId::RawAbandonInstalled, SourceEffect::Cleanup),
        (
            MapSourceStepId::ViewMapNullCustodyRetained,
            SourceEffect::RetainCustody,
        ),
        (MapSourceStepId::ViewMapNullPoisoned, SourceEffect::Poison),
    ] {
        require_effect(steps, id, effect)?;
    }
    let raw_drop = steps
        .iter()
        .find(|step| step.id == MapSourceStepId::RawAbandonInstalled)
        .ok_or("Map review raw-drop invariant references an absent step")?;
    if !matches!(
        raw_drop.kind,
        MapStepKind::Pending {
            terminal: Some(template),
            ..
        } if template.retention == MapRetention::BranchDependent
    ) {
        return Err("Map raw drop overclaims a resolved downstream custody outcome");
    }
    validate_cleanup_axes(steps)?;
    Ok(())
}

fn validate_cleanup_axes(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    let cleanup =
        |cause, returned_terminal, stored_poison, retention, multiplicity| MapTerminalTemplate {
            cause,
            returned_terminal,
            stored_poison,
            route_marker: MapPhaseProjection::Returned,
            timing: MapTiming::Cleanup,
            exit: MapExit::AbiUnavailableNull,
            retention,
            multiplicity,
        };
    let expected = [
        (
            MapSourceStepId::ExactOpenCloseRewrite,
            SourceEffect::RetainCustody,
            cleanup(
                MapPhase::ExactSiblingOpen,
                MapPhase::FileClose,
                MapPhaseProjection::Pending,
                MapRetention::FileCloseCustody,
                MapMultiplicity::OncePerMapCall,
            ),
        ),
        (
            MapSourceStepId::DmsExclusiveFaultBeforeCloseRewrite,
            SourceEffect::RetainCustody,
            cleanup(
                MapPhase::DmsExclusiveAcquire,
                MapPhase::FileClose,
                MapPhaseProjection::Pending,
                MapRetention::FileCloseCustody,
                MapMultiplicity::OncePerMapCall,
            ),
        ),
        (
            MapSourceStepId::DmsTruncateFaultBeforeReleaseFailed,
            SourceEffect::RetainCustody,
            cleanup(
                MapPhase::DmsTruncate,
                MapPhase::DmsExclusiveRelease,
                MapPhaseProjection::Returned,
                MapRetention::NodeCustody,
                MapMultiplicity::SymbolicPhaseOccurrence,
            ),
        ),
        (
            MapSourceStepId::DmsTruncateNativeReleaseFailed,
            SourceEffect::Poison,
            cleanup(
                MapPhase::DmsTruncate,
                MapPhase::DmsExclusiveRelease,
                MapPhaseProjection::Returned,
                MapRetention::NodeCustody,
                MapMultiplicity::OncePerMapCall,
            ),
        ),
        (
            MapSourceStepId::DmsTruncateCloseRewrite,
            SourceEffect::RetainCustody,
            cleanup(
                MapPhase::DmsTruncate,
                MapPhase::FileClose,
                MapPhaseProjection::Cause,
                MapRetention::FileCloseCustody,
                MapMultiplicity::SymbolicPhaseOccurrence,
            ),
        ),
        (
            MapSourceStepId::DmsSharedFaultBeforeCloseRewrite,
            SourceEffect::RetainCustody,
            cleanup(
                MapPhase::DmsSharedAcquire,
                MapPhase::FileClose,
                MapPhaseProjection::Pending,
                MapRetention::FileCloseCustody,
                MapMultiplicity::OncePerMapCall,
            ),
        ),
        (
            MapSourceStepId::ViewMapFaultBeforeCleanupFailed,
            SourceEffect::RetainCustody,
            cleanup(
                MapPhase::ViewMap,
                MapPhase::MappingClose,
                MapPhaseProjection::Cause,
                MapRetention::MappingCustody,
                MapMultiplicity::SymbolicRegionLoop,
            ),
        ),
        (
            MapSourceStepId::ViewMapNativeCleanupFailed,
            SourceEffect::RetainCustody,
            cleanup(
                MapPhase::ViewMap,
                MapPhase::MappingClose,
                MapPhaseProjection::Returned,
                MapRetention::MappingCustody,
                MapMultiplicity::SymbolicRegionLoop,
            ),
        ),
    ];
    let actual_ids = steps
        .iter()
        .filter_map(|step| matches!(step.kind, MapStepKind::CleanupRewrite(_)).then_some(step.id))
        .collect::<std::collections::BTreeSet<_>>();
    let expected_ids = expected
        .iter()
        .map(|(id, _, _)| *id)
        .collect::<std::collections::BTreeSet<_>>();
    if actual_ids != expected_ids {
        return Err("Map cleanup rewrite id set is not exact");
    }
    for (id, effect, template) in expected {
        let step = steps
            .iter()
            .find(|step| step.id == id)
            .ok_or("Map cleanup manifest references an absent step")?;
        if step.effect != effect || step.kind != MapStepKind::CleanupRewrite(template) {
            return Err("Map cleanup rewrite lost an exact effect or terminal axis");
        }
    }
    Ok(())
}

fn require_effect(
    steps: &[MapSourceStep],
    id: MapSourceStepId,
    expected: SourceEffect,
) -> Result<(), &'static str> {
    let step = steps
        .iter()
        .find(|step| step.id == id)
        .ok_or("Map review effect invariant references an absent step")?;
    if step.effect == expected {
        Ok(())
    } else {
        Err("Map review step has the wrong source effect")
    }
}

fn require_ops(
    steps: &[MapSourceStep],
    id: MapSourceStepId,
    expected: &[PathOp],
) -> Result<(), &'static str> {
    let step = steps
        .iter()
        .find(|step| step.id == id)
        .ok_or("Map review effect invariant references an absent step")?;
    if step.ops == expected {
        Ok(())
    } else {
        Err("Map review step has the wrong Observe/Extend reachability")
    }
}
