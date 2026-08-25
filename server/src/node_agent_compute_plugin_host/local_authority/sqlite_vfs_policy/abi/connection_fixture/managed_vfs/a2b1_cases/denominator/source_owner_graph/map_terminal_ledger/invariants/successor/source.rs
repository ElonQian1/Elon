use super::super::super::super::model::{Epoch, SourceEffect, SourceOwnerId};
use super::super::super::model::{
    source_anchor, MapRetention, MapSiteId, MapSourceStep, MapSourceStepId, MapStepKind,
    MapValueFlow, SourceAnchor, MAP_BOTH,
};
use super::shared::{find_step, require_kind, require_raw_pending};

pub(super) fn validate(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    validate_source_shapes(steps)?;
    validate_call_contexts(steps)?;
    require_kind(steps, MapSourceStepId::AbiInputRejected, |kind| {
        matches!(kind, MapStepKind::StructuralJoin)
    })?;
    require_kind(steps, MapSourceStepId::RawStateAccepted, |kind| {
        matches!(kind, MapStepKind::Continuation)
    })?;
    require_kind(steps, MapSourceStepId::RawFallbackProjection, |kind| {
        matches!(kind, MapStepKind::StructuralJoin)
    })?;

    for (id, retention) in [
        (MapSourceStepId::RawStateNullFile, MapRetention::None),
        (MapSourceStepId::RawStateUninstalled, MapRetention::None),
        (
            MapSourceStepId::RawStateForeignMethodsNullTable,
            MapRetention::UnvalidatedRawStateSlot,
        ),
        (
            MapSourceStepId::RawStateForeignMethodsForeignTable,
            MapRetention::BranchDependent,
        ),
        (MapSourceStepId::RawStateMissing, MapRetention::None),
        (
            MapSourceStepId::RawStateTypeMismatch,
            MapRetention::BranchDependent,
        ),
        (
            MapSourceStepId::RawStateCaughtPanic,
            MapRetention::BranchDependent,
        ),
        (MapSourceStepId::RawAbandonEmpty, MapRetention::None),
        (
            MapSourceStepId::RawAbandonInstalled,
            MapRetention::BranchDependent,
        ),
        (
            MapSourceStepId::RawAbandonNullFileRejected,
            MapRetention::None,
        ),
        (
            MapSourceStepId::RawAbandonForeignMethodsNullTableRejected,
            MapRetention::UnvalidatedRawStateSlot,
        ),
        (
            MapSourceStepId::RawAbandonForeignMethodsForeignTableRejected,
            MapRetention::BranchDependent,
        ),
        (
            MapSourceStepId::RawAbandonStateMissingRejected,
            MapRetention::None,
        ),
    ] {
        require_raw_pending(steps, id, retention)?;
    }

    Ok(())
}

fn validate_source_shapes(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    for (id, site, owner, symbol, needle, occurrence, effect) in [
        (
            MapSourceStepId::RawStateAccepted,
            MapSiteId::RawGate,
            SourceOwnerId::AbiRawState,
            "unsafe fn with_installed_state",
            "Ok(unsafe { envelope.with_typed(operation) })",
            1,
            SourceEffect::None,
        ),
        (
            MapSourceStepId::RawStateNullFile,
            MapSiteId::RawGate,
            SourceOwnerId::AbiRawState,
            "unsafe fn installed_envelope",
            "RawSqliteFileStateRejection::NullFile",
            1,
            SourceEffect::None,
        ),
        (
            MapSourceStepId::RawStateUninstalled,
            MapSiteId::RawGate,
            SourceOwnerId::AbiRawState,
            "fn validate_installed",
            "RawSqliteFileStateRejection::Uninstalled",
            1,
            SourceEffect::None,
        ),
        (
            MapSourceStepId::RawStateForeignMethodsNullTable,
            MapSiteId::RawGate,
            SourceOwnerId::AbiRawState,
            "fn validate_installed",
            "RawSqliteFileStateRejection::ForeignMethods",
            1,
            SourceEffect::None,
        ),
        (
            MapSourceStepId::RawStateForeignMethodsForeignTable,
            MapSiteId::RawGate,
            SourceOwnerId::AbiRawState,
            "fn validate_installed",
            "RawSqliteFileStateRejection::ForeignMethods",
            2,
            SourceEffect::None,
        ),
        (
            MapSourceStepId::RawStateMissing,
            MapSiteId::RawGate,
            SourceOwnerId::AbiRawState,
            "fn validate_installed",
            "RawSqliteFileStateRejection::StateMissing",
            1,
            SourceEffect::None,
        ),
        (
            MapSourceStepId::RawStateTypeMismatch,
            MapSiteId::RawGate,
            SourceOwnerId::AbiRawState,
            "unsafe fn with_installed_state",
            "RawSqliteFileStateRejection::TypeMismatch",
            1,
            SourceEffect::None,
        ),
        (
            MapSourceStepId::RawStateCaughtPanic,
            MapSiteId::RawGate,
            SourceOwnerId::AbiFileState,
            "unsafe fn run_code",
            "| Err(_) =>",
            1,
            SourceEffect::None,
        ),
        (
            MapSourceStepId::RawAbandonEmpty,
            MapSiteId::RawAbandon,
            SourceOwnerId::AbiRawState,
            "unsafe fn abandon_installed_state",
            "return Ok(false);",
            1,
            SourceEffect::None,
        ),
        (
            MapSourceStepId::RawAbandonInstalled,
            MapSiteId::RawAbandon,
            SourceOwnerId::AbiRawState,
            "unsafe fn abandon_installed_state",
            "drop(Box::from_raw",
            1,
            SourceEffect::Cleanup,
        ),
        (
            MapSourceStepId::RawAbandonNullFileRejected,
            MapSiteId::RawAbandon,
            SourceOwnerId::AbiRawState,
            "unsafe fn abandon_installed_state",
            "RawSqliteFileStateRejection::NullFile",
            1,
            SourceEffect::None,
        ),
        (
            MapSourceStepId::RawAbandonForeignMethodsNullTableRejected,
            MapSiteId::RawAbandon,
            SourceOwnerId::AbiRawState,
            "fn validate_installed",
            "RawSqliteFileStateRejection::ForeignMethods",
            1,
            SourceEffect::None,
        ),
        (
            MapSourceStepId::RawAbandonForeignMethodsForeignTableRejected,
            MapSiteId::RawAbandon,
            SourceOwnerId::AbiRawState,
            "fn validate_installed",
            "RawSqliteFileStateRejection::ForeignMethods",
            2,
            SourceEffect::None,
        ),
        (
            MapSourceStepId::RawAbandonStateMissingRejected,
            MapSiteId::RawAbandon,
            SourceOwnerId::AbiRawState,
            "fn validate_installed",
            "RawSqliteFileStateRejection::StateMissing",
            1,
            SourceEffect::None,
        ),
        (
            MapSourceStepId::RawFallbackProjection,
            MapSiteId::AbiProjection,
            SourceOwnerId::AbiFileState,
            "unsafe fn run_code",
            "fallback",
            2,
            SourceEffect::None,
        ),
    ] {
        let step = find_step(steps, id)?;
        if step.site != site
            || step.anchor != source_anchor(owner, symbol, needle, occurrence)
            || step.ops != MAP_BOTH
            || step.epoch != Epoch::AbiInput
            || step.effect != effect
            || step.value_flow != MapValueFlow::None
        {
            return Err("Map ABI/raw prefix source step changed its exact reviewed shape");
        }
    }
    Ok(())
}

fn validate_call_contexts(steps: &[MapSourceStep]) -> Result<(), &'static str> {
    for id in [
        MapSourceStepId::RawStateAccepted,
        MapSourceStepId::RawStateTypeMismatch,
        MapSourceStepId::RawAbandonEmpty,
        MapSourceStepId::RawAbandonInstalled,
        MapSourceStepId::RawAbandonNullFileRejected,
    ] {
        if find_step(steps, id)?.call_context.is_some() {
            return Err("Map raw primary source step gained an unreviewed caller context");
        }
    }

    let raw_borrow = source_anchor(
        SourceOwnerId::AbiRawState,
        "unsafe fn with_installed_state",
        "installed_envelope(file)?",
        1,
    );
    require_contexts(
        steps,
        &[
            MapSourceStepId::RawStateNullFile,
            MapSourceStepId::RawStateUninstalled,
            MapSourceStepId::RawStateForeignMethodsNullTable,
            MapSourceStepId::RawStateForeignMethodsForeignTable,
            MapSourceStepId::RawStateMissing,
        ],
        raw_borrow,
    )?;

    let raw_abandon = source_anchor(
        SourceOwnerId::AbiRawState,
        "unsafe fn abandon_installed_state",
        "validate_installed(methods, state)?;",
        1,
    );
    require_contexts(
        steps,
        &[
            MapSourceStepId::RawAbandonForeignMethodsNullTableRejected,
            MapSourceStepId::RawAbandonForeignMethodsForeignTableRejected,
            MapSourceStepId::RawAbandonStateMissingRejected,
        ],
        raw_abandon,
    )?;

    let map_run_code = source_anchor(
        SourceOwnerId::AbiIoShm,
        "unsafe extern \"C\" fn map",
        "file_state::run_code(",
        1,
    );
    require_contexts(
        steps,
        &[
            MapSourceStepId::RawStateCaughtPanic,
            MapSourceStepId::RawFallbackProjection,
        ],
        map_run_code,
    )
}

fn require_contexts(
    steps: &[MapSourceStep],
    ids: &[MapSourceStepId],
    expected: SourceAnchor,
) -> Result<(), &'static str> {
    for id in ids {
        if find_step(steps, *id)?.call_context != Some(expected) {
            return Err("Map raw shared source step has a non-exact caller context");
        }
    }
    Ok(())
}
