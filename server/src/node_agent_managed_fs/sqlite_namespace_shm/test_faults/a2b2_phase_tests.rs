use std::num::NonZeroU64;

use super::{
    super::types::{ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase},
    controller::{ManagedSqliteShmTestFaultController, ManagedSqliteShmTestFaultTarget},
};

const A2B2_PHASES: [ManagedSqliteShmFailurePhase; 3] = [
    ManagedSqliteShmFailurePhase::Barrier,
    ManagedSqliteShmFailurePhase::ConnectionDetach,
    ManagedSqliteShmFailurePhase::ExactSiblingDelete,
];

fn target() -> ManagedSqliteShmTestFaultTarget {
    ManagedSqliteShmTestFaultTarget::new(NonZeroU64::new(29).expect("non-zero A2b2 generation"), 31)
}

#[test]
fn every_a2b2_phase_accepts_an_exact_before_selector() {
    for phase in A2B2_PHASES {
        let exact = target();
        let mut faults = ManagedSqliteShmTestFaultController::default();
        faults
            .install(exact, &[(phase, 1)], &[])
            .expect("install A2b2 before-call phase");
        let matched = faults
            .observe(exact, phase)
            .expect("observe A2b2 before-call phase")
            .expect("match A2b2 before-call phase");
        let failure = faults
            .activate_before(matched)
            .expect("activate A2b2 before-call phase")
            .into_before_failure(false);
        assert_eq!(
            failure.class(),
            ManagedSqliteShmFailureClass::IoBeforeMutation
        );
    }
}

#[test]
fn barrier_after_success_is_terminal_without_claiming_mutation() {
    let exact = target();
    let mut faults = ManagedSqliteShmTestFaultController::default();
    faults
        .install(
            exact,
            &[],
            &[(
                ManagedSqliteShmFailurePhase::Barrier,
                1,
                ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned,
            )],
        )
        .expect("install barrier after-success phase");
    let matched = faults
        .observe(exact, ManagedSqliteShmFailurePhase::Barrier)
        .expect("observe barrier after-success phase")
        .expect("match barrier after-success phase");
    let failure = faults
        .activate_after(matched, false)
        .expect("activate mutation-free barrier completion")
        .into_after_failure(false);
    assert_eq!(
        failure.class(),
        ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned
    );
    assert!(!failure.mutation_may_have_occurred());
    assert!(!failure.lock_outcome_uncertain());

    assert!(ManagedSqliteShmTestFaultController::default()
        .install(
            exact,
            &[],
            &[(
                ManagedSqliteShmFailurePhase::Barrier,
                1,
                ManagedSqliteShmFailureClass::MutatedButKnown,
            )],
        )
        .is_err());
}

#[test]
fn detach_and_delete_after_success_require_known_mutation() {
    for phase in [
        ManagedSqliteShmFailurePhase::ConnectionDetach,
        ManagedSqliteShmFailurePhase::ExactSiblingDelete,
    ] {
        let exact = target();
        let mut faults = ManagedSqliteShmTestFaultController::default();
        faults
            .install(
                exact,
                &[],
                &[(phase, 1, ManagedSqliteShmFailureClass::MutatedButKnown)],
            )
            .expect("install mutating A2b2 after-success phase");
        let matched = faults
            .observe(exact, phase)
            .expect("observe mutating A2b2 after-success phase")
            .expect("match mutating A2b2 after-success phase");
        assert!(faults.activate_after(matched, false).is_err());
        let triggered = faults
            .activate_after(matched, true)
            .expect("activate known A2b2 mutation");
        let failure = triggered.into_after_failure(true);
        assert_eq!(
            failure.class(),
            ManagedSqliteShmFailureClass::MutatedButKnown
        );
        assert!(failure.mutation_may_have_occurred());
    }
}
