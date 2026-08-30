//! Sealed identity/state projection created only by real-boundary validation.

use anyhow::anyhow;
use rusqlite::ffi;

use super::super::a2b2_cases::{
    JointCloseCause, JointCloseFailureClass, JointCloseLogicalRoutePhase,
    JointCloseMainLockOffsetClass, JointCloseMainLockPrestate, JointClosePhase,
    JointCloseRegistryRoutePhase, JointCloseSelector, JointCloseSqliteOutcome, JointCloseTiming,
};

mod validate;
pub(super) use validate::{seal, BoundaryEvidence};

#[derive(Debug, Clone, Copy)]
pub(super) struct SealedJointCloseBoundary {
    selector: JointCloseSelector,
    phase: JointClosePhase,
    cause: JointCloseCause,
    timing: JointCloseTiming,
    class: JointCloseFailureClass,
    variant: u8,
    main_lock_prestate: JointCloseMainLockPrestate,
    main_lock_offset_class: JointCloseMainLockOffsetClass,
    sqlite_outcome: JointCloseSqliteOutcome,
    mutation_may_have_occurred: bool,
    lock_outcome_uncertain: bool,
    domain_terminal: bool,
    registry_route_phase: JointCloseRegistryRoutePhase,
    logical_route_phase: JointCloseLogicalRoutePhase,
    later_callback_allowed: bool,
}

#[derive(Debug, Clone, Copy)]
struct BoundaryProjection {
    phase: JointClosePhase,
    cause: JointCloseCause,
    timing: JointCloseTiming,
    class: JointCloseFailureClass,
    variant: u8,
    main_lock_prestate: JointCloseMainLockPrestate,
    main_lock_offset_class: JointCloseMainLockOffsetClass,
    mutation_may_have_occurred: bool,
    lock_outcome_uncertain: bool,
    domain_terminal: bool,
    registry_route_phase: JointCloseRegistryRoutePhase,
    logical_route_phase: JointCloseLogicalRoutePhase,
    later_callback_allowed: bool,
}

impl SealedJointCloseBoundary {
    fn new(
        selector: JointCloseSelector,
        code: i32,
        projection: BoundaryProjection,
    ) -> anyhow::Result<Self> {
        let sqlite_outcome = match code {
            ffi::SQLITE_OK => JointCloseSqliteOutcome::Ok,
            ffi::SQLITE_IOERR_CLOSE => JointCloseSqliteOutcome::IoerrClose,
            _ => {
                return Err(anyhow!(
                    "JointClose sealed boundary received a noncanonical real xClose result"
                ))
            }
        };
        if (projection.phase == JointClosePhase::Success) != (code == ffi::SQLITE_OK) {
            return Err(anyhow!(
                "JointClose real xClose result disagrees with its sealed physical phase"
            ));
        }
        Ok(Self {
            selector,
            phase: projection.phase,
            cause: projection.cause,
            timing: projection.timing,
            class: projection.class,
            variant: projection.variant,
            main_lock_prestate: projection.main_lock_prestate,
            main_lock_offset_class: projection.main_lock_offset_class,
            sqlite_outcome,
            mutation_may_have_occurred: projection.mutation_may_have_occurred,
            lock_outcome_uncertain: projection.lock_outcome_uncertain,
            domain_terminal: projection.domain_terminal,
            registry_route_phase: projection.registry_route_phase,
            logical_route_phase: projection.logical_route_phase,
            later_callback_allowed: projection.later_callback_allowed,
        })
    }

    pub(super) fn selector(self) -> JointCloseSelector {
        self.selector
    }
    pub(super) fn phase(self) -> JointClosePhase {
        self.phase
    }
    pub(super) fn cause(self) -> JointCloseCause {
        self.cause
    }
    pub(super) fn timing(self) -> JointCloseTiming {
        self.timing
    }
    pub(super) fn class(self) -> JointCloseFailureClass {
        self.class
    }
    pub(super) fn variant(self) -> u8 {
        self.variant
    }
    pub(super) fn main_lock_prestate(self) -> JointCloseMainLockPrestate {
        self.main_lock_prestate
    }
    pub(super) fn main_lock_offset_class(self) -> JointCloseMainLockOffsetClass {
        self.main_lock_offset_class
    }
    pub(super) fn sqlite_outcome(self) -> JointCloseSqliteOutcome {
        self.sqlite_outcome
    }
    pub(super) fn mutation_may_have_occurred(self) -> bool {
        self.mutation_may_have_occurred
    }
    pub(super) fn lock_outcome_uncertain(self) -> bool {
        self.lock_outcome_uncertain
    }
    pub(super) fn domain_terminal(self) -> bool {
        self.domain_terminal
    }
    pub(super) fn registry_route_phase(self) -> JointCloseRegistryRoutePhase {
        self.registry_route_phase
    }
    pub(super) fn logical_route_phase(self) -> JointCloseLogicalRoutePhase {
        self.logical_route_phase
    }
    pub(super) fn later_callback_allowed(self) -> bool {
        self.later_callback_allowed
    }
}
