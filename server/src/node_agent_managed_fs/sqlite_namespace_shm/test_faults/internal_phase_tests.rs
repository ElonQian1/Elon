use std::num::NonZeroU64;

use super::{
    super::types::{ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase},
    controller::{ManagedSqliteShmTestFaultController, ManagedSqliteShmTestFaultTarget},
};

const INTERNAL_PHASES: [ManagedSqliteShmFailurePhase; 11] = [
    ManagedSqliteShmFailurePhase::ExactSiblingOpen,
    ManagedSqliteShmFailurePhase::DmsExclusiveAcquire,
    ManagedSqliteShmFailurePhase::DmsTruncate,
    ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
    ManagedSqliteShmFailurePhase::DmsSharedAcquire,
    ManagedSqliteShmFailurePhase::FileSize,
    ManagedSqliteShmFailurePhase::FileGrow,
    ManagedSqliteShmFailurePhase::MappingCreate,
    ManagedSqliteShmFailurePhase::ViewMap,
    ManagedSqliteShmFailurePhase::LockAcquire,
    ManagedSqliteShmFailurePhase::LockRelease,
];

const MUTATING_INTERNAL_PHASES: [ManagedSqliteShmFailurePhase; 10] = [
    ManagedSqliteShmFailurePhase::ExactSiblingOpen,
    ManagedSqliteShmFailurePhase::DmsExclusiveAcquire,
    ManagedSqliteShmFailurePhase::DmsTruncate,
    ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
    ManagedSqliteShmFailurePhase::DmsSharedAcquire,
    ManagedSqliteShmFailurePhase::FileGrow,
    ManagedSqliteShmFailurePhase::MappingCreate,
    ManagedSqliteShmFailurePhase::ViewMap,
    ManagedSqliteShmFailurePhase::LockAcquire,
    ManagedSqliteShmFailurePhase::LockRelease,
];

fn target() -> ManagedSqliteShmTestFaultTarget {
    ManagedSqliteShmTestFaultTarget::new(
        NonZeroU64::new(17).expect("non-zero internal phase generation"),
        23,
    )
}

#[test]
fn every_internal_phase_accepts_an_exact_before_selector() {
    for phase in INTERNAL_PHASES {
        let exact = target();
        let mut before = ManagedSqliteShmTestFaultController::default();
        before
            .install(exact, &[(phase, 1)], &[])
            .expect("install internal before-call phase");
        let matched = before
            .observe(exact, phase)
            .expect("observe internal before-call phase")
            .expect("match internal before-call phase");
        assert!(matched.is_before_call());
    }
}

#[test]
fn every_mutating_internal_phase_accepts_an_exact_after_selector() {
    for phase in MUTATING_INTERNAL_PHASES {
        let exact = target();
        let mut after = ManagedSqliteShmTestFaultController::default();
        after
            .install(
                exact,
                &[],
                &[(phase, 1, ManagedSqliteShmFailureClass::MutatedButKnown)],
            )
            .expect("install internal after-success phase");
        let matched = after
            .observe(exact, phase)
            .expect("observe internal after-success phase")
            .expect("match internal after-success phase");
        assert!(!matched.is_before_call());
    }
}

#[test]
fn file_size_rejects_after_success_instead_of_claiming_mutation() {
    let mut faults = ManagedSqliteShmTestFaultController::default();
    assert_eq!(
        faults.install(
            target(),
            &[],
            &[(
                ManagedSqliteShmFailurePhase::FileSize,
                1,
                ManagedSqliteShmFailureClass::MutatedButKnown,
            )],
        ),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_AFTER_PHASE_UNSUPPORTED")
    );
}

#[test]
fn uncertain_lock_phase_faults_report_uncertain_lock_custody() {
    for phase in [
        ManagedSqliteShmFailurePhase::DmsExclusiveAcquire,
        ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
        ManagedSqliteShmFailurePhase::DmsSharedAcquire,
        ManagedSqliteShmFailurePhase::LockAcquire,
        ManagedSqliteShmFailurePhase::LockRelease,
    ] {
        let exact = target();
        let mut faults = ManagedSqliteShmTestFaultController::default();
        faults
            .install(
                exact,
                &[],
                &[(
                    phase,
                    1,
                    ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned,
                )],
            )
            .expect("install uncertain lock phase");
        let matched = faults
            .observe(exact, phase)
            .expect("observe uncertain lock phase")
            .expect("match uncertain lock phase");
        let failure = faults
            .activate(matched, true)
            .expect("activate uncertain lock phase")
            .into_failure(true);
        assert!(failure.lock_outcome_uncertain());
    }
}
