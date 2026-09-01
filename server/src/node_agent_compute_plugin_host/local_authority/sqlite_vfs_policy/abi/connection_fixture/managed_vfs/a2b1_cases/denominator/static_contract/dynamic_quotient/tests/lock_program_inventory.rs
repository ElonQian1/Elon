//! Exact pre-manifest inventory checks for all source-present Lock programs.

use std::collections::BTreeSet;

use super::super::runner_admission::ExecutionProgramInventoryStatusV1;
use super::lock_abi_scalar_rejection_cases::{
    lock_abi_scalar_rejection_expected_groups_v1, LOCK_ABI_SCALAR_REJECTION_MEMBER_COUNT,
};
use super::lock_callback_completion_route_unknown_cases::{
    frozen_lock_callback_completion_route_unknown_leaves_v1,
    lock_callback_completion_route_unknown_descriptor_v1,
    LOCK_CALLBACK_COMPLETION_ROUTE_UNKNOWN_MEMBER_COUNT,
};
use super::lock_local_protocol_rejection_cases::{
    frozen_lock_local_protocol_rejection_leaves_v1, lock_local_protocol_rejection_descriptor_v1,
    LOCK_LOCAL_PROTOCOL_REJECTION_MEMBER_COUNT,
};
use super::lock_local_sibling_contention_cases::{
    frozen_lock_local_sibling_contention_leaves_v1, lock_local_sibling_contention_descriptor_v1,
    LOCK_LOCAL_SIBLING_CONTENTION_MEMBER_COUNT,
};
use super::lock_native_acquire_busy_cases::{
    frozen_lock_native_acquire_busy_leaves_v1, lock_native_acquire_busy_descriptor_v1,
    LOCK_NATIVE_ACQUIRE_BUSY_MEMBER_COUNT,
};
use super::lock_pre_managed_callback_rejection_cases::{
    frozen_lock_pre_managed_callback_rejection_leaves_v1,
    LOCK_PRE_MANAGED_CALLBACK_REJECTION_MEMBER_COUNT,
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
fn exact_lock_callback_completion_route_unknown_programs_are_inventoried_without_granting_supported(
) {
    let leaves = frozen_lock_callback_completion_route_unknown_leaves_v1();
    assert_eq!(
        leaves.len(),
        LOCK_CALLBACK_COMPLETION_ROUTE_UNKNOWN_MEMBER_COUNT
    );
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
                &lock_callback_completion_route_unknown_descriptor_v1(
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
fn exact_lock_local_sibling_contention_programs_are_inventoried_without_granting_supported() {
    let leaves = frozen_lock_local_sibling_contention_leaves_v1();
    assert_eq!(leaves.len(), LOCK_LOCAL_SIBLING_CONTENTION_MEMBER_COUNT);
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
                &lock_local_sibling_contention_descriptor_v1(case, RunnerCapabilityV1::Supported,),
            ),
            Err(ProjectionErrorV1::Invalid(
                ProjectionViolationV1::RunnerAdmissionUnsealedSupported,
            )),
        );
    }
}

#[test]
fn exact_lock_local_protocol_rejection_programs_are_inventoried_without_granting_supported() {
    let leaves = frozen_lock_local_protocol_rejection_leaves_v1();
    assert_eq!(leaves.len(), LOCK_LOCAL_PROTOCOL_REJECTION_MEMBER_COUNT);
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
                &lock_local_protocol_rejection_descriptor_v1(case, RunnerCapabilityV1::Supported,),
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
    assert_eq!(inventory.program_group_count, 8_140);
    assert_eq!(inventory.source_present_member_count, 3_657);
    assert_eq!(inventory.source_present_group_count, 3_657);
    assert_eq!(inventory.planned_missing_member_count, 5_011);
    assert_eq!(inventory.planned_missing_group_count, 4_483);
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
    assert_eq!(source_groups.len(), 3_657);
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
        frozen_lock_callback_completion_route_unknown_leaves_v1()
            .values()
            .map(|leaf| {
                prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor)
                    .unwrap()
                    .key
            }),
    );
    expected_source_keys.extend(
        frozen_lock_local_sibling_contention_leaves_v1()
            .values()
            .map(|leaf| {
                prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor)
                    .unwrap()
                    .key
            }),
    );
    expected_source_keys.extend(
        frozen_lock_local_protocol_rejection_leaves_v1()
            .values()
            .map(|leaf| {
                prepare_dynamic_terminal_v1(&leaf.record, &leaf.descriptor)
                    .unwrap()
                    .key
            }),
    );
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
    let prior_source_keys = expected_source_keys
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    assert_eq!(prior_source_keys.len(), 3_122);
    let q9_expected_groups = frozen_lock_pre_managed_callback_rejection_leaves_v1()
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
        q9_expected_groups.len(),
        LOCK_PRE_MANAGED_CALLBACK_REJECTION_MEMBER_COUNT
    );
    assert!(q9_expected_groups
        .iter()
        .all(|(key, _)| !prior_source_keys.contains(key)));
    expected_source_keys.extend(q9_expected_groups.iter().map(|(key, _)| *key));
    let q1_through_q9_source_keys =
        expected_source_keys.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(q1_through_q9_source_keys.len(), 3_650);
    let q10_expected_groups = lock_abi_scalar_rejection_expected_groups_v1();
    assert_eq!(
        q10_expected_groups.len(),
        LOCK_ABI_SCALAR_REJECTION_MEMBER_COUNT
    );
    assert!(q10_expected_groups
        .iter()
        .all(|(key, _)| !q1_through_q9_source_keys.contains(key)));
    expected_source_keys.extend(q10_expected_groups.iter().map(|(key, _)| *key));
    let expected_source_keys = expected_source_keys.into_iter().collect::<BTreeSet<_>>();
    assert_eq!(expected_source_keys.len(), 3_657);

    let actual_source_groups = source_groups
        .iter()
        .map(|group| (group.normalized_key, group.members[0]))
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_source_groups.len(), 3_657);
    assert!(actual_source_groups
        .iter()
        .all(|(key, _)| expected_source_keys.contains(key)));
    assert!(q9_expected_groups.is_subset(&actual_source_groups));
    assert!(q10_expected_groups.is_subset(&actual_source_groups));
    let q9_members = q9_expected_groups
        .iter()
        .map(|(_, member)| *member)
        .collect::<BTreeSet<_>>();
    let prior_members = actual_source_groups
        .iter()
        .filter(|(key, _)| prior_source_keys.contains(key))
        .map(|(_, member)| *member)
        .collect::<BTreeSet<_>>();
    assert_eq!(prior_members.len(), 3_122);
    assert!(q9_members.is_disjoint(&prior_members));
    let q10_members = q10_expected_groups
        .iter()
        .map(|(_, member)| *member)
        .collect::<BTreeSet<_>>();
    let q1_through_q9_members = actual_source_groups
        .iter()
        .filter(|(key, _)| q1_through_q9_source_keys.contains(key))
        .map(|(_, member)| *member)
        .collect::<BTreeSet<_>>();
    assert_eq!(q1_through_q9_members.len(), 3_650);
    assert!(q10_members.is_disjoint(&q1_through_q9_members));
    let callback_route_unknown_expected_groups =
        frozen_lock_callback_completion_route_unknown_leaves_v1()
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
        callback_route_unknown_expected_groups.len(),
        LOCK_CALLBACK_COMPLETION_ROUTE_UNKNOWN_MEMBER_COUNT
    );
    assert!(callback_route_unknown_expected_groups.is_subset(&actual_source_groups));
    let local_contention_expected_groups = frozen_lock_local_sibling_contention_leaves_v1()
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
        local_contention_expected_groups.len(),
        LOCK_LOCAL_SIBLING_CONTENTION_MEMBER_COUNT
    );
    assert!(local_contention_expected_groups.is_subset(&actual_source_groups));
    let local_protocol_rejection_expected_groups = frozen_lock_local_protocol_rejection_leaves_v1()
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
        local_protocol_rejection_expected_groups.len(),
        LOCK_LOCAL_PROTOCOL_REJECTION_MEMBER_COUNT
    );
    assert!(local_protocol_rejection_expected_groups.is_subset(&actual_source_groups));
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
