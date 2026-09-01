//! Source-only admission contracts for all 88 q19 existing-first shared-busy close-ok members.

use super::super::super::terminal_descriptor::{InitializationFaultSiteV1, InitializationPathV1};
use super::super::runner_admission::{
    compile_for_test,
    native_acquire_existing_first_shared_busy_close_succeeded_catalog_row_count_for_test,
    validate_lock_program_for_test,
};
use super::lock_native_acquire_busy_cases::frozen_lock_native_acquire_busy_leaves_v1;
use super::lock_native_acquire_created_first_shared_busy_close_succeeded_cases::frozen_lock_created_first_shared_busy_close_succeeded_leaves_v1;
use super::lock_native_acquire_existing_first_shared_busy_close_succeeded_cases::{
    frozen_lock_existing_first_shared_busy_close_succeeded_leaves_v1,
    lock_existing_first_shared_busy_close_succeeded_descriptor_v1,
    FrozenLockExistingFirstSharedBusyCloseSucceededCaseV1,
    LOCK_EXISTING_FIRST_SHARED_BUSY_CLOSE_SUCCEEDED_MEMBER_COUNT,
};
use super::lock_native_acquire_existing_first_truncate_error_release_failed_cases::frozen_lock_existing_first_truncate_error_release_failed_leaves_v1;
use super::*;

fn supported_key_and_member(
    case: FrozenLockExistingFirstSharedBusyCloseSucceededCaseV1,
) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let leaf = &frozen_lock_existing_first_shared_busy_close_succeeded_leaves_v1()[&case];
    let validated = project_validated_dynamic_terminal_v1(&leaf.record, &leaf.descriptor).unwrap();
    assert_eq!(validated.descriptor_binding.member, leaf.member);
    let mut key = validated.semantic_key;
    key.recipe.capability = RunnerCapabilityV1::Supported;
    (key, leaf.member)
}

fn assert_rejected(key: DynamicClassKeyV1, member: StaticMemberSealV1, mutation: &str) {
    assert!(
        validate_lock_program_for_test(&key, member, compile_for_test(&key)).is_err(),
        "q19 initialization admission accepted {mutation}"
    );
}

#[test]
fn all_88_q19_descriptors_and_exact_catalog_seals_are_source_present() {
    let leaves = frozen_lock_existing_first_shared_busy_close_succeeded_leaves_v1();
    assert_eq!(
        leaves.len(),
        LOCK_EXISTING_FIRST_SHARED_BUSY_CLOSE_SUCCEEDED_MEMBER_COUNT
    );
    assert_eq!(
        native_acquire_existing_first_shared_busy_close_succeeded_catalog_row_count_for_test(),
        LOCK_EXISTING_FIRST_SHARED_BUSY_CLOSE_SUCCEEDED_MEMBER_COUNT
    );
    for &case in leaves.keys() {
        let (key, member) = supported_key_and_member(case);
        validate_lock_program_for_test(&key, member, compile_for_test(&key))
            .unwrap_or_else(|error| panic!("exact q19 member {case:?} was rejected: {error:?}"));
    }
}

#[test]
fn q19_is_inventory_present_without_granting_supported() {
    for (&case, leaf) in frozen_lock_existing_first_shared_busy_close_succeeded_leaves_v1() {
        let prepared = prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor).unwrap();
        let receipt = super::super::runner_admission::inventory_v1(&prepared.key).unwrap();
        assert!(matches!(
            receipt.status(),
            super::super::runner_admission::ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. }
        ));
        assert_eq!(
            project_dynamic_class_v1(
                &leaf.record,
                &lock_existing_first_shared_busy_close_succeeded_descriptor_v1(
                    case,
                    RunnerCapabilityV1::Supported,
                ),
            ),
            Err(ProjectionErrorV1::Invalid(
                ProjectionViolationV1::RunnerAdmissionUnsealedSupported,
            )),
        );
    }
}

#[test]
fn q19_rejects_neighboring_seals_and_every_typed_contract_mutation() {
    let leaves = frozen_lock_existing_first_shared_busy_close_succeeded_leaves_v1();
    let (&case, leaf) = leaves.first_key_value().unwrap();
    let (key, member) = supported_key_and_member(case);
    let sibling = leaves
        .values()
        .find(|candidate| candidate.member != leaf.member)
        .unwrap()
        .member;
    assert_rejected(key, sibling, "a sibling frozen seal");
    for (neighbor, label) in [
        (
            frozen_lock_native_acquire_busy_leaves_v1()
                .first_key_value()
                .unwrap()
                .1
                .member,
            "a warm native-busy seal",
        ),
        (
            frozen_lock_created_first_shared_busy_close_succeeded_leaves_v1()
                .first_key_value()
                .unwrap()
                .1
                .member,
            "the q18 created-first shared-busy seal",
        ),
        (
            frozen_lock_existing_first_truncate_error_release_failed_leaves_v1()
                .first_key_value()
                .unwrap()
                .1
                .member,
            "a q17 existing-first cleanup seal",
        ),
    ] {
        assert_rejected(key, neighbor, label);
    }

    let mut unlock = key;
    let DynamicAxesV1::Lock(axes) = &mut unlock.axes else {
        unreachable!()
    };
    axes.action = ReachabilityV1::Reached(LockActionV1::UnlockExclusive);
    assert_rejected(unlock, member, "an unlock action");

    let mut mask = key;
    let DynamicAxesV1::Lock(axes) = &mut mask.axes else {
        unreachable!()
    };
    axes.mask = ReachabilityV1::Reached(0);
    assert_rejected(mask, member, "a mismatched request mask");

    let mut profile = key;
    let DynamicAxesV1::Lock(axes) = &mut profile.axes else {
        unreachable!()
    };
    axes.initialization = ReachabilityV1::Reached(InitializationProfileV1::ExistingFirstShared);
    assert_rejected(profile, member, "a post-initialization profile");

    let mut fault = key;
    let StimulusV1::Initialization(stimulus) = &mut fault.stimulus else {
        unreachable!()
    };
    stimulus.fault_site = InitializationFaultSiteV1::DmsExclusiveRelease;
    assert_rejected(fault, member, "a different DMS fault site");

    let mut cleanup_rewrite = key;
    let StimulusV1::Initialization(stimulus) = &mut cleanup_rewrite.stimulus else {
        unreachable!()
    };
    stimulus.cleanup_rewrite = true;
    assert_rejected(cleanup_rewrite, member, "cleanup_rewrite=true");

    let mut path = key;
    let StimulusV1::Initialization(stimulus) = &mut path.stimulus else {
        unreachable!()
    };
    stimulus.path = InitializationPathV1::CreatedFirst;
    assert_rejected(path, member, "the created-first path");

    let mut phase = key;
    phase.phase = PhaseV1::DmsExclusiveRelease;
    assert_rejected(phase, member, "a different DMS phase");

    let mut timing = key;
    timing.timing = TimingV1::Cleanup;
    assert_rejected(timing, member, "cleanup timing instead of at-call");

    let mut cleanup = key;
    cleanup.recipe.cleanup = CleanupV1::ParentOwnedRoot;
    assert_rejected(cleanup, member, "a non-retention cleanup recipe");

    let mut completion = key;
    let DynamicAxesV1::Lock(axes) = &mut completion.axes else {
        unreachable!()
    };
    axes.completion = ReachabilityV1::Reached(LockCompletionV1::RouteUnknown);
    assert_rejected(completion, member, "a non-retention completion");

    let mut disposition = key;
    disposition.expected.disposition = TerminalDispositionV1::CleanupRewritten;
    assert_rejected(disposition, member, "a cleanup-rewritten disposition");

    let mut failure = key;
    failure.expected.failure = FailureClassV1::OutcomeUncertainPoisoned;
    assert_rejected(failure, member, "an uncertain failure class");

    let mut mutation = key;
    mutation.expected.mutation = MutationStateV1::Uncertain;
    assert_rejected(mutation, member, "an uncertain mutation");

    let mut lock_uncertainty = key;
    lock_uncertainty.expected.lock_outcome_uncertain = true;
    assert_rejected(lock_uncertainty, member, "an uncertain lock outcome");

    let mut dms = key;
    dms.expected.dms_lock = DmsLockCustodyV1::ExclusiveOutcomeUncertain;
    assert_rejected(dms, member, "unreleased DMS custody");

    let mut file = key;
    file.expected.file = CustodyStateV1::Retained;
    assert_rejected(file, member, "retained file custody despite close success");

    let mut counts = key;
    counts.expected.counts.native_lock = 1;
    assert_rejected(counts, member, "one instead of two native lock attempts");

    let mut unlock_count = key;
    unlock_count.expected.counts.native_unlock = 0;
    assert_rejected(unlock_count, member, "zero instead of one native unlock");
}
