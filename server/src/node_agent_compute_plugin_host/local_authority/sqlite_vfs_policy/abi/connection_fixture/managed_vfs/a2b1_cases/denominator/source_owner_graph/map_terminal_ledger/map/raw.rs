use super::super::super::model::{Epoch, SourceEffect, SourceOwnerId};
use super::super::model::*;

const fn pending(
    cause: MapPhase,
    retention: MapRetention,
    reason: MapPendingReason,
) -> MapStepKind {
    MapStepKind::Pending {
        terminal: Some(MapTerminalTemplate {
            cause,
            returned_terminal: cause,
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

const fn raw_pending(retention: MapRetention) -> MapStepKind {
    pending(
        MapPhase::RawStateGate,
        retention,
        MapPendingReason::RawFallbackCustodyProjection,
    )
}

const fn raw_borrow_context() -> SourceAnchor {
    source_anchor(
        SourceOwnerId::AbiRawState,
        "unsafe fn with_installed_state",
        "installed_envelope(file)?",
        1,
    )
}

const fn raw_abandon_context() -> SourceAnchor {
    source_anchor(
        SourceOwnerId::AbiRawState,
        "unsafe fn abandon_installed_state",
        "validate_installed(methods, state)?;",
        1,
    )
}

const fn map_run_code_context() -> SourceAnchor {
    source_anchor(
        SourceOwnerId::AbiIoShm,
        "unsafe extern \"C\" fn map",
        "file_state::run_code(",
        1,
    )
}

const fn run_code_abandon_context() -> SourceAnchor {
    source_anchor(
        SourceOwnerId::AbiFileState,
        "unsafe fn run_code",
        "abandon_without_unwind(file)",
        1,
    )
}

const fn validate_step(
    id: MapSourceStepId,
    site: MapSiteId,
    needle: &'static str,
    occurrence: u8,
    retention: MapRetention,
    call_context: SourceAnchor,
) -> MapSourceStep {
    with_call_context(
        step(
            id,
            site,
            SourceOwnerId::AbiRawState,
            "fn validate_installed",
            needle,
            occurrence,
            MAP_BOTH,
            Epoch::AbiInput,
            SourceEffect::None,
            raw_pending(retention),
        ),
        call_context,
    )
}

pub(in super::super) const STEPS: &[MapSourceStep] = &[
    step(
        MapSourceStepId::RawStateAccepted,
        MapSiteId::RawGate,
        SourceOwnerId::AbiRawState,
        "unsafe fn with_installed_state",
        "Ok(unsafe { envelope.with_typed(operation) })",
        1,
        MAP_BOTH,
        Epoch::AbiInput,
        SourceEffect::None,
        MapStepKind::Continuation,
    ),
    with_call_context(
        step(
            MapSourceStepId::RawNormalCodeProjection,
            MapSiteId::AbiProjection,
            SourceOwnerId::AbiFileState,
            "unsafe fn run_code",
            "Ok(Ok(code)) => code",
            1,
            MAP_BOTH,
            Epoch::AbiInput,
            SourceEffect::None,
            MapStepKind::StructuralJoin,
        ),
        map_run_code_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::RawStateNullFile,
            MapSiteId::RawGate,
            SourceOwnerId::AbiRawState,
            "unsafe fn installed_envelope",
            "RawSqliteFileStateRejection::NullFile",
            1,
            MAP_BOTH,
            Epoch::AbiInput,
            SourceEffect::None,
            raw_pending(MapRetention::None),
        ),
        raw_borrow_context(),
    ),
    validate_step(
        MapSourceStepId::RawStateUninstalled,
        MapSiteId::RawGate,
        "RawSqliteFileStateRejection::Uninstalled",
        1,
        MapRetention::None,
        raw_borrow_context(),
    ),
    validate_step(
        MapSourceStepId::RawStateForeignMethodsNullTable,
        MapSiteId::RawGate,
        "RawSqliteFileStateRejection::ForeignMethods",
        1,
        MapRetention::UnvalidatedRawStateSlot,
        raw_borrow_context(),
    ),
    validate_step(
        MapSourceStepId::RawStateForeignMethodsForeignTable,
        MapSiteId::RawGate,
        "RawSqliteFileStateRejection::ForeignMethods",
        2,
        MapRetention::BranchDependent,
        raw_borrow_context(),
    ),
    validate_step(
        MapSourceStepId::RawStateMissing,
        MapSiteId::RawGate,
        "RawSqliteFileStateRejection::StateMissing",
        1,
        MapRetention::None,
        raw_borrow_context(),
    ),
    step(
        MapSourceStepId::RawStateTypeMismatch,
        MapSiteId::RawGate,
        SourceOwnerId::AbiRawState,
        "unsafe fn with_installed_state",
        "RawSqliteFileStateRejection::TypeMismatch",
        1,
        MAP_BOTH,
        Epoch::AbiInput,
        SourceEffect::None,
        raw_pending(MapRetention::BranchDependent),
    ),
    with_call_context(
        step(
            MapSourceStepId::RawStateCaughtPanic,
            MapSiteId::RawGate,
            SourceOwnerId::AbiFileState,
            "unsafe fn run_code",
            "| Err(_) =>",
            1,
            MAP_BOTH,
            Epoch::AbiInput,
            SourceEffect::None,
            raw_pending(MapRetention::BranchDependent),
        ),
        map_run_code_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::RawAbandonUnwindFence,
            MapSiteId::AbiProjection,
            SourceOwnerId::AbiFileState,
            "unsafe fn abandon_without_unwind",
            "let _ = catch_unwind(AssertUnwindSafe(||",
            1,
            MAP_BOTH,
            Epoch::AbiInput,
            SourceEffect::None,
            MapStepKind::StructuralJoin,
        ),
        run_code_abandon_context(),
    ),
    step(
        MapSourceStepId::FileStateInnerMissing,
        MapSiteId::RawState,
        SourceOwnerId::AbiFileState,
        "fn file_mut",
        "self.file.as_deref_mut().ok_or(())",
        1,
        MAP_BOTH,
        Epoch::AbiInput,
        SourceEffect::None,
        pending(
            MapPhase::TypedFileStateGate,
            MapRetention::ExistingCustody,
            MapPendingReason::ManagedStateInvariant,
        ),
    ),
    step(
        MapSourceStepId::RawAbandonEmpty,
        MapSiteId::RawAbandon,
        SourceOwnerId::AbiRawState,
        "unsafe fn abandon_installed_state",
        "return Ok(false);",
        1,
        MAP_BOTH,
        Epoch::AbiInput,
        SourceEffect::None,
        raw_pending(MapRetention::None),
    ),
    step(
        MapSourceStepId::RawAbandonInstalled,
        MapSiteId::RawAbandon,
        SourceOwnerId::AbiRawState,
        "unsafe fn abandon_installed_state",
        "drop(Box::from_raw",
        1,
        MAP_BOTH,
        Epoch::AbiInput,
        SourceEffect::Cleanup,
        raw_pending(MapRetention::BranchDependent),
    ),
    step(
        MapSourceStepId::RawAbandonNullFileRejected,
        MapSiteId::RawAbandon,
        SourceOwnerId::AbiRawState,
        "unsafe fn abandon_installed_state",
        "RawSqliteFileStateRejection::NullFile",
        1,
        MAP_BOTH,
        Epoch::AbiInput,
        SourceEffect::None,
        raw_pending(MapRetention::None),
    ),
    validate_step(
        MapSourceStepId::RawAbandonForeignMethodsNullTableRejected,
        MapSiteId::RawAbandon,
        "RawSqliteFileStateRejection::ForeignMethods",
        1,
        MapRetention::UnvalidatedRawStateSlot,
        raw_abandon_context(),
    ),
    validate_step(
        MapSourceStepId::RawAbandonForeignMethodsForeignTableRejected,
        MapSiteId::RawAbandon,
        "RawSqliteFileStateRejection::ForeignMethods",
        2,
        MapRetention::BranchDependent,
        raw_abandon_context(),
    ),
    validate_step(
        MapSourceStepId::RawAbandonStateMissingRejected,
        MapSiteId::RawAbandon,
        "RawSqliteFileStateRejection::StateMissing",
        1,
        MapRetention::None,
        raw_abandon_context(),
    ),
    with_call_context(
        step(
            MapSourceStepId::RawFallbackProjection,
            MapSiteId::AbiProjection,
            SourceOwnerId::AbiFileState,
            "unsafe fn run_code",
            "fallback",
            2,
            MAP_BOTH,
            Epoch::AbiInput,
            SourceEffect::None,
            MapStepKind::StructuralJoin,
        ),
        map_run_code_context(),
    ),
];
