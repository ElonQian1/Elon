use std::num::NonZeroU8;

use super::{
    controller::{
        ColdPrestateV1, ManagedSqliteShmTestInitializationControllerV1, TerminalStateV1,
    },
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
    types::{
        ManagedSqliteShmLockAction, ManagedSqliteShmLockRequest, SHM_DMS_OFFSET,
    },
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
        poisoned: true,
        mutation_may_have_occurred: true,
        lock_outcome_uncertain: true,
        domain_terminal: true,
        shared_mask: 0,
        exclusive_mask: 0,
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

fn advance_to_terminal(controller: &mut ManagedSqliteShmTestInitializationControllerV1) {
    assert!(controller.record_request(TARGET, request()).unwrap());
    assert!(controller.record_open_attempt(TARGET).unwrap());
    assert!(controller.record_open_created(TARGET, true).unwrap());
    assert!(controller
        .record_dms_exclusive_lock_attempt(TARGET)
        .unwrap());
    assert!(controller
        .record_dms_exclusive_acquired(TARGET)
        .unwrap());
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

#[test]
fn exact_created_first_release_sequence_seals_one_controlled_receipt() {
    let mut controller = ManagedSqliteShmTestInitializationControllerV1::default();
    controller.arm(TARGET, expectation(), cold()).unwrap();
    advance_to_terminal(&mut controller);
    let receipt = controller
        .finish(TARGET, terminal(), requested_lock_receipt())
        .unwrap();
    let ordered = receipt.ordered_values();
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
