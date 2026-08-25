use super::super::super::super::model::{Epoch, SourceEffect, SourceOwnerId};
use super::super::super::model::*;

const fn pending_state_invariant() -> MapStepKind {
    MapStepKind::Pending {
        terminal: Some(MapTerminalTemplate {
            cause: MapPhase::RequestValidation,
            returned_terminal: MapPhase::RequestValidation,
            stored_poison: MapPhaseProjection::Pending,
            route_marker: MapPhaseProjection::Pending,
            timing: MapTiming::Validation,
            exit: MapExit::AbiUnavailableNull,
            retention: MapRetention::PrefixDependent,
            multiplicity: MapMultiplicity::SymbolicPhaseOccurrence,
        }),
        reason: MapPendingReason::ManagedStateInvariant,
    }
}

pub(super) const STEPS: &[MapSourceStep] = &[
    step(
        MapSourceStepId::FirstProcessInitialized,
        MapSiteId::NodeInitialization,
        SourceOwnerId::ManagedInitialization,
        "fn open_node",
        "Ok(PlatformManagedSqliteLockAttempt::Acquired) =>",
        1,
        MAP_BOTH,
        Epoch::FirstMapBootstrap,
        SourceEffect::None,
        MapStepKind::Continuation,
    ),
    step(
        MapSourceStepId::SharedDmsInitialized,
        MapSiteId::NodeInitialization,
        SourceOwnerId::ManagedInitialization,
        "fn open_node",
        "Ok(PlatformManagedSqliteLockAttempt::Acquired) =>",
        2,
        MAP_BOTH,
        Epoch::FirstMapBootstrap,
        SourceEffect::CustodyMutation,
        MapStepKind::Continuation,
    ),
    step(
        MapSourceStepId::NodeMissingAfterOpen,
        MapSiteId::NodeInitialization,
        SourceOwnerId::ManagedInitialization,
        "fn ensure_node",
        "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_AFTER_OPEN",
        1,
        MAP_BOTH,
        Epoch::FirstMapBootstrap,
        SourceEffect::Poison,
        pending_state_invariant(),
    ),
];
