use super::super::super::super::model::{Epoch, SourceEffect, SourceOwnerId};
use super::super::super::model::{
    MapPendingReason, MapPhase, MapRetention, MapSiteId, MapSourceStep, MapSourceStepId,
    MapStepKind,
};
use super::route_callback_source_shapes::{
    pending_unavailable, require_source_order, validate_shape,
};

pub(super) fn validate(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    validate_shape(
        steps,
        MapSourceStepId::RouteNoPlan,
        MapSiteId::RoutePlan,
        SourceOwnerId::FixtureRouteFile,
        "fn prepare_first_main_shm_map",
        "let observer = self.inner.promote_main_to_wal_for_shm()?;",
        Epoch::MapRoutePreparation,
        SourceEffect::None,
        MapStepKind::Continuation,
        None,
    )?;
    validate_shape(
        steps,
        MapSourceStepId::FaultObserverRecordRejected,
        MapSiteId::RoutePlan,
        SourceOwnerId::FixtureRouteFile,
        "fn prepare_first_main_shm_map",
        "if let Err(code) = self.shm_faults.record_promoted(observer)",
        Epoch::MapRoutePreparation,
        SourceEffect::RetainCustody,
        pending_unavailable(
            MapPhase::RoutePreparation,
            MapRetention::RegistryMarkerBeforeCompletion,
            MapPendingReason::RouteOrPlanPrecondition,
        ),
        None,
    )?;
    validate_shape(
        steps,
        MapSourceStepId::FaultObserverRecorded,
        MapSiteId::RoutePlan,
        SourceOwnerId::FixtureFaultPlan,
        "fn record_promoted",
        "ManagedTestShmFaultPlanState::Promoted(observer)",
        Epoch::MapRoutePreparation,
        SourceEffect::CustodyMutation,
        MapStepKind::Continuation,
        None,
    )?;
    require_source_order(
        SourceOwnerId::FixtureRouteFile,
        "fn prepare_first_main_shm_map",
        &[
            "None => {",
            "let observer = self.inner.promote_main_to_wal_for_shm()?;",
            "if let Err(code) = self.shm_faults.record_promoted(observer)",
            "let _ = self.inner.retain_test_fault_bridge_failure(code);",
            "return Err(());",
        ],
    )
}
