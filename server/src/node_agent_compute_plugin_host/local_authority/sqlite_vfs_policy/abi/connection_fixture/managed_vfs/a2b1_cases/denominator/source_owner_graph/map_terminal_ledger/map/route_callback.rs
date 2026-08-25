use super::super::super::model::{Epoch, SourceEffect, SourceOwnerId};
use super::super::model::*;

const fn outer_route_context() -> SourceAnchor {
    source_anchor(
        SourceOwnerId::FixtureFaultFile,
        "fn shm_map",
        "self.inner()?.shm_map(region, region_size, extend)",
        1,
    )
}

const fn route_dispatch_context() -> SourceAnchor {
    source_anchor(
        SourceOwnerId::FixtureRouteFile,
        "fn shm_map",
        "self.inner.shm_map(region, region_size, extend)",
        1,
    )
}

const fn bridge_dispatch_context() -> SourceAnchor {
    source_anchor(
        SourceOwnerId::RegistryTestBridge,
        "fn shm_map",
        "self.file.shm_map(region, region_size, extend)",
        1,
    )
}

const fn adapter_dispatch_context() -> SourceAnchor {
    source_anchor(
        SourceOwnerId::RegistryAbiFile,
        "fn shm_map",
        "self.file.shm_map(region, region_size, mode).map_err(drop)?",
        1,
    )
}

const fn registry_map_context() -> SourceAnchor {
    source_anchor(
        SourceOwnerId::RegistryOperations,
        "pub(super) fn shm_map",
        "self.with_shm(|shm| shm.map(region, region_size, mode))",
        1,
    )
}

const fn completion_call_context() -> SourceAnchor {
    source_anchor(
        SourceOwnerId::RegistryOperations,
        "fn with_shm<T>",
        "callback.complete()",
        1,
    )
}

const fn quarantine_call_context() -> SourceAnchor {
    source_anchor(
        SourceOwnerId::RegistryOperations,
        "fn with_shm<T>",
        "self.quarantine_unsafe_shm_failure(failure)",
        1,
    )
}

const fn pending_unavailable(
    phase: MapPhase,
    retention: MapRetention,
    reason: MapPendingReason,
) -> MapStepKind {
    MapStepKind::Pending {
        terminal: Some(MapTerminalTemplate {
            cause: phase,
            returned_terminal: phase,
            stored_poison: MapPhaseProjection::Pending,
            route_marker: MapPhaseProjection::Pending,
            timing: MapTiming::LocalDeterministic,
            exit: MapExit::AbiUnavailableNull,
            retention,
            multiplicity: MapMultiplicity::OncePerMapCall,
        }),
        reason,
    }
}

pub(in super::super) const STEPS: &[MapSourceStep] = &[
    with_call_context(
        step(
            MapSourceStepId::RoutePreparationResultGate,
            MapSiteId::RoutePlan,
            SourceOwnerId::FixtureRouteFile,
            "fn shm_map",
            "self.prepare_first_main_shm_map()?;",
            1,
            MAP_BOTH,
            Epoch::MapRoutePreparation,
            SourceEffect::None,
            MapStepKind::StructuralJoin,
        ),
        outer_route_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::RouteOperationDispatch,
            MapSiteId::OperationCallback,
            SourceOwnerId::FixtureRouteFile,
            "fn shm_map",
            "self.inner.shm_map(region, region_size, extend)",
            1,
            MAP_BOTH,
            Epoch::WalMainSteady,
            SourceEffect::None,
            MapStepKind::Continuation,
        ),
        outer_route_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::BridgeOperationDispatch,
            MapSiteId::OperationCallback,
            SourceOwnerId::RegistryTestBridge,
            "fn shm_map",
            "self.file.shm_map(region, region_size, extend)",
            1,
            MAP_BOTH,
            Epoch::WalMainSteady,
            SourceEffect::None,
            MapStepKind::Continuation,
        ),
        route_dispatch_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::AdapterOperationDispatch,
            MapSiteId::OperationCallback,
            SourceOwnerId::RegistryAbiFile,
            "fn shm_map",
            "self.file.shm_map(region, region_size, mode).map_err(drop)?",
            1,
            MAP_BOTH,
            Epoch::WalMainSteady,
            SourceEffect::None,
            MapStepKind::Continuation,
        ),
        bridge_dispatch_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::RegistryOperationDispatch,
            MapSiteId::OperationCallback,
            SourceOwnerId::RegistryOperations,
            "pub(super) fn shm_map",
            "self.with_shm(|shm| shm.map(region, region_size, mode))",
            1,
            MAP_BOTH,
            Epoch::WalMainSteady,
            SourceEffect::None,
            MapStepKind::Continuation,
        ),
        adapter_dispatch_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::OperationAdmissionRejected,
            MapSiteId::OperationCallback,
            SourceOwnerId::RegistryOperations,
            "fn with_shm<T>",
            ".begin_callback(self.route, ManagedSqliteRegistryCallbackKind::Shm)",
            1,
            MAP_BOTH,
            Epoch::WalMainSteady,
            SourceEffect::None,
            pending_unavailable(
                MapPhase::OperationCallbackAdmission,
                MapRetention::ExistingCustody,
                MapPendingReason::CallbackOwnerVariant,
            ),
        ),
        registry_map_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::OperationUnsupportedRole,
            MapSiteId::OperationCallback,
            SourceOwnerId::RegistryOperations,
            "fn with_shm<T>",
            "ManagedSqliteRegistryPinnedFileOperationRejection::UnsupportedFileRole",
            1,
            MAP_BOTH,
            Epoch::WalMainSteady,
            SourceEffect::None,
            MapStepKind::Excluded(MapExclusionReason::ExactFixtureInvariant),
        ),
        registry_map_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::OperationShmDetached,
            MapSiteId::OperationCallback,
            SourceOwnerId::RegistryOperations,
            "fn with_shm<T>",
            "ManagedSqliteRegistryPinnedFileOperationRejection::ShmDetached",
            1,
            MAP_BOTH,
            Epoch::WalMainSteady,
            SourceEffect::None,
            MapStepKind::Excluded(MapExclusionReason::ExactFixtureInvariant),
        ),
        registry_map_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::OperationManagedFailure,
            MapSiteId::OperationCallback,
            SourceOwnerId::RegistryOperations,
            "fn with_shm<T>",
            "ManagedSqliteRegistryPinnedFileOperationRejection::Shm(failure)",
            1,
            MAP_BOTH,
            Epoch::WalMainSteady,
            SourceEffect::None,
            MapStepKind::Continuation,
        ),
        registry_map_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::OperationUnsafeRetain,
            MapSiteId::OperationCallback,
            SourceOwnerId::RegistryOperations,
            "fn quarantine_unsafe_shm_failure",
            "let _ = self.owner.retain_terminal_custody(",
            1,
            MAP_BOTH,
            Epoch::WalMainSteady,
            SourceEffect::RetainCustody,
            MapStepKind::Continuation,
        ),
        quarantine_call_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::OperationCompletionAttempt,
            MapSiteId::OperationCallback,
            SourceOwnerId::RegistryOperations,
            "fn with_shm<T>",
            "match (result, callback.complete())",
            1,
            MAP_BOTH,
            Epoch::WalMainSteady,
            SourceEffect::CallbackLease,
            MapStepKind::StructuralJoin,
        ),
        registry_map_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::OperationCompletionResultDomain,
            MapSiteId::OperationCallback,
            SourceOwnerId::RegistryProcessOwner,
            "pub(super) fn complete",
            "-> Result<(), ManagedSqliteRegistryProcessRouteRejection>",
            1,
            MAP_BOTH,
            Epoch::WalMainSteady,
            SourceEffect::CallbackLease,
            MapStepKind::StructuralJoin,
        ),
        completion_call_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::OperationCompletionDelegate,
            MapSiteId::OperationCallback,
            SourceOwnerId::RegistryProcessOwner,
            "pub(super) fn complete",
            "self.owner.finish_callback(self.route, lease)",
            1,
            MAP_BOTH,
            Epoch::WalMainSteady,
            SourceEffect::CallbackLease,
            MapStepKind::Continuation,
        ),
        completion_call_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::OperationErrorWinsCompletion,
            MapSiteId::OperationCallback,
            SourceOwnerId::RegistryOperations,
            "fn with_shm<T>",
            "(Err(rejection), _) => Err(rejection)",
            1,
            MAP_BOTH,
            Epoch::WalMainSteady,
            SourceEffect::CallbackLease,
            MapStepKind::StructuralJoin,
        ),
        registry_map_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::OperationCompletionRejected,
            MapSiteId::OperationCallback,
            SourceOwnerId::RegistryOperations,
            "fn with_shm<T>",
            "(Ok(value), Err(rejection)) => Err(",
            1,
            MAP_BOTH,
            Epoch::WalMainSteady,
            SourceEffect::CallbackLease,
            pending_unavailable(
                MapPhase::OperationCallbackCompletion,
                MapRetention::ExistingCustody,
                MapPendingReason::CallbackOwnerVariant,
            ),
        ),
        registry_map_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::OperationCompleted,
            MapSiteId::OperationCallback,
            SourceOwnerId::RegistryOperations,
            "fn with_shm<T>",
            "(Ok(value), Ok(())) => Ok(value)",
            1,
            MAP_BOTH,
            Epoch::WalMainSteady,
            SourceEffect::CallbackLease,
            MapStepKind::Continuation,
        ),
        registry_map_context(),
    ),
];
