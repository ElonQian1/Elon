use std::collections::BTreeSet;

use super::super::model::{
    MapPendingReason, MapSiteId, MapSourceStep, MapSourceStepId, MapStepKind,
};

pub(super) fn validate(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    let ids = steps.iter().map(|step| step.id).collect::<BTreeSet<_>>();
    if ids.len() != steps.len() {
        return Err("Map review ledger contains a duplicate source step id");
    }
    if ids.contains(&MapSourceStepId::Count) || ids.len() != MapSourceStepId::Count as usize {
        return Err("Map review ledger does not materialize its declared source-step id set");
    }

    let sites = steps.iter().map(|step| step.site).collect::<BTreeSet<_>>();
    let expected_sites = [
        MapSiteId::AbiInput,
        MapSiteId::RawState,
        MapSiteId::OuterFault,
        MapSiteId::RoutePlan,
        MapSiteId::Promotion,
        MapSiteId::FaultInstall,
        MapSiteId::OperationCallback,
        MapSiteId::ManagedValidation,
        MapSiteId::NodeInitialization,
        MapSiteId::FileSize,
        MapSiteId::FileGrow,
        MapSiteId::MappingCreate,
        MapSiteId::ViewMap,
        MapSiteId::RegionSelection,
        MapSiteId::AbiProjection,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if sites != expected_sites {
        return Err("Map review ledger site partition is not the declared review scope");
    }

    let pending_reasons = steps
        .iter()
        .filter_map(|step| match step.kind {
            MapStepKind::Pending { reason, .. } => Some(reason),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let expected_pending = [
        MapPendingReason::RawAbandonSubbranch,
        MapPendingReason::RouteOrPlanPrecondition,
        MapPendingReason::PromotionCustodyVariant,
        MapPendingReason::CallbackOwnerVariant,
        MapPendingReason::PlatformTypedOutcome,
        MapPendingReason::PrefixMutationSplit,
        MapPendingReason::SymbolicLoopOrOccurrence,
        MapPendingReason::AbiInputShapeSplit,
        MapPendingReason::ControllerInternalFailure,
        MapPendingReason::ManagedStateInvariant,
        MapPendingReason::PlatformCfgScope,
        MapPendingReason::CallbackLifetimeOccurrence,
        MapPendingReason::SuccessPrestatePartition,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if !pending_reasons.is_superset(&expected_pending) {
        return Err("Map review ledger lost an explicit source-closure pending reason");
    }
    if !pending_reasons.contains(&MapPendingReason::CallbackOwnerVariant) {
        return Err("Map review ledger incorrectly appears source-exhaustive");
    }

    for step in steps {
        match step.kind {
            MapStepKind::CleanupRewrite(template)
                if template.cause == template.returned_terminal =>
            {
                return Err("Map cleanup rewrite erased its distinct terminal phase");
            }
            MapStepKind::Pending {
                terminal: None,
                reason,
            } if matches!(
                reason,
                MapPendingReason::DynamicObservableMissing
                    | MapPendingReason::AbiInputShapeSplit
                    | MapPendingReason::SuccessPrestatePartition
            ) => {}
            MapStepKind::Pending { terminal: None, .. } => {
                return Err("Map pending terminal lost its reviewed projection template");
            }
            _ => {}
        }
    }
    Ok(())
}
