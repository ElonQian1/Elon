use std::num::NonZeroU8;

use super::{
    controller::{ColdPrestateV1, ManagedSqliteShmTestInitializationControllerV1, TerminalStateV1},
    model::{
        ManagedSqliteShmTestInitializationExpectationV1,
        ManagedSqliteShmTestInitializationFailureV1,
        ManagedSqliteShmTestInitializationNativeObservationV1,
        ManagedSqliteShmTestInitializationNativeReceiptV1,
    },
};
use crate::node_agent_managed_fs::sqlite_namespace::shm::{
    test_lock_runtime::{
        ManagedSqliteShmTestLockExpectation, ManagedSqliteShmTestLockPath,
        ManagedSqliteShmTestLockReceipt,
    },
    types::{ManagedSqliteShmLockAction, ManagedSqliteShmLockRequest, SHM_DMS_OFFSET},
};

const TARGET: (u64, u64) = (17, 23);

fn expectation() -> ManagedSqliteShmTestInitializationExpectationV1 {
    ManagedSqliteShmTestInitializationExpectationV1 {
        case_v1:
            ManagedSqliteShmTestInitializationFailureV1::CreatedFirstExclusiveReleaseOutcomeUncertain,
        action: ManagedSqliteShmLockAction::LockShared,
        first: 0,
        count: 1,
        mask: 1,
    }
}

fn existing_first_expectation() -> ManagedSqliteShmTestInitializationExpectationV1 {
    ManagedSqliteShmTestInitializationExpectationV1 {
        case_v1:
            ManagedSqliteShmTestInitializationFailureV1::ExistingFirstExclusiveReleaseOutcomeUncertain,
        ..expectation()
    }
}

fn truncate_release_succeeded_expectation() -> ManagedSqliteShmTestInitializationExpectationV1 {
    ManagedSqliteShmTestInitializationExpectationV1 {
        case_v1:
            ManagedSqliteShmTestInitializationFailureV1::CreatedFirstTruncateOutcomeUncertainReleaseSucceeded,
        ..expectation()
    }
}

fn existing_first_truncate_release_succeeded_expectation(
) -> ManagedSqliteShmTestInitializationExpectationV1 {
    ManagedSqliteShmTestInitializationExpectationV1 {
        case_v1:
            ManagedSqliteShmTestInitializationFailureV1::ExistingFirstTruncateOutcomeUncertainReleaseSucceeded,
        ..expectation()
    }
}

fn truncate_release_failed_expectation() -> ManagedSqliteShmTestInitializationExpectationV1 {
    ManagedSqliteShmTestInitializationExpectationV1 {
        case_v1:
            ManagedSqliteShmTestInitializationFailureV1::CreatedFirstTruncateOutcomeUncertainReleaseFailed,
        ..expectation()
    }
}

fn request() -> ManagedSqliteShmLockRequest {
    ManagedSqliteShmLockRequest::new(
        0,
        NonZeroU8::new(1).expect("one-slot request"),
        ManagedSqliteShmLockAction::LockShared,
    )
    .expect("legal one-slot Lock request")
}

fn cold() -> ColdPrestateV1 {
    ColdPrestateV1 {
        target_attached: true,
        shm_connections: 1,
        node_present: false,
        shm_file_present: false,
        poisoned: false,
        domain_terminal: false,
        shared_mask: 0,
        exclusive_mask: 0,
    }
}

fn terminal() -> TerminalStateV1 {
    TerminalStateV1 {
        target_attached: true,
        shm_connections: 1,
        node_present: true,
        shm_file_present: true,
        dms_exclusive_outcome_uncertain: true,
        dms_released: false,
        poisoned: true,
        mutation_may_have_occurred: true,
        lock_outcome_uncertain: true,
        domain_terminal: true,
        shared_mask: 0,
        exclusive_mask: 0,
    }
}

fn truncate_release_succeeded_terminal() -> TerminalStateV1 {
    TerminalStateV1 {
        dms_exclusive_outcome_uncertain: false,
        dms_released: true,
        lock_outcome_uncertain: false,
        ..terminal()
    }
}

fn requested_lock_receipt() -> ManagedSqliteShmTestLockReceipt {
    let expected = expectation();
    ManagedSqliteShmTestLockReceipt {
        runtime_generation: TARGET.0,
        shm_connection_id: TARGET.1,
        expectation: ManagedSqliteShmTestLockExpectation {
            action: expected.action,
            first: expected.first,
            count: expected.count,
            mask: expected.mask,
            path: ManagedSqliteShmTestLockPath::InitializationFailure,
        },
        managed_attempts: 1,
        managed_successes: 0,
        native_lock_attempts: 0,
        native_lock_acquired: 0,
        native_lock_contended: 0,
        native_lock_errors: 0,
        native_unlock_attempts: 0,
        native_unlock_successes: 0,
        native_unlock_errors: 0,
        local_transitions: 0,
        finished: true,
    }
}

fn advance_to_terminal_with_open(
    controller: &mut ManagedSqliteShmTestInitializationControllerV1,
    created: bool,
) {
    assert!(controller.record_request(TARGET, request()).unwrap());
    assert!(controller.record_open_attempt(TARGET).unwrap());
    assert!(controller.record_open_created(TARGET, created).unwrap());
    assert!(controller
        .record_dms_exclusive_lock_attempt(TARGET)
        .unwrap());
    assert!(controller.record_dms_exclusive_acquired(TARGET).unwrap());
    assert!(controller.record_truncate_attempt(TARGET).unwrap());
    assert!(controller.record_truncate_success(TARGET).unwrap());
    assert!(controller.begin_dms_exclusive_unlock(TARGET).unwrap());
    controller
        .record_return_receipt_unavailable(
            TARGET,
            ManagedSqliteShmTestInitializationNativeReceiptV1 {
                observation:
                    ManagedSqliteShmTestInitializationNativeObservationV1::ReturnReceiptUnavailable,
                offset: SHM_DMS_OFFSET,
                length: 1,
                exact_call_occurrence: 1,
            },
        )
        .unwrap();
    controller.record_poisoned(TARGET).unwrap();
}

fn advance_to_terminal(controller: &mut ManagedSqliteShmTestInitializationControllerV1) {
    advance_to_terminal_with_open(controller, true);
}

fn advance_q16_to_truncate_attempt(
    controller: &mut ManagedSqliteShmTestInitializationControllerV1,
) {
    controller
        .arm(TARGET, truncate_release_failed_expectation(), cold())
        .unwrap();
    assert!(controller.record_request(TARGET, request()).unwrap());
    assert!(controller.record_open_attempt(TARGET).unwrap());
    assert!(controller.record_open_created(TARGET, true).unwrap());
    assert!(controller
        .record_dms_exclusive_lock_attempt(TARGET)
        .unwrap());
    assert!(controller.record_dms_exclusive_acquired(TARGET).unwrap());
    assert!(controller.record_truncate_attempt(TARGET).unwrap());
}

fn q16_truncate_native() -> ManagedSqliteShmTestInitializationNativeReceiptV1 {
    ManagedSqliteShmTestInitializationNativeReceiptV1 {
        observation:
            ManagedSqliteShmTestInitializationNativeObservationV1::ReturnReceiptUnavailable,
        offset: 0,
        length: 0,
        exact_call_occurrence: 1,
    }
}

fn q16_cleanup_native() -> ManagedSqliteShmTestInitializationNativeReceiptV1 {
    ManagedSqliteShmTestInitializationNativeReceiptV1 {
        observation:
            ManagedSqliteShmTestInitializationNativeObservationV1::ReturnReceiptUnavailable,
        offset: SHM_DMS_OFFSET,
        length: 1,
        exact_call_occurrence: 1,
    }
}

#[test]
fn exact_created_first_release_sequence_seals_one_controlled_receipt() {
    let mut controller = ManagedSqliteShmTestInitializationControllerV1::default();
    controller.arm(TARGET, expectation(), cold()).unwrap();
    advance_to_terminal(&mut controller);
    let receipt = controller
        .finish(TARGET, terminal(), requested_lock_receipt())
        .unwrap();
    let ordered = receipt.ordered_values();
    assert_eq!(receipt.cleanup_native_receipt(), None);
    assert_eq!(ordered[0], 1);
    assert_eq!(ordered[3], TARGET.0);
    assert_eq!(ordered[4], TARGET.1);
    assert_eq!(ordered[14..21], [1, 1, 1, 1, 1, 1, 1]);
    assert_eq!(ordered[22], SHM_DMS_OFFSET);
    assert_eq!(ordered[23], 1);
    assert_eq!(ordered[24], 1);
    assert_eq!(ordered[29..32], [0, 1, 1]);
}

#[test]
fn exact_existing_first_release_sequence_requires_created_false() {
    let mut controller = ManagedSqliteShmTestInitializationControllerV1::default();
    let expected = existing_first_expectation();
    controller.arm(TARGET, expected, cold()).unwrap();
    advance_to_terminal_with_open(&mut controller, false);
    let mut lock = requested_lock_receipt();
    lock.expectation.action = expected.action;
    let receipt = controller.finish(TARGET, terminal(), lock).unwrap();
    assert_eq!(receipt.case_v1(), expected.case_v1);
    assert_eq!(receipt.cleanup_native_receipt(), None);
    assert_eq!(receipt.ordered_values()[1], 2);
}

#[test]
fn exact_created_first_truncate_unavailable_then_release_success_is_case_specific() {
    let mut controller = ManagedSqliteShmTestInitializationControllerV1::default();
    let expected = truncate_release_succeeded_expectation();
    controller.arm(TARGET, expected, cold()).unwrap();
    assert!(controller.record_request(TARGET, request()).unwrap());
    assert!(controller.record_open_attempt(TARGET).unwrap());
    assert!(controller.record_open_created(TARGET, true).unwrap());
    assert!(controller
        .record_dms_exclusive_lock_attempt(TARGET)
        .unwrap());
    assert!(controller.record_dms_exclusive_acquired(TARGET).unwrap());
    assert!(controller.record_truncate_attempt(TARGET).unwrap());
    assert!(controller
        .begin_created_first_truncate_outcome_unavailable(TARGET)
        .unwrap());
    controller
        .record_created_first_truncate_return_receipt_unavailable(
            TARGET,
            ManagedSqliteShmTestInitializationNativeReceiptV1 {
                observation:
                    ManagedSqliteShmTestInitializationNativeObservationV1::ReturnReceiptUnavailable,
                offset: 0,
                length: 0,
                exact_call_occurrence: 1,
            },
        )
        .unwrap();
    controller
        .begin_created_first_truncate_cleanup_unlock(TARGET)
        .unwrap();
    controller
        .record_created_first_truncate_cleanup_unlock_succeeded(TARGET)
        .unwrap();
    controller.record_poisoned(TARGET).unwrap();
    let receipt = controller
        .finish(
            TARGET,
            truncate_release_succeeded_terminal(),
            requested_lock_receipt(),
        )
        .unwrap();
    assert_eq!(receipt.case_v1(), expected.case_v1);
    assert_eq!(receipt.cleanup_native_receipt(), None);
    assert_eq!(receipt.ordered_values()[1], 3);
    assert_eq!(
        receipt.ordered_values()[19..28],
        [0, 1, 1, 0, 0, 1, 1, 1, 0]
    );
    assert_eq!(receipt.ordered_values()[28], 447);
}

#[test]
fn exact_created_first_truncate_unavailable_then_release_failed_seals_two_receipts() {
    let mut controller = ManagedSqliteShmTestInitializationControllerV1::default();
    let expected = truncate_release_failed_expectation();
    controller.arm(TARGET, expected, cold()).unwrap();
    assert!(controller.record_request(TARGET, request()).unwrap());
    assert!(controller.record_open_attempt(TARGET).unwrap());
    assert!(controller.record_open_created(TARGET, true).unwrap());
    assert!(controller
        .record_dms_exclusive_lock_attempt(TARGET)
        .unwrap());
    assert!(controller.record_dms_exclusive_acquired(TARGET).unwrap());
    assert!(controller.record_truncate_attempt(TARGET).unwrap());
    assert!(controller
        .begin_created_first_truncate_error_release_failed(TARGET)
        .unwrap());
    let truncate_native = ManagedSqliteShmTestInitializationNativeReceiptV1 {
        observation:
            ManagedSqliteShmTestInitializationNativeObservationV1::ReturnReceiptUnavailable,
        offset: 0,
        length: 0,
        exact_call_occurrence: 1,
    };
    controller
        .record_created_first_truncate_error_release_failed_truncate_receipt(
            TARGET,
            truncate_native,
        )
        .unwrap();
    controller
        .begin_created_first_truncate_error_release_failed_cleanup_unlock(TARGET)
        .unwrap();
    let cleanup_native = ManagedSqliteShmTestInitializationNativeReceiptV1 {
        observation:
            ManagedSqliteShmTestInitializationNativeObservationV1::ReturnReceiptUnavailable,
        offset: SHM_DMS_OFFSET,
        length: 1,
        exact_call_occurrence: 1,
    };
    controller
        .record_created_first_truncate_error_release_failed_cleanup_receipt(
            TARGET,
            cleanup_native,
        )
        .unwrap();
    controller.record_poisoned(TARGET).unwrap();
    let receipt = controller
        .finish(TARGET, terminal(), requested_lock_receipt())
        .unwrap();
    assert_eq!(receipt.case_v1(), expected.case_v1);
    assert_eq!(receipt.native_receipt(), truncate_native);
    assert_eq!(receipt.cleanup_native_receipt(), Some(cleanup_native));
    assert_eq!(receipt.ordered_values()[1], 5);
    assert_eq!(
        receipt.ordered_values()[19..28],
        [0, 1, 1, 0, 0, 1, 1, 0, 1]
    );
    assert_eq!(receipt.ordered_values()[28], 511);
    assert_eq!(receipt.ordered_values()[29..32], [0, 1, 1]);
}

#[test]
fn q16_missing_reordered_and_duplicate_receipts_fail_closed() {
    let mut missing_cleanup = ManagedSqliteShmTestInitializationControllerV1::default();
    advance_q16_to_truncate_attempt(&mut missing_cleanup);
    assert!(missing_cleanup
        .begin_created_first_truncate_error_release_failed(TARGET)
        .unwrap());
    missing_cleanup
        .record_created_first_truncate_error_release_failed_truncate_receipt(
            TARGET,
            q16_truncate_native(),
        )
        .unwrap();
    missing_cleanup
        .begin_created_first_truncate_error_release_failed_cleanup_unlock(TARGET)
        .unwrap();
    assert!(missing_cleanup.record_poisoned(TARGET).is_err());

    let mut reordered_cleanup = ManagedSqliteShmTestInitializationControllerV1::default();
    advance_q16_to_truncate_attempt(&mut reordered_cleanup);
    assert!(reordered_cleanup
        .begin_created_first_truncate_error_release_failed_cleanup_unlock(TARGET)
        .is_err());

    let mut duplicate_primary = ManagedSqliteShmTestInitializationControllerV1::default();
    advance_q16_to_truncate_attempt(&mut duplicate_primary);
    assert!(duplicate_primary
        .begin_created_first_truncate_error_release_failed(TARGET)
        .unwrap());
    duplicate_primary
        .record_created_first_truncate_error_release_failed_truncate_receipt(
            TARGET,
            q16_truncate_native(),
        )
        .unwrap();
    assert!(duplicate_primary
        .record_created_first_truncate_error_release_failed_truncate_receipt(
            TARGET,
            q16_truncate_native(),
        )
        .is_err());

    let mut duplicate_cleanup = ManagedSqliteShmTestInitializationControllerV1::default();
    advance_q16_to_truncate_attempt(&mut duplicate_cleanup);
    assert!(duplicate_cleanup
        .begin_created_first_truncate_error_release_failed(TARGET)
        .unwrap());
    duplicate_cleanup
        .record_created_first_truncate_error_release_failed_truncate_receipt(
            TARGET,
            q16_truncate_native(),
        )
        .unwrap();
    duplicate_cleanup
        .begin_created_first_truncate_error_release_failed_cleanup_unlock(TARGET)
        .unwrap();
    duplicate_cleanup
        .record_created_first_truncate_error_release_failed_cleanup_receipt(
            TARGET,
            q16_cleanup_native(),
        )
        .unwrap();
    assert!(duplicate_cleanup
        .record_created_first_truncate_error_release_failed_cleanup_receipt(
            TARGET,
            q16_cleanup_native(),
        )
        .is_err());
}

#[test]
fn exact_existing_first_truncate_unavailable_then_release_success_is_case_specific() {
    let mut controller = ManagedSqliteShmTestInitializationControllerV1::default();
    let expected = existing_first_truncate_release_succeeded_expectation();
    controller.arm(TARGET, expected, cold()).unwrap();
    assert!(controller.record_request(TARGET, request()).unwrap());
    assert!(controller.record_open_attempt(TARGET).unwrap());
    assert!(controller.record_open_created(TARGET, false).unwrap());
    assert!(controller
        .record_dms_exclusive_lock_attempt(TARGET)
        .unwrap());
    assert!(controller.record_dms_exclusive_acquired(TARGET).unwrap());
    assert!(controller.record_truncate_attempt(TARGET).unwrap());
    assert!(controller
        .begin_existing_first_truncate_outcome_unavailable(TARGET)
        .unwrap());
    controller
        .record_existing_first_truncate_return_receipt_unavailable(
            TARGET,
            ManagedSqliteShmTestInitializationNativeReceiptV1 {
                observation:
                    ManagedSqliteShmTestInitializationNativeObservationV1::ReturnReceiptUnavailable,
                offset: 0,
                length: 0,
                exact_call_occurrence: 1,
            },
        )
        .unwrap();
    controller
        .begin_existing_first_truncate_cleanup_unlock(TARGET)
        .unwrap();
    controller
        .record_existing_first_truncate_cleanup_unlock_succeeded(TARGET)
        .unwrap();
    controller.record_poisoned(TARGET).unwrap();
    let receipt = controller
        .finish(
            TARGET,
            truncate_release_succeeded_terminal(),
            requested_lock_receipt(),
        )
        .unwrap();
    assert_eq!(receipt.case_v1(), expected.case_v1);
    assert_eq!(receipt.cleanup_native_receipt(), None);
    assert_eq!(receipt.ordered_values()[1], 4);
    assert_eq!(
        receipt.ordered_values()[19..28],
        [0, 1, 1, 0, 0, 1, 1, 1, 0]
    );
    assert_eq!(receipt.ordered_values()[28], 447);
}

#[test]
fn initialization_open_existence_is_case_specific() {
    let mut created_first = ManagedSqliteShmTestInitializationControllerV1::default();
    created_first.arm(TARGET, expectation(), cold()).unwrap();
    assert!(created_first.record_request(TARGET, request()).unwrap());
    assert!(created_first.record_open_attempt(TARGET).unwrap());
    assert!(created_first.record_open_created(TARGET, false).is_err());

    let mut existing_first = ManagedSqliteShmTestInitializationControllerV1::default();
    existing_first
        .arm(TARGET, existing_first_expectation(), cold())
        .unwrap();
    assert!(existing_first.record_request(TARGET, request()).unwrap());
    assert!(existing_first.record_open_attempt(TARGET).unwrap());
    assert!(existing_first.record_open_created(TARGET, true).is_err());

    let mut existing_first_truncate = ManagedSqliteShmTestInitializationControllerV1::default();
    existing_first_truncate
        .arm(
            TARGET,
            existing_first_truncate_release_succeeded_expectation(),
            cold(),
        )
        .unwrap();
    assert!(existing_first_truncate
        .record_request(TARGET, request())
        .unwrap());
    assert!(existing_first_truncate.record_open_attempt(TARGET).unwrap());
    assert!(existing_first_truncate
        .record_open_created(TARGET, true)
        .is_err());
}

#[test]
fn wrong_target_and_out_of_order_event_poison_the_observation() {
    let mut wrong_target = ManagedSqliteShmTestInitializationControllerV1::default();
    wrong_target.arm(TARGET, expectation(), cold()).unwrap();
    assert!(wrong_target.record_request((17, 24), request()).is_err());
    assert!(wrong_target.record_request(TARGET, request()).is_err());

    let mut out_of_order = ManagedSqliteShmTestInitializationControllerV1::default();
    out_of_order.arm(TARGET, expectation(), cold()).unwrap();
    assert!(out_of_order.record_open_attempt(TARGET).is_err());
    assert!(out_of_order.record_request(TARGET, request()).is_err());
}

#[test]
fn wrong_request_and_unconsumed_receipt_fail_closed() {
    let mut wrong_request = ManagedSqliteShmTestInitializationControllerV1::default();
    wrong_request.arm(TARGET, expectation(), cold()).unwrap();
    let request = ManagedSqliteShmLockRequest::new(
        0,
        NonZeroU8::new(1).unwrap(),
        ManagedSqliteShmLockAction::LockExclusive,
    )
    .unwrap();
    assert!(wrong_request.record_request(TARGET, request).is_err());

    let mut unconsumed = ManagedSqliteShmTestInitializationControllerV1::default();
    unconsumed.arm(TARGET, expectation(), cold()).unwrap();
    assert!(unconsumed
        .finish(TARGET, terminal(), requested_lock_receipt())
        .is_err());
}

#[test]
fn different_thread_cannot_consume_the_exact_observation() {
    let mut controller = ManagedSqliteShmTestInitializationControllerV1::default();
    controller.arm(TARGET, expectation(), cold()).unwrap();
    let result = std::thread::spawn(move || controller.record_request(TARGET, request()))
        .join()
        .expect("thread must return normally");
    assert!(result.is_err());
}
