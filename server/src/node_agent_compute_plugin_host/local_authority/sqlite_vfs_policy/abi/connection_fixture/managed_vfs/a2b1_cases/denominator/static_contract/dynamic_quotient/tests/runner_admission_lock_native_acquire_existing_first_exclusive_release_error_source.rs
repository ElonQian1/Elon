//! Source-only admission contracts for all 88 q13 initialization-release members.

use super::super::runner_admission::{
    compile_for_test,
    native_acquire_existing_first_exclusive_release_error_catalog_row_count_for_test,
    validate_lock_program_for_test,
};
use super::lock_native_acquire_existing_first_exclusive_release_error_cases::{
    frozen_lock_existing_first_exclusive_release_error_leaves_v1,
    lock_existing_first_exclusive_release_error_descriptor_v1,
    FrozenLockExistingFirstExclusiveReleaseErrorCaseV1,
    LOCK_EXISTING_FIRST_EXCLUSIVE_RELEASE_ERROR_MEMBER_COUNT,
};
use super::lock_native_acquire_created_first_exclusive_release_error_cases::
    frozen_lock_created_first_exclusive_release_error_leaves_v1;
use super::*;

fn supported_key_and_member(
    case: FrozenLockExistingFirstExclusiveReleaseErrorCaseV1,
) -> (DynamicClassKeyV1, StaticMemberSealV1) {
    let leaf = &frozen_lock_existing_first_exclusive_release_error_leaves_v1()[&case];
    let validated = project_validated_dynamic_terminal_v1(&leaf.record, &leaf.descriptor).unwrap();
    assert_eq!(validated.descriptor_binding.member, leaf.member);
    let mut key = validated.semantic_key;
    key.recipe.capability = RunnerCapabilityV1::Supported;
    (key, leaf.member)
}

fn assert_rejected(key: DynamicClassKeyV1, member: StaticMemberSealV1, mutation: &str) {
    assert!(
        validate_lock_program_for_test(&key, member, compile_for_test(&key)).is_err(),
        "q13 initialization admission accepted {mutation}"
    );
}

#[test]
fn all_88_q13_descriptors_and_exact_catalog_seals_are_source_present() {
    let leaves = frozen_lock_existing_first_exclusive_release_error_leaves_v1();
    assert_eq!(
        leaves.len(),
        LOCK_EXISTING_FIRST_EXCLUSIVE_RELEASE_ERROR_MEMBER_COUNT
    );
    assert_eq!(
        native_acquire_existing_first_exclusive_release_error_catalog_row_count_for_test(),
        LOCK_EXISTING_FIRST_EXCLUSIVE_RELEASE_ERROR_MEMBER_COUNT
    );
    for &case in leaves.keys() {
        let (key, member) = supported_key_and_member(case);
        validate_lock_program_for_test(&key, member, compile_for_test(&key))
            .unwrap_or_else(|error| panic!("exact q13 member {case:?} was rejected: {error:?}"));
    }
}

#[test]
fn q13_is_inventory_present_without_granting_supported() {
    for (&case, leaf) in frozen_lock_existing_first_exclusive_release_error_leaves_v1() {
        let prepared = prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor).unwrap();
        let receipt = super::super::runner_admission::inventory_v1(&prepared.key).unwrap();
        assert!(matches!(
            receipt.status(),
            super::super::runner_admission::ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. }
        ));
        assert_eq!(
            project_dynamic_class_v1(
                &leaf.record,
                &lock_existing_first_exclusive_release_error_descriptor_v1(
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
fn q13_keys_reject_sibling_seals_unlock_actions_and_nearby_initialization_shapes() {
    let leaves = frozen_lock_existing_first_exclusive_release_error_leaves_v1();
    let (&case, leaf) = leaves.first_key_value().unwrap();
    let (key, member) = supported_key_and_member(case);
    let sibling = leaves
        .values()
        .find(|candidate| candidate.member != leaf.member)
        .unwrap()
        .member;
    assert_rejected(key, sibling, "a sibling frozen seal");
    let q12_seal = frozen_lock_created_first_exclusive_release_error_leaves_v1()
        .first_key_value()
        .unwrap()
        .1
        .member;
    assert_rejected(key, q12_seal, "a q12 created-first seal");

    let mut neighbor = key;
    let StimulusV1::Initialization(stimulus) = &mut neighbor.stimulus else {
        unreachable!()
    };
    stimulus.path = super::super::super::terminal_descriptor::InitializationPathV1::CreatedFirst;
    assert_rejected(neighbor, member, "the q12 created-first neighbor family");

    let mut unlock = key;
    let DynamicAxesV1::Lock(axes) = &mut unlock.axes else {
        unreachable!()
    };
    axes.action = ReachabilityV1::Reached(LockActionV1::UnlockExclusive);
    assert_rejected(unlock, member, "an unlock action");

    let mut wrong_mask = key;
    let DynamicAxesV1::Lock(axes) = &mut wrong_mask.axes else {
        unreachable!()
    };
    axes.mask = ReachabilityV1::Reached(case.mask ^ 0xff);
    assert_rejected(wrong_mask, member, "an action/range mask mismatch");

    let mut profile = key;
    let DynamicAxesV1::Lock(axes) = &mut profile.axes else {
        unreachable!()
    };
    axes.initialization = ReachabilityV1::Reached(InitializationProfileV1::ExistingFirstShared);
    assert_rejected(profile, member, "a post-initialization profile");

    let mut phase = key;
    phase.phase = PhaseV1::DmsTruncate;
    assert_rejected(phase, member, "the adjacent truncate failure");

    let mut completion = key;
    let DynamicAxesV1::Lock(axes) = &mut completion.axes else {
        unreachable!()
    };
    axes.completion = ReachabilityV1::Reached(LockCompletionV1::RouteUnknown);
    assert_rejected(completion, member, "a non-retention completion");

    let mut expected = key;
    expected.expected.dms_lock = DmsLockCustodyV1::Released;
    assert_rejected(expected, member, "known DMS custody");
}
