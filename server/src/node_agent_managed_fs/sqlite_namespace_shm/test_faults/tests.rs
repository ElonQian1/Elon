use std::num::NonZeroU64;

use super::{
    super::types::{ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase},
    controller::{ManagedSqliteShmTestFaultController, ManagedSqliteShmTestFaultTarget},
};

fn target(generation: u64, connection_id: u64) -> ManagedSqliteShmTestFaultTarget {
    ManagedSqliteShmTestFaultTarget::new(
        NonZeroU64::new(generation).expect("non-zero test generation"),
        connection_id,
    )
}

#[test]
fn exact_connection_ordinals_and_whole_teardown_mutation_are_fenced() {
    let exact = target(7, 11);
    let mut faults = ManagedSqliteShmTestFaultController::default();
    faults
        .install(
            exact,
            &[
                (ManagedSqliteShmFailurePhase::ViewUnmap, 2),
                (ManagedSqliteShmFailurePhase::FileClose, 1),
            ],
            &[
                (
                    ManagedSqliteShmFailurePhase::MappingClose,
                    1,
                    ManagedSqliteShmFailureClass::MutatedButKnown,
                ),
                (
                    ManagedSqliteShmFailurePhase::DmsSharedRelease,
                    1,
                    ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned,
                ),
            ],
        )
        .expect("install exact-connection script");

    assert!(faults
        .observe(target(7, 12), ManagedSqliteShmFailurePhase::ViewUnmap)
        .expect("ignore sibling connection")
        .is_none());
    assert!(faults
        .observe(target(8, 11), ManagedSqliteShmFailurePhase::ViewUnmap)
        .expect("ignore another runtime generation")
        .is_none());
    // Controller observation alone does not perform an OS action. Therefore observing unmatched
    // ViewUnmap ordinal one does not make the later ordinal-two unit activation mutated.
    assert!(faults
        .observe(exact, ManagedSqliteShmFailurePhase::ViewUnmap)
        .expect("observe exact first view without running a platform action")
        .is_none());

    let mapping = faults
        .observe(exact, ManagedSqliteShmFailurePhase::MappingClose)
        .expect("observe mapping phase")
        .expect("match mapping ordinal one");
    assert!(!mapping.is_before_call());
    let mapping_failure = faults
        .activate_after(mapping, true)
        .expect("activate post-success mapping fault")
        .into_after_failure(true);
    assert_eq!(
        mapping_failure.class(),
        ManagedSqliteShmFailureClass::MutatedButKnown
    );

    let view = faults
        .observe(exact, ManagedSqliteShmFailurePhase::ViewUnmap)
        .expect("observe exact second view")
        .expect("match exact second view");
    assert!(view.is_before_call());
    let view_failure = faults
        .activate_before(view)
        .expect("activate mutation-free before-call fault")
        .into_before_failure(false);
    assert_eq!(
        view_failure.class(),
        ManagedSqliteShmFailureClass::IoBeforeMutation
    );
    assert!(!view_failure.mutation_may_have_occurred());

    let dms = faults
        .observe(exact, ManagedSqliteShmFailurePhase::DmsSharedRelease)
        .expect("observe DMS phase")
        .expect("match DMS ordinal one");
    let dms_failure = faults
        .activate_after(dms, true)
        .expect("activate post-success DMS fault")
        .into_after_failure(true);
    assert_eq!(
        dms_failure.class(),
        ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned
    );
    assert!(dms_failure.lock_outcome_uncertain());

    let file = faults
        .observe(exact, ManagedSqliteShmFailurePhase::FileClose)
        .expect("observe file close")
        .expect("match file close ordinal one");
    let file_failure = faults
        .activate_before(file)
        .expect("activate before-call with prior mutation")
        .into_before_failure(true);
    assert_eq!(
        file_failure.class(),
        ManagedSqliteShmFailureClass::MutatedButKnown
    );
    assert!(file_failure.mutation_may_have_occurred());
    assert_eq!(faults.pending_count(exact), 0);
    assert!(faults.was_triggered(exact, ManagedSqliteShmFailurePhase::ViewUnmap, 2));
}

#[test]
fn script_rejects_ambiguous_or_incompatible_steps() {
    let exact = target(9, 3);
    assert!(ManagedSqliteShmTestFaultController::default()
        .install(
            exact,
            &[(ManagedSqliteShmFailurePhase::FileClose, 1)],
            &[(
                ManagedSqliteShmFailurePhase::FileClose,
                1,
                ManagedSqliteShmFailureClass::MutatedButKnown,
            )],
        )
        .is_err());
    assert!(ManagedSqliteShmTestFaultController::default()
        .install(
            exact,
            &[(ManagedSqliteShmFailurePhase::RequestValidation, 1)],
            &[],
        )
        .is_err());
    assert!(ManagedSqliteShmTestFaultController::default()
        .install(
            exact,
            &[],
            &[(
                ManagedSqliteShmFailurePhase::ViewUnmap,
                1,
                ManagedSqliteShmFailureClass::IoBeforeMutation,
            )],
        )
        .is_err());
}

#[test]
fn after_success_cannot_activate_without_a_recorded_mutation() {
    let exact = target(10, 4);
    let mut faults = ManagedSqliteShmTestFaultController::default();
    faults
        .install(
            exact,
            &[],
            &[(
                ManagedSqliteShmFailurePhase::ViewUnmap,
                1,
                ManagedSqliteShmFailureClass::MutatedButKnown,
            )],
        )
        .expect("install after-success step");
    let matched = faults
        .observe(exact, ManagedSqliteShmFailurePhase::ViewUnmap)
        .expect("observe exact view")
        .expect("match exact view");
    assert!(faults.activate_after(matched, false).is_err());
    assert_eq!(faults.pending_count(exact), 1);
    assert!(!faults.was_triggered(exact, ManagedSqliteShmFailurePhase::ViewUnmap, 1));
}
