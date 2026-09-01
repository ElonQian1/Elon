use super::*;
use std::num::NonZeroU8;

const TARGET: ExactTarget = (7, 11);

fn expectation(
    action: ManagedSqliteShmLockAction,
    first: u8,
    count: u8,
    path: ManagedSqliteShmTestLockPath,
) -> ManagedSqliteShmTestLockExpectation {
    let end = first + count;
    let mask = ((1u16 << end) - (1u16 << first)) as u8;
    ManagedSqliteShmTestLockExpectation {
        action,
        first,
        count,
        mask,
        path,
    }
}

fn request(
    action: ManagedSqliteShmLockAction,
    first: u8,
    count: u8,
) -> ManagedSqliteShmLockRequest {
    ManagedSqliteShmLockRequest::new(first, NonZeroU8::new(count).unwrap(), action).unwrap()
}

fn local_expectation() -> ManagedSqliteShmTestLockExpectation {
    expectation(
        ManagedSqliteShmLockAction::LockShared,
        2,
        1,
        ManagedSqliteShmTestLockPath::Local,
    )
}

fn local_request(first: u8) -> ManagedSqliteShmLockRequest {
    request(ManagedSqliteShmLockAction::LockShared, first, 1)
}

fn rejection_expectation(
    action: ManagedSqliteShmLockAction,
    first: u8,
    count: u8,
) -> ManagedSqliteShmTestLockExpectation {
    expectation(
        action,
        first,
        count,
        ManagedSqliteShmTestLockPath::LocalProtocolRejection,
    )
}

fn arm_local(controller: &mut ManagedSqliteShmTestLockController) {
    controller.arm(TARGET, local_expectation()).unwrap();
}

fn arm_rejection(
    controller: &mut ManagedSqliteShmTestLockController,
    action: ManagedSqliteShmLockAction,
    first: u8,
    count: u8,
) -> ManagedSqliteShmLockRequest {
    controller
        .arm(TARGET, rejection_expectation(action, first, count))
        .unwrap();
    request(action, first, count)
}

#[test]
fn cancelling_exact_target_disarms_the_one_shot_observation() {
    let mut controller = ManagedSqliteShmTestLockController::default();
    arm_local(&mut controller);
    controller.cancel(TARGET).unwrap();
    arm_local(&mut controller);
}

#[test]
fn cancelling_another_target_preserves_the_armed_observation() {
    let mut controller = ManagedSqliteShmTestLockController::default();
    arm_local(&mut controller);
    assert_eq!(
        controller.cancel((7, 12)),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_TARGET_MISMATCH")
    );
    assert_eq!(
        controller.arm(TARGET, local_expectation()),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_OBSERVATION_ALREADY_ARMED")
    );
}

#[test]
fn wrong_request_invalidates_then_finish_disarms_the_observation() {
    let mut controller = ManagedSqliteShmTestLockController::default();
    arm_local(&mut controller);

    assert_eq!(
        controller.record(TARGET, local_request(3), LockEvent::ManagedAttempt),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_REQUEST_MISMATCH")
    );
    assert_eq!(
        controller.finish(TARGET),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_OBSERVATION_INVALID")
    );
    arm_local(&mut controller);
}

#[test]
fn wrong_path_invalidates_then_finish_disarms_the_observation() {
    let mut controller = ManagedSqliteShmTestLockController::default();
    let request = local_request(2);
    arm_local(&mut controller);
    controller
        .record(TARGET, request, LockEvent::ManagedAttempt)
        .unwrap();

    assert_eq!(
        controller.record(TARGET, request, LockEvent::NativeLockAttempt),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_PATH_MISMATCH")
    );
    assert_eq!(
        controller.finish(TARGET),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_OBSERVATION_INVALID")
    );
    arm_local(&mut controller);
}

#[test]
fn duplicate_event_invalidates_then_finish_disarms_the_observation() {
    let mut controller = ManagedSqliteShmTestLockController::default();
    let request = local_request(2);
    arm_local(&mut controller);
    controller
        .record(TARGET, request, LockEvent::ManagedAttempt)
        .unwrap();

    assert_eq!(
        controller.record(TARGET, request, LockEvent::ManagedAttempt),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_EVENT_SEQUENCE_INVALID")
    );
    assert_eq!(
        controller.finish(TARGET),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_OBSERVATION_INVALID")
    );
    arm_local(&mut controller);
}

#[test]
fn unfinished_finish_disarms_the_observation() {
    let mut controller = ManagedSqliteShmTestLockController::default();
    arm_local(&mut controller);

    assert_eq!(
        controller.finish(TARGET),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_OBSERVATION_INCOMPLETE")
    );
    arm_local(&mut controller);
}

#[test]
fn stored_poison_finish_seals_zero_events_and_allows_rearm() {
    let mut controller = ManagedSqliteShmTestLockController::default();
    arm_local(&mut controller);

    let receipt = controller
        .finish_stored_poison_without_attempt(TARGET)
        .unwrap();
    assert!(receipt.finished);
    assert_eq!(receipt.runtime_generation, TARGET.0);
    assert_eq!(receipt.shm_connection_id, TARGET.1);
    assert_eq!(receipt.expectation, local_expectation());
    assert_eq!(receipt.managed_attempts, 0);
    assert_eq!(receipt.managed_successes, 0);
    assert_eq!(receipt.native_lock_attempts, 0);
    assert_eq!(receipt.native_lock_acquired, 0);
    assert_eq!(receipt.native_lock_contended, 0);
    assert_eq!(receipt.native_lock_errors, 0);
    assert_eq!(receipt.native_unlock_attempts, 0);
    assert_eq!(receipt.native_unlock_successes, 0);
    assert_eq!(receipt.native_unlock_errors, 0);
    assert_eq!(receipt.local_transitions, 0);
    arm_local(&mut controller);
}

#[test]
fn stored_poison_finish_rejects_any_managed_event_and_disarms() {
    let mut controller = ManagedSqliteShmTestLockController::default();
    arm_local(&mut controller);
    controller
        .record(TARGET, local_request(2), LockEvent::ManagedAttempt)
        .unwrap();

    assert_eq!(
        controller.finish_stored_poison_without_attempt(TARGET),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_STORED_POISON_EVENT_OBSERVED")
    );
    arm_local(&mut controller);
}

#[test]
fn successful_local_sequence_returns_finished_receipt_and_allows_rearm() {
    let mut controller = ManagedSqliteShmTestLockController::default();
    let request = local_request(2);
    arm_local(&mut controller);
    controller
        .record(TARGET, request, LockEvent::ManagedAttempt)
        .unwrap();
    controller
        .record(TARGET, request, LockEvent::LocalTransition)
        .unwrap();
    controller
        .record(TARGET, request, LockEvent::ManagedSuccess)
        .unwrap();

    let receipt = controller.finish(TARGET).unwrap();
    assert!(receipt.finished);
    assert_eq!(receipt.runtime_generation, TARGET.0);
    assert_eq!(receipt.shm_connection_id, TARGET.1);
    assert_eq!(receipt.managed_attempts, 1);
    assert_eq!(receipt.local_transitions, 1);
    assert_eq!(receipt.managed_successes, 1);
    arm_local(&mut controller);
}

#[test]
fn exact_local_protocol_rejections_finish_without_synthetic_receipt_fields() {
    let cases = [
        (
            ManagedSqliteShmLockAction::LockShared,
            2,
            1,
            LocalProtocolRejectionKind::OwnOverlap,
        ),
        (
            ManagedSqliteShmLockAction::LockExclusive,
            2,
            2,
            LocalProtocolRejectionKind::OwnOverlap,
        ),
        (
            ManagedSqliteShmLockAction::UnlockShared,
            2,
            1,
            LocalProtocolRejectionKind::SharedNotHeld,
        ),
        (
            ManagedSqliteShmLockAction::UnlockExclusive,
            2,
            2,
            LocalProtocolRejectionKind::ExclusiveNotHeld,
        ),
    ];

    for (action, first, count, kind) in cases {
        let mut controller = ManagedSqliteShmTestLockController::default();
        let request = arm_rejection(&mut controller, action, first, count);
        controller
            .record(TARGET, request, LockEvent::ManagedAttempt)
            .unwrap();
        controller
            .record(TARGET, request, LockEvent::LocalProtocolRejected(kind))
            .unwrap();

        let receipt = controller.finish(TARGET).unwrap();
        assert!(receipt.finished);
        assert_eq!(receipt.runtime_generation, TARGET.0);
        assert_eq!(receipt.shm_connection_id, TARGET.1);
        assert_eq!(
            receipt.expectation,
            rejection_expectation(action, first, count)
        );
        assert_eq!(receipt.managed_attempts, 1);
        assert_eq!(receipt.managed_successes, 0);
        assert_eq!(receipt.native_lock_attempts, 0);
        assert_eq!(receipt.native_lock_acquired, 0);
        assert_eq!(receipt.native_lock_contended, 0);
        assert_eq!(receipt.native_lock_errors, 0);
        assert_eq!(receipt.native_unlock_attempts, 0);
        assert_eq!(receipt.native_unlock_successes, 0);
        assert_eq!(receipt.native_unlock_errors, 0);
        assert_eq!(receipt.local_transitions, 0);
    }
}

#[test]
fn local_protocol_rejection_requires_the_exact_kind_for_the_action() {
    let mut controller = ManagedSqliteShmTestLockController::default();
    let request = arm_rejection(
        &mut controller,
        ManagedSqliteShmLockAction::UnlockShared,
        2,
        1,
    );
    controller
        .record(TARGET, request, LockEvent::ManagedAttempt)
        .unwrap();

    assert_eq!(
        controller.record(
            TARGET,
            request,
            LockEvent::LocalProtocolRejected(LocalProtocolRejectionKind::OwnOverlap),
        ),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_LOCAL_PROTOCOL_REJECTION_KIND_MISMATCH")
    );
    assert_eq!(
        controller.finish(TARGET),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_OBSERVATION_INVALID")
    );
}

#[test]
fn local_protocol_rejection_requires_managed_attempt_first() {
    let mut controller = ManagedSqliteShmTestLockController::default();
    let request = arm_rejection(
        &mut controller,
        ManagedSqliteShmLockAction::LockShared,
        2,
        1,
    );

    assert_eq!(
        controller.record(
            TARGET,
            request,
            LockEvent::LocalProtocolRejected(LocalProtocolRejectionKind::OwnOverlap),
        ),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_EVENT_SEQUENCE_INVALID")
    );
    assert_eq!(
        controller.finish(TARGET),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_OBSERVATION_INVALID")
    );
}

#[test]
fn local_protocol_rejection_is_one_shot() {
    let mut controller = ManagedSqliteShmTestLockController::default();
    let request = arm_rejection(
        &mut controller,
        ManagedSqliteShmLockAction::LockExclusive,
        2,
        2,
    );
    controller
        .record(TARGET, request, LockEvent::ManagedAttempt)
        .unwrap();
    controller
        .record(
            TARGET,
            request,
            LockEvent::LocalProtocolRejected(LocalProtocolRejectionKind::OwnOverlap),
        )
        .unwrap();

    assert_eq!(
        controller.record(
            TARGET,
            request,
            LockEvent::LocalProtocolRejected(LocalProtocolRejectionKind::OwnOverlap),
        ),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_EVENT_SEQUENCE_INVALID")
    );
    assert_eq!(
        controller.finish(TARGET),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_OBSERVATION_INVALID")
    );
}

#[test]
fn local_protocol_rejection_rejects_managed_success() {
    let mut controller = ManagedSqliteShmTestLockController::default();
    let request = arm_rejection(
        &mut controller,
        ManagedSqliteShmLockAction::LockShared,
        2,
        1,
    );
    controller
        .record(TARGET, request, LockEvent::ManagedAttempt)
        .unwrap();

    assert_eq!(
        controller.record(TARGET, request, LockEvent::ManagedSuccess),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_LOCAL_PROTOCOL_REJECTION_SUCCEEDED")
    );
    assert_eq!(
        controller.finish(TARGET),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_OBSERVATION_INVALID")
    );
}

#[test]
fn local_protocol_rejection_rejects_native_and_other_local_events() {
    for event in [
        LockEvent::NativeLockAttempt,
        LockEvent::NativeUnlockAttempt,
        LockEvent::LocalTransition,
        LockEvent::LocalContention,
    ] {
        let mut controller = ManagedSqliteShmTestLockController::default();
        let request = arm_rejection(
            &mut controller,
            ManagedSqliteShmLockAction::LockShared,
            2,
            1,
        );
        controller
            .record(TARGET, request, LockEvent::ManagedAttempt)
            .unwrap();

        assert_eq!(
            controller.record(TARGET, request, event),
            Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_PATH_MISMATCH")
        );
        assert_eq!(
            controller.finish(TARGET),
            Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_OBSERVATION_INVALID")
        );
    }
}

#[test]
fn local_protocol_rejection_shared_ranges_are_exactly_one_byte() {
    let mut controller = ManagedSqliteShmTestLockController::default();
    assert_eq!(
        controller.arm(
            TARGET,
            rejection_expectation(ManagedSqliteShmLockAction::LockShared, 2, 2),
        ),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_ACTION_PATH_MISMATCH")
    );
    assert_eq!(
        controller.arm(
            TARGET,
            rejection_expectation(ManagedSqliteShmLockAction::UnlockShared, 2, 2),
        ),
        Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_ACTION_PATH_MISMATCH")
    );
}
