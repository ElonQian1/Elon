//! Source-only admission contracts for all 88 q15 initialization truncate-release members.

use super::super::super::terminal_descriptor::{InitializationFaultSiteV1, InitializationPathV1};
use super::super::runner_admission::{
    compile_for_test,
    native_acquire_existing_first_truncate_error_release_succeeded_catalog_row_count_for_test,
    validate_lock_program_for_test,
};
use super::lock_native_acquire_existing_first_truncate_error_release_succeeded_cases::{
    frozen_lock_existing_first_truncate_error_release_succeeded_leaves_v1,
    lock_existing_first_truncate_error_release_succeeded_descriptor_v1,
    FrozenLockExistingFirstTruncateErrorReleaseSucceededCaseV1,
    LOCK_EXISTING_FIRST_TRUNCATE_ERROR_RELEASE_SUCCEEDED_MEMBER_COUNT,
};
use super::lock_native_acquire_created_first_truncate_error_release_succeeded_cases::frozen_lock_created_first_truncate_error_release_succeeded_leaves_v1;
use super::lock_native_acquire_existing_first_exclusive_release_error_cases::frozen_lock_existing_first_exclusive_release_error_leaves_v1;
use super::*;

fn supported_key_and_member(
    case: FrozenLockExistingFirstTruncateErrorReleaseSucceededCaseV1,
) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let leaf = &frozen_lock_existing_first_truncate_error_release_succeeded_leaves_v1()[&case];
    let validated = project_validated_dynamic_terminal_v1(&leaf.record, &leaf.descriptor).unwrap();
    assert_eq!(validated.descriptor_binding.member, leaf.member);
    let mut key = validated.semantic_key;
    key.recipe.capability = RunnerCapabilityV1::Supported;
    (key, leaf.member)
}

fn assert_rejected(key: DynamicClassKeyV1, member: StaticMemberSealV1, mutation: &str) {
    assert!(
        validate_lock_program_for_test(&key, member, compile_for_test(&key)).is_err(),
        "q15 initialization admission accepted {mutation}"
    );
}

#[test]
fn all_88_q15_descriptors_and_exact_catalog_seals_are_source_present() {
    let leaves = frozen_lock_existing_first_truncate_error_release_succeeded_leaves_v1();
    assert_eq!(
        leaves.len(),
        LOCK_EXISTING_FIRST_TRUNCATE_ERROR_RELEASE_SUCCEEDED_MEMBER_COUNT
    );
    assert_eq!(
        native_acquire_existing_first_truncate_error_release_succeeded_catalog_row_count_for_test(),
        LOCK_EXISTING_FIRST_TRUNCATE_ERROR_RELEASE_SUCCEEDED_MEMBER_COUNT
    );
    for &case in leaves.keys() {
        let (key, member) = supported_key_and_member(case);
        validate_lock_program_for_test(&key, member, compile_for_test(&key))
            .unwrap_or_else(|error| panic!("exact q15 member {case:?} was rejected: {error:?}"));
    }
}

#[test]
fn q15_is_inventory_present_without_granting_supported() {
    for (&case, leaf) in frozen_lock_existing_first_truncate_error_release_succeeded_leaves_v1() {
        let prepared = prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor).unwrap();
        let receipt = super::super::runner_admission::inventory_v1(&prepared.key).unwrap();
        assert!(matches!(
            receipt.status(),
            super::super::runner_admission::ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. }
        ));
        assert_eq!(
            project_dynamic_class_v1(
                &leaf.record,
                &lock_existing_first_truncate_error_release_succeeded_descriptor_v1(
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
fn q15_keys_reject_sibling_seals_unlock_actions_and_nearby_initialization_shapes() {
    let leaves = frozen_lock_existing_first_truncate_error_release_succeeded_leaves_v1();
    let (&case, leaf) = leaves.first_key_value().unwrap();
    let (key, member) = supported_key_and_member(case);
    let sibling = leaves
        .values()
        .find(|candidate| candidate.member != leaf.member)
        .unwrap()
        .member;
    assert_rejected(key, sibling, "a sibling frozen seal");
    let q13_seal = frozen_lock_existing_first_exclusive_release_error_leaves_v1()
        .first_key_value()
        .unwrap()
        .1
        .member;
    assert_rejected(key, q13_seal, "a q13 neighboring-family seal");
    let q14_seal = frozen_lock_created_first_truncate_error_release_succeeded_leaves_v1()
        .first_key_value()
        .unwrap()
        .1
        .member;
    assert_rejected(key, q14_seal, "a q14 created-first neighboring-family seal");

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
    assert_rejected(fault, member, "the adjacent exclusive-release fault site");

    let mut path = key;
    let StimulusV1::Initialization(stimulus) = &mut path.stimulus else {
        unreachable!()
    };
    stimulus.path = InitializationPathV1::CreatedFirst;
    assert_rejected(path, member, "the created-first path");

    let mut phase = key;
    phase.phase = PhaseV1::DmsExclusiveRelease;
    assert_rejected(phase, member, "the adjacent exclusive-release failure");

    let mut completion = key;
    let DynamicAxesV1::Lock(axes) = &mut completion.axes else {
        unreachable!()
    };
    axes.completion = ReachabilityV1::Reached(LockCompletionV1::RouteUnknown);
    assert_rejected(completion, member, "a non-retention completion");

    let mut expected = key;
    expected.expected.dms_lock = DmsLockCustodyV1::ExclusiveOutcomeUncertain;
    assert_rejected(expected, member, "uncertain DMS custody");
}
