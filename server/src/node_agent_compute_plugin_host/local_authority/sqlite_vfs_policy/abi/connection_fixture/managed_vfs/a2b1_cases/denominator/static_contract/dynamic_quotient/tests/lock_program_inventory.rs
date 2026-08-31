//! Exact pre-manifest inventory checks for all source-present Lock programs.

use std::collections::BTreeSet;

use super::super::runner_admission::ExecutionProgramInventoryStatusV1;
use super::lock_native_acquire_busy_cases::{
    frozen_lock_native_acquire_busy_leaves_v1, lock_native_acquire_busy_descriptor_v1,
    LOCK_NATIVE_ACQUIRE_BUSY_MEMBER_COUNT,
};
use super::lock_stored_poison_cases::{
    frozen_lock_stored_poison_leaves_v1, lock_stored_poison_descriptor_v1,
    LOCK_STORED_POISON_MEMBER_COUNT,
};
use super::program_inventory::{
    lock_lifecycle_cases, lock_lifecycle_descriptor, lock_lifecycle_record,
    lock_request_validation_descriptor, lock_request_validation_record,
    LOCK_REQUEST_VALIDATION_PROGRAMS,
};
use super::*;

#[test]
fn exact_lock_request_validation_programs_are_inventoried_without_granting_supported() {
    let record = lock_request_validation_record();
    for (action, stimulus) in LOCK_REQUEST_VALIDATION_PROGRAMS {
        let descriptor = lock_request_validation_descriptor(
            action,
            stimulus,
            RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
        );
        let prepared = prepare_dynamic_terminal_v1(&record, &descriptor).unwrap();
        let receipt = super::super::runner_admission::inventory_v1(&prepared.key).unwrap();
        assert!(matches!(
            receipt.status(),
            ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. }
        ));
        assert_eq!(
            receipt.normalized_key().recipe.capability,
            RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
        );
        assert_eq!(
            project_dynamic_class_v1(
                &record,
                &lock_request_validation_descriptor(
                    action,
                    stimulus,
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
fn exact_lock_lifecycle_programs_are_inventoried_without_granting_supported() {
    let cases = lock_lifecycle_cases();
    assert_eq!(cases.len(), 104);
    for case in cases {
        let record = lock_lifecycle_record(case);
        let descriptor = lock_lifecycle_descriptor(
            case,
            RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
        );
        let prepared = prepare_dynamic_terminal_v1(&record, &descriptor).unwrap();
        let receipt = super::super::runner_admission::inventory_v1(&prepared.key).unwrap();
        assert!(matches!(
            receipt.status(),
            ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. }
        ));
        assert_eq!(
            project_dynamic_class_v1(
                &record,
                &lock_lifecycle_descriptor(case, RunnerCapabilityV1::Supported),
            ),
            Err(ProjectionErrorV1::Invalid(
                ProjectionViolationV1::RunnerAdmissionUnsealedSupported,
            )),
        );
    }
}

#[test]
fn exact_lock_native_acquire_busy_programs_are_inventoried_without_granting_supported() {
    let leaves = frozen_lock_native_acquire_busy_leaves_v1();
    assert_eq!(leaves.len(), LOCK_NATIVE_ACQUIRE_BUSY_MEMBER_COUNT);
    for (&case, leaf) in leaves {
        let prepared = prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor).unwrap();
        let receipt = super::super::runner_admission::inventory_v1(&prepared.key).unwrap();
        assert!(matches!(
            receipt.status(),
            ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. }
        ));
        assert_eq!(
            receipt.normalized_key().recipe.capability,
            RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
        );
        assert_eq!(
            project_dynamic_class_v1(
                &leaf.record,
                &lock_native_acquire_busy_descriptor_v1(case, RunnerCapabilityV1::Supported),
            ),
            Err(ProjectionErrorV1::Invalid(
                ProjectionViolationV1::RunnerAdmissionUnsealedSupported,
            )),
        );
    }
}

#[test]
fn exact_lock_stored_poison_programs_are_inventoried_without_granting_supported() {
    let leaves = frozen_lock_stored_poison_leaves_v1();
    assert_eq!(leaves.len(), LOCK_STORED_POISON_MEMBER_COUNT);
    for (&case, leaf) in leaves {
        let prepared = prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor).unwrap();
        let receipt = super::super::runner_admission::inventory_v1(&prepared.key).unwrap();
        assert!(matches!(
            receipt.status(),
            ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. }
        ));
        assert_eq!(
            receipt.normalized_key().recipe.capability,
            RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
        );
        assert_eq!(
            project_dynamic_class_v1(
                &leaf.record,
                &lock_stored_poison_descriptor_v1(case, RunnerCapabilityV1::Supported),
            ),
            Err(ProjectionErrorV1::Invalid(
                ProjectionViolationV1::RunnerAdmissionUnsealedSupported,
            )),
        );
    }
}

#[test]
fn full_lock_program_inventory_accounts_for_every_frozen_member_without_opening_quotient() {
    let bundle =
        build_lock_execution_program_inventory_v1(&super::super::super::lock::graph()).unwrap();
    let inventory = &bundle.inventory;
    assert_eq!(inventory.member_count, 8_668);
    assert_eq!(bundle.reverse_index.len(), 8_668);
    assert_eq!(inventory.source_present_member_count, 2_798);
    assert_eq!(inventory.source_present_group_count, 2_798);
    assert_eq!(inventory.planned_missing_member_count, 5_870);
    let source_groups = bundle
        .groups
        .iter()
        .filter(|group| {
            matches!(
                group.status,
                ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. }
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(source_groups.len(), 2_798);
    assert!(source_groups.iter().all(|group| group.member_count == 1));

    let record = lock_request_validation_record();
    let mut expected_source_keys = LOCK_REQUEST_VALIDATION_PROGRAMS
        .into_iter()
        .map(|(action, stimulus)| {
            prepare_dynamic_terminal_v1(
                &record,
                &lock_request_validation_descriptor(
                    action,
                    stimulus,
                    RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
                ),
            )
            .unwrap()
            .key
        })
        .collect::<Vec<_>>();
    expected_source_keys.extend(lock_lifecycle_cases().into_iter().map(|case| {
        let record = lock_lifecycle_record(case);
        prepare_dynamic_terminal_v1(
            &record,
            &lock_lifecycle_descriptor(
                case,
                RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete),
            ),
        )
        .unwrap()
        .key
    }));
    expected_source_keys.extend(
        frozen_lock_native_acquire_busy_leaves_v1()
            .values()
            .map(|leaf| {
                prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor)
                    .unwrap()
                    .key
            }),
    );
    expected_source_keys.extend(frozen_lock_stored_poison_leaves_v1().values().map(|leaf| {
        prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor)
            .unwrap()
            .key
    }));
    let expected_source_keys = expected_source_keys.into_iter().collect::<BTreeSet<_>>();
    assert_eq!(expected_source_keys.len(), 2_798);

    let actual_source_groups = source_groups
        .iter()
        .map(|group| (group.normalized_key, group.members[0]))
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_source_groups.len(), 2_798);
    assert!(actual_source_groups
        .iter()
        .all(|(key, _)| expected_source_keys.contains(key)));
    let native_busy_expected_groups = frozen_lock_native_acquire_busy_leaves_v1()
        .values()
        .map(|leaf| {
            (
                prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor)
                    .unwrap()
                    .key,
                leaf.member,
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        native_busy_expected_groups.len(),
        LOCK_NATIVE_ACQUIRE_BUSY_MEMBER_COUNT
    );
    assert!(native_busy_expected_groups.is_subset(&actual_source_groups));
    let stored_expected_groups = frozen_lock_stored_poison_leaves_v1()
        .values()
        .map(|leaf| {
            (
                prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor)
                    .unwrap()
                    .key,
                leaf.member,
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        stored_expected_groups.len(),
        LOCK_STORED_POISON_MEMBER_COUNT
    );
    assert!(stored_expected_groups.is_subset(&actual_source_groups));
    assert!(bundle.groups.iter().all(|group| match group.status {
        ExecutionProgramInventoryStatusV1::SourcePresentReceiptRequired { .. } => {
            expected_source_keys.contains(&group.normalized_key)
        }
        ExecutionProgramInventoryStatusV1::PlannedMissing(gap) => {
            gap == CapabilityGapV1::LockObservationIncomplete
        }
    }));
}
