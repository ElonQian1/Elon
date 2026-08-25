use super::super::super::super::{
    model::{Epoch, SourceEffect, SourceOwnerId},
    owners,
};
use super::super::super::model::{
    MapExclusionReason, MapExit, MapMultiplicity, MapPendingReason, MapPhase, MapPhaseProjection,
    MapRetention, MapSiteId, MapSourceStep, MapSourceStepId, MapStepKind, MapTerminalTemplate,
    MapTiming, MapValueFlow, SourceAnchor, MAP_BOTH,
};
use super::super::anchors::source_symbol_span;

pub(super) fn validate(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    validate_outer_failure_shapes(steps)?;
    validate_dispatch_shapes(steps)?;
    validate_callback_shapes(steps)?;
    validate_exact_source_order()
}

fn validate_outer_failure_shapes(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    validate_shape(
        steps,
        MapSourceStepId::OuterFaultPass,
        MapSiteId::OuterFault,
        SourceOwnerId::FixtureFaultFile,
        "fn shm_map",
        "self.inner()?.shm_map(region, region_size, extend)",
        Epoch::AbiInput,
        SourceEffect::None,
        MapStepKind::Continuation,
        None,
    )?;
    validate_shape(
        steps,
        MapSourceStepId::RawStateAccepted,
        MapSiteId::RawGate,
        SourceOwnerId::AbiRawState,
        "unsafe fn with_installed_state",
        "Ok(unsafe { envelope.with_typed(operation) })",
        Epoch::AbiInput,
        SourceEffect::None,
        MapStepKind::Continuation,
        None,
    )?;
    validate_shape(
        steps,
        MapSourceStepId::RawNormalCodeProjection,
        MapSiteId::AbiProjection,
        SourceOwnerId::AbiFileState,
        "unsafe fn run_code",
        "Ok(Ok(code)) => code",
        Epoch::AbiInput,
        SourceEffect::None,
        MapStepKind::StructuralJoin,
        Some(source_anchor(
            SourceOwnerId::AbiIoShm,
            "unsafe extern \"C\" fn map",
            "file_state::run_code(",
        )),
    )?;
    validate_shape(
        steps,
        MapSourceStepId::AbiFailureProjection,
        MapSiteId::AbiProjection,
        SourceOwnerId::AbiIoShm,
        "unsafe extern \"C\" fn map",
        "Err(()) => result_codes::SHM_MAP_UNAVAILABLE",
        Epoch::AbiInput,
        SourceEffect::None,
        MapStepKind::StructuralJoin,
        None,
    )
}

fn validate_dispatch_shapes(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    validate_shape(
        steps,
        MapSourceStepId::RoutePreparationResultGate,
        MapSiteId::RoutePlan,
        SourceOwnerId::FixtureRouteFile,
        "fn shm_map",
        "self.prepare_first_main_shm_map()?;",
        Epoch::MapRoutePreparation,
        SourceEffect::None,
        MapStepKind::StructuralJoin,
        Some(outer_route_context()),
    )?;
    for (id, owner, symbol, needle, context) in [
        (
            MapSourceStepId::RouteOperationDispatch,
            SourceOwnerId::FixtureRouteFile,
            "fn shm_map",
            "self.inner.shm_map(region, region_size, extend)",
            outer_route_context(),
        ),
        (
            MapSourceStepId::BridgeOperationDispatch,
            SourceOwnerId::RegistryTestBridge,
            "fn shm_map",
            "self.file.shm_map(region, region_size, extend)",
            source_anchor(
                SourceOwnerId::FixtureRouteFile,
                "fn shm_map",
                "self.inner.shm_map(region, region_size, extend)",
            ),
        ),
        (
            MapSourceStepId::AdapterOperationDispatch,
            SourceOwnerId::RegistryAbiFile,
            "fn shm_map",
            "self.file.shm_map(region, region_size, mode).map_err(drop)?",
            source_anchor(
                SourceOwnerId::RegistryTestBridge,
                "fn shm_map",
                "self.file.shm_map(region, region_size, extend)",
            ),
        ),
        (
            MapSourceStepId::RegistryOperationDispatch,
            SourceOwnerId::RegistryOperations,
            "pub(super) fn shm_map",
            "self.with_shm(|shm| shm.map(region, region_size, mode))",
            source_anchor(
                SourceOwnerId::RegistryAbiFile,
                "fn shm_map",
                "self.file.shm_map(region, region_size, mode).map_err(drop)?",
            ),
        ),
    ] {
        validate_shape(
            steps,
            id,
            MapSiteId::OperationCallback,
            owner,
            symbol,
            needle,
            Epoch::WalMainSteady,
            SourceEffect::None,
            MapStepKind::Continuation,
            Some(context),
        )?;
    }
    Ok(())
}

fn validate_callback_shapes(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    let map_context = Some(registry_map_context());
    validate_shape(
        steps,
        MapSourceStepId::OperationAdmissionRejected,
        MapSiteId::OperationCallback,
        SourceOwnerId::RegistryOperations,
        "fn with_shm<T>",
        ".begin_callback(self.route, ManagedSqliteRegistryCallbackKind::Shm)",
        Epoch::WalMainSteady,
        SourceEffect::None,
        pending_unavailable(
            MapPhase::OperationCallbackAdmission,
            MapRetention::ExistingCustody,
            MapPendingReason::CallbackOwnerVariant,
        ),
        map_context,
    )?;
    for (id, needle) in [
        (
            MapSourceStepId::OperationUnsupportedRole,
            "ManagedSqliteRegistryPinnedFileOperationRejection::UnsupportedFileRole",
        ),
        (
            MapSourceStepId::OperationShmDetached,
            "ManagedSqliteRegistryPinnedFileOperationRejection::ShmDetached",
        ),
    ] {
        validate_shape(
            steps,
            id,
            MapSiteId::OperationCallback,
            SourceOwnerId::RegistryOperations,
            "fn with_shm<T>",
            needle,
            Epoch::WalMainSteady,
            SourceEffect::None,
            MapStepKind::Excluded(MapExclusionReason::ExactFixtureInvariant),
            map_context,
        )?;
    }
    validate_shape(
        steps,
        MapSourceStepId::OperationManagedFailure,
        MapSiteId::OperationCallback,
        SourceOwnerId::RegistryOperations,
        "fn with_shm<T>",
        "ManagedSqliteRegistryPinnedFileOperationRejection::Shm(failure)",
        Epoch::WalMainSteady,
        SourceEffect::None,
        MapStepKind::Continuation,
        map_context,
    )?;
    validate_shape(
        steps,
        MapSourceStepId::OperationUnsafeRetain,
        MapSiteId::OperationCallback,
        SourceOwnerId::RegistryOperations,
        "fn quarantine_unsafe_shm_failure",
        "let _ = self.owner.retain_terminal_custody(",
        Epoch::WalMainSteady,
        SourceEffect::RetainCustody,
        MapStepKind::Continuation,
        Some(source_anchor(
            SourceOwnerId::RegistryOperations,
            "fn with_shm<T>",
            "self.quarantine_unsafe_shm_failure(failure)",
        )),
    )?;
    validate_shape(
        steps,
        MapSourceStepId::OperationCompletionAttempt,
        MapSiteId::OperationCallback,
        SourceOwnerId::RegistryOperations,
        "fn with_shm<T>",
        "match (result, callback.complete())",
        Epoch::WalMainSteady,
        SourceEffect::CallbackLease,
        MapStepKind::StructuralJoin,
        map_context,
    )?;
    for (id, needle, kind) in [
        (
            MapSourceStepId::OperationCompletionResultDomain,
            "-> Result<(), ManagedSqliteRegistryProcessRouteRejection>",
            MapStepKind::StructuralJoin,
        ),
        (
            MapSourceStepId::OperationCompletionDelegate,
            "self.owner.finish_callback(self.route, lease)",
            MapStepKind::Continuation,
        ),
    ] {
        validate_shape(
            steps,
            id,
            MapSiteId::OperationCallback,
            SourceOwnerId::RegistryProcessOwner,
            "pub(super) fn complete",
            needle,
            Epoch::WalMainSteady,
            SourceEffect::CallbackLease,
            kind,
            Some(source_anchor(
                SourceOwnerId::RegistryOperations,
                "fn with_shm<T>",
                "callback.complete()",
            )),
        )?;
    }
    validate_shape(
        steps,
        MapSourceStepId::OperationErrorWinsCompletion,
        MapSiteId::OperationCallback,
        SourceOwnerId::RegistryOperations,
        "fn with_shm<T>",
        "(Err(rejection), _) => Err(rejection)",
        Epoch::WalMainSteady,
        SourceEffect::CallbackLease,
        MapStepKind::StructuralJoin,
        map_context,
    )?;
    validate_shape(
        steps,
        MapSourceStepId::OperationCompletionRejected,
        MapSiteId::OperationCallback,
        SourceOwnerId::RegistryOperations,
        "fn with_shm<T>",
        "(Ok(value), Err(rejection)) => Err(",
        Epoch::WalMainSteady,
        SourceEffect::CallbackLease,
        pending_unavailable(
            MapPhase::OperationCallbackCompletion,
            MapRetention::ExistingCustody,
            MapPendingReason::CallbackOwnerVariant,
        ),
        map_context,
    )?;
    validate_shape(
        steps,
        MapSourceStepId::OperationCompleted,
        MapSiteId::OperationCallback,
        SourceOwnerId::RegistryOperations,
        "fn with_shm<T>",
        "(Ok(value), Ok(())) => Ok(value)",
        Epoch::WalMainSteady,
        SourceEffect::CallbackLease,
        MapStepKind::Continuation,
        map_context,
    )
}

fn validate_exact_source_order() -> Result<(), &'static str> {
    require_source_order(
        SourceOwnerId::FixtureRouteFile,
        "fn shm_map",
        &[
            "self.prepare_first_main_shm_map()?;",
            "self.inner.shm_map(region, region_size, extend)",
        ],
    )?;
    require_source_order(
        SourceOwnerId::RegistryOperations,
        "fn with_shm<T>",
        &[
            ".begin_callback(self.route, ManagedSqliteRegistryCallbackKind::Shm)",
            "if let Err(ManagedSqliteRegistryPinnedFileOperationRejection::Shm(failure)) = &result",
            "self.quarantine_unsafe_shm_failure(failure)",
            "match (result, callback.complete())",
            "(Err(rejection), _) => Err(rejection)",
            "(Ok(value), Err(rejection)) => Err(",
            "(Ok(value), Ok(())) => Ok(value)",
        ],
    )?;
    require_source_order(
        SourceOwnerId::RegistryOperations,
        "fn quarantine_unsafe_shm_failure",
        &[
            "if failure.class()",
            "!= crate::node_agent_managed_fs::ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned",
            "&& !failure.mutation_may_have_occurred()",
            "&& !failure.lock_outcome_uncertain()",
            "return;",
            "let marker = ManagedSqliteRegistryUnsafeShmFailureMarker",
            "let _ = self.owner.retain_terminal_custody(",
        ],
    )?;
    require_source_order(
        SourceOwnerId::RegistryProcessOwner,
        "pub(super) fn complete",
        &[
            "-> Result<(), ManagedSqliteRegistryProcessRouteRejection>",
            "self.owner.finish_callback(self.route, lease)",
        ],
    )
}

#[allow(clippy::too_many_arguments)]
fn validate_shape(
    steps: &[MapSourceStep],
    id: MapSourceStepId,
    site: MapSiteId,
    owner: SourceOwnerId,
    symbol: &'static str,
    needle: &'static str,
    epoch: Epoch,
    effect: SourceEffect,
    kind: MapStepKind,
    context: Option<SourceAnchor>,
) -> Result<(), &'static str> {
    let step = require_step(steps, id)?;
    if step.site != site
        || step.anchor != source_anchor(owner, symbol, needle)
        || step.call_context != context
        || step.ops != MAP_BOTH
        || step.epoch != epoch
        || step.effect != effect
        || step.value_flow != MapValueFlow::None
        || step.kind != kind
    {
        return Err("Map route/callback source witness changed its exact reviewed shape");
    }
    Ok(())
}

fn require_source_order(
    owner: SourceOwnerId,
    symbol: &'static str,
    needles: &[&'static str],
) -> Result<(), &'static str> {
    let source = owners::source_content(owner);
    let mut tail = source_symbol_span(source, symbol)
        .ok_or("Map route/callback ordered source symbol span is missing")?;
    for needle in needles {
        let offset = tail
            .find(needle)
            .ok_or("Map route/callback source needles are absent or reordered")?;
        tail = tail
            .get(offset + needle.len()..)
            .ok_or("Map route/callback ordered source suffix is invalid")?;
    }
    Ok(())
}

fn pending_unavailable(
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

fn require_step(
    steps: &[MapSourceStep],
    id: MapSourceStepId,
) -> Result<&MapSourceStep, &'static str> {
    steps
        .iter()
        .find(|step| step.id == id)
        .ok_or("Map route/callback fragment lost a required source-ledger witness")
}

const fn source_anchor(
    owner: SourceOwnerId,
    symbol: &'static str,
    needle: &'static str,
) -> SourceAnchor {
    SourceAnchor {
        owner,
        symbol,
        needle,
        occurrence: 1,
    }
}

const fn outer_route_context() -> SourceAnchor {
    source_anchor(
        SourceOwnerId::FixtureFaultFile,
        "fn shm_map",
        "self.inner()?.shm_map(region, region_size, extend)",
    )
}

const fn registry_map_context() -> SourceAnchor {
    source_anchor(
        SourceOwnerId::RegistryOperations,
        "pub(super) fn shm_map",
        "self.with_shm(|shm| shm.map(region, region_size, mode))",
    )
}
