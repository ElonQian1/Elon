//! Exact-target, Windows-test-only observation for one managed SHM lock action.

use super::{
    coordinator::ManagedSqliteShmCoordinator,
    types::{
        ManagedSqliteShmFailure, ManagedSqliteShmFailurePhase, ManagedSqliteShmLockAction,
        ManagedSqliteShmLockRequest, SHM_LOCK_COUNT,
    },
};

type ExactTarget = (u64, u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmTestLockPath {
    NativeAcquire,
    NativeRelease,
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestLockExpectation {
    pub(crate) action: ManagedSqliteShmLockAction,
    pub(crate) first: u8,
    pub(crate) count: u8,
    pub(crate) mask: u8,
    pub(crate) path: ManagedSqliteShmTestLockPath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestLockReceipt {
    pub(crate) runtime_generation: u64,
    pub(crate) shm_connection_id: u64,
    pub(crate) expectation: ManagedSqliteShmTestLockExpectation,
    pub(crate) managed_attempts: u8,
    pub(crate) managed_successes: u8,
    pub(crate) native_lock_attempts: u8,
    pub(crate) native_lock_acquired: u8,
    pub(crate) native_lock_contended: u8,
    pub(crate) native_lock_errors: u8,
    pub(crate) native_unlock_attempts: u8,
    pub(crate) native_unlock_successes: u8,
    pub(crate) native_unlock_errors: u8,
    pub(crate) local_transitions: u8,
    pub(crate) finished: bool,
}

pub(super) enum ManagedSqliteShmTestNativeLockOutcome {
    Acquired,
    Contended,
    Error,
}

pub(super) enum ManagedSqliteShmTestNativeUnlockOutcome {
    Success,
    Error,
}

enum LockEvent {
    ManagedAttempt,
    ManagedSuccess,
    NativeLockAttempt,
    NativeLockOutcome(ManagedSqliteShmTestNativeLockOutcome),
    NativeUnlockAttempt,
    NativeUnlockOutcome(ManagedSqliteShmTestNativeUnlockOutcome),
    LocalTransition,
}

#[derive(PartialEq, Eq)]
enum Progress {
    Armed,
    ManagedAttempted,
    NativeLockAttempted,
    NativeLockAcquired,
    NativeLockContended,
    NativeLockError,
    NativeUnlockAttempted,
    NativeUnlockSucceeded,
    NativeUnlockError,
    LocalTransitioned,
    ManagedSucceeded,
}

struct ArmedLockObservation {
    target: ExactTarget,
    receipt: ManagedSqliteShmTestLockReceipt,
    progress: Progress,
    invalid: bool,
}

#[derive(Default)]
pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) struct ManagedSqliteShmTestLockController
{
    armed: Option<ArmedLockObservation>,
}

impl ManagedSqliteShmTestLockController {
    pub(super) fn arm(
        &mut self,
        target: ExactTarget,
        expectation: ManagedSqliteShmTestLockExpectation,
    ) -> Result<(), &'static str> {
        validate_expectation(target, expectation)?;
        if self.armed.is_some() {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_OBSERVATION_ALREADY_ARMED");
        }
        self.armed = Some(ArmedLockObservation {
            target,
            receipt: ManagedSqliteShmTestLockReceipt {
                runtime_generation: target.0,
                shm_connection_id: target.1,
                expectation,
                managed_attempts: 0,
                managed_successes: 0,
                native_lock_attempts: 0,
                native_lock_acquired: 0,
                native_lock_contended: 0,
                native_lock_errors: 0,
                native_unlock_attempts: 0,
                native_unlock_successes: 0,
                native_unlock_errors: 0,
                local_transitions: 0,
                finished: false,
            },
            progress: Progress::Armed,
            invalid: false,
        });
        Ok(())
    }

    fn record(
        &mut self,
        target: ExactTarget,
        request: ManagedSqliteShmLockRequest,
        event: LockEvent,
    ) -> Result<(), &'static str> {
        let Some(armed) = self.matching_armed_mut(target) else {
            return Ok(());
        };
        require_request(armed, request)?;
        macro_rules! record_once {
            ($counter:ident, $required:expr, $next:expr, $duplicate:literal) => {{
                require_progress(armed, $required)?;
                if armed.receipt.$counter != 0 {
                    armed.invalid = true;
                    return Err($duplicate);
                }
                armed.receipt.$counter = 1;
                armed.progress = $next;
            }};
        }
        match event {
            LockEvent::ManagedAttempt => record_once!(
                managed_attempts,
                Progress::Armed,
                Progress::ManagedAttempted,
                "NODE_MANAGED_SQLITE_SHM_TEST_LOCK_MANAGED_ATTEMPT_DUPLICATE"
            ),
            LockEvent::ManagedSuccess => {
                let required = match armed.receipt.expectation.path {
                    ManagedSqliteShmTestLockPath::NativeAcquire => Progress::NativeLockAcquired,
                    ManagedSqliteShmTestLockPath::NativeRelease => Progress::NativeUnlockSucceeded,
                    ManagedSqliteShmTestLockPath::Local => Progress::LocalTransitioned,
                };
                record_once!(
                    managed_successes,
                    required,
                    Progress::ManagedSucceeded,
                    "NODE_MANAGED_SQLITE_SHM_TEST_LOCK_MANAGED_SUCCESS_DUPLICATE"
                );
            }
            LockEvent::NativeLockAttempt => {
                require_path(armed, ManagedSqliteShmTestLockPath::NativeAcquire)?;
                record_once!(
                    native_lock_attempts,
                    Progress::ManagedAttempted,
                    Progress::NativeLockAttempted,
                    "NODE_MANAGED_SQLITE_SHM_TEST_LOCK_NATIVE_LOCK_ATTEMPT_DUPLICATE"
                );
            }
            LockEvent::NativeLockOutcome(outcome) => {
                require_path(armed, ManagedSqliteShmTestLockPath::NativeAcquire)?;
                match outcome {
                    ManagedSqliteShmTestNativeLockOutcome::Acquired => record_once!(
                        native_lock_acquired,
                        Progress::NativeLockAttempted,
                        Progress::NativeLockAcquired,
                        "NODE_MANAGED_SQLITE_SHM_TEST_LOCK_NATIVE_LOCK_ACQUIRED_DUPLICATE"
                    ),
                    ManagedSqliteShmTestNativeLockOutcome::Contended => record_once!(
                        native_lock_contended,
                        Progress::NativeLockAttempted,
                        Progress::NativeLockContended,
                        "NODE_MANAGED_SQLITE_SHM_TEST_LOCK_NATIVE_LOCK_CONTENDED_DUPLICATE"
                    ),
                    ManagedSqliteShmTestNativeLockOutcome::Error => record_once!(
                        native_lock_errors,
                        Progress::NativeLockAttempted,
                        Progress::NativeLockError,
                        "NODE_MANAGED_SQLITE_SHM_TEST_LOCK_NATIVE_LOCK_ERROR_DUPLICATE"
                    ),
                }
            }
            LockEvent::NativeUnlockAttempt => {
                require_path(armed, ManagedSqliteShmTestLockPath::NativeRelease)?;
                record_once!(
                    native_unlock_attempts,
                    Progress::ManagedAttempted,
                    Progress::NativeUnlockAttempted,
                    "NODE_MANAGED_SQLITE_SHM_TEST_LOCK_NATIVE_UNLOCK_ATTEMPT_DUPLICATE"
                );
            }
            LockEvent::NativeUnlockOutcome(outcome) => {
                require_path(armed, ManagedSqliteShmTestLockPath::NativeRelease)?;
                match outcome {
                    ManagedSqliteShmTestNativeUnlockOutcome::Success => record_once!(
                        native_unlock_successes,
                        Progress::NativeUnlockAttempted,
                        Progress::NativeUnlockSucceeded,
                        "NODE_MANAGED_SQLITE_SHM_TEST_LOCK_NATIVE_UNLOCK_SUCCESS_DUPLICATE"
                    ),
                    ManagedSqliteShmTestNativeUnlockOutcome::Error => record_once!(
                        native_unlock_errors,
                        Progress::NativeUnlockAttempted,
                        Progress::NativeUnlockError,
                        "NODE_MANAGED_SQLITE_SHM_TEST_LOCK_NATIVE_UNLOCK_ERROR_DUPLICATE"
                    ),
                }
            }
            LockEvent::LocalTransition => {
                require_path(armed, ManagedSqliteShmTestLockPath::Local)?;
                record_once!(
                    local_transitions,
                    Progress::ManagedAttempted,
                    Progress::LocalTransitioned,
                    "NODE_MANAGED_SQLITE_SHM_TEST_LOCK_LOCAL_TRANSITION_DUPLICATE"
                );
            }
        }
        Ok(())
    }

    pub(super) fn finish(
        &mut self,
        target: ExactTarget,
    ) -> Result<ManagedSqliteShmTestLockReceipt, &'static str> {
        if self.armed.as_ref().map(|armed| armed.target) != Some(target) {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_TARGET_MISMATCH");
        }
        let armed = self
            .armed
            .take()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_OBSERVATION_NOT_ARMED")?;
        if armed.invalid {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_OBSERVATION_INVALID");
        }
        if !matches!(
            armed.progress,
            Progress::ManagedSucceeded
                | Progress::NativeLockContended
                | Progress::NativeLockError
                | Progress::NativeUnlockError
        ) {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_OBSERVATION_INCOMPLETE");
        }
        let mut receipt = armed.receipt;
        receipt.finished = true;
        Ok(receipt)
    }

    pub(super) fn cancel(&mut self, target: ExactTarget) -> Result<(), &'static str> {
        if self.armed.as_ref().map(|armed| armed.target) != Some(target) {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_TARGET_MISMATCH");
        }
        self.armed.take();
        Ok(())
    }

    fn matching_armed_mut(&mut self, target: ExactTarget) -> Option<&mut ArmedLockObservation> {
        self.armed.as_mut().filter(|armed| armed.target == target)
    }
}

fn validate_expectation(
    target: ExactTarget,
    expectation: ManagedSqliteShmTestLockExpectation,
) -> Result<(), &'static str> {
    if target.0 == 0 || target.1 == 0 {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_TARGET_ZERO");
    }
    let end = expectation
        .first
        .checked_add(expectation.count)
        .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_RANGE_OVERFLOW")?;
    if expectation.count == 0 || end > SHM_LOCK_COUNT {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_RANGE_INVALID");
    }
    let low = 1u16 << expectation.first;
    let high = 1u16 << end;
    if expectation.mask != (high - low) as u8 {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_MASK_MISMATCH");
    }
    let path_matches_action = match expectation.path {
        ManagedSqliteShmTestLockPath::NativeAcquire => matches!(
            expectation.action,
            ManagedSqliteShmLockAction::LockShared | ManagedSqliteShmLockAction::LockExclusive
        ),
        ManagedSqliteShmTestLockPath::NativeRelease => matches!(
            expectation.action,
            ManagedSqliteShmLockAction::UnlockShared | ManagedSqliteShmLockAction::UnlockExclusive
        ),
        ManagedSqliteShmTestLockPath::Local => matches!(
            expectation.action,
            ManagedSqliteShmLockAction::LockShared | ManagedSqliteShmLockAction::UnlockShared
        ),
    };
    let shared_range_invalid = expectation.count != 1
        && matches!(
            expectation.action,
            ManagedSqliteShmLockAction::LockShared | ManagedSqliteShmLockAction::UnlockShared
        );
    if !path_matches_action || shared_range_invalid {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_ACTION_PATH_MISMATCH");
    }
    Ok(())
}

fn require_request(
    armed: &mut ArmedLockObservation,
    request: ManagedSqliteShmLockRequest,
) -> Result<(), &'static str> {
    if armed.receipt.expectation.action != request.action()
        || armed.receipt.expectation.first != request.first()
        || armed.receipt.expectation.count != request.count()
        || armed.receipt.expectation.mask != request.mask()
    {
        armed.invalid = true;
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_REQUEST_MISMATCH");
    }
    Ok(())
}

fn require_path(
    armed: &mut ArmedLockObservation,
    path: ManagedSqliteShmTestLockPath,
) -> Result<(), &'static str> {
    if armed.receipt.expectation.path != path {
        armed.invalid = true;
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_PATH_MISMATCH");
    }
    Ok(())
}

fn require_progress(
    armed: &mut ArmedLockObservation,
    progress: Progress,
) -> Result<(), &'static str> {
    if armed.progress != progress {
        armed.invalid = true;
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_EVENT_SEQUENCE_INVALID");
    }
    Ok(())
}

impl ManagedSqliteShmCoordinator {
    pub(super) fn begin_test_lock_action(
        &self,
        connection_id: u64,
        request: ManagedSqliteShmLockRequest,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_test_lock_event(
            connection_id,
            ManagedSqliteShmFailurePhase::RequestValidation,
            false,
            |controller, target| controller.record(target, request, LockEvent::ManagedAttempt),
        )
    }

    pub(super) fn finish_test_lock_action(
        &self,
        connection_id: u64,
        request: ManagedSqliteShmLockRequest,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let phase = lock_phase(request.action());
        self.record_test_lock_event(connection_id, phase, true, |controller, target| {
            controller.record(target, request, LockEvent::ManagedSuccess)
        })
    }

    pub(super) fn begin_test_native_lock_action(
        &self,
        connection_id: u64,
        request: ManagedSqliteShmLockRequest,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_test_lock_event(
            connection_id,
            ManagedSqliteShmFailurePhase::LockAcquire,
            false,
            |controller, target| controller.record(target, request, LockEvent::NativeLockAttempt),
        )
    }

    pub(super) fn finish_test_native_lock_action(
        &self,
        connection_id: u64,
        request: ManagedSqliteShmLockRequest,
        outcome: ManagedSqliteShmTestNativeLockOutcome,
        known_mutation: bool,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_test_lock_event(
            connection_id,
            ManagedSqliteShmFailurePhase::LockAcquire,
            known_mutation,
            |controller, target| {
                controller.record(target, request, LockEvent::NativeLockOutcome(outcome))
            },
        )
    }

    pub(super) fn begin_test_native_unlock_action(
        &self,
        connection_id: u64,
        request: ManagedSqliteShmLockRequest,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_test_lock_event(
            connection_id,
            ManagedSqliteShmFailurePhase::LockRelease,
            false,
            |controller, target| controller.record(target, request, LockEvent::NativeUnlockAttempt),
        )
    }

    pub(super) fn finish_test_native_unlock_action(
        &self,
        connection_id: u64,
        request: ManagedSqliteShmLockRequest,
        outcome: ManagedSqliteShmTestNativeUnlockOutcome,
        known_mutation: bool,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_test_lock_event(
            connection_id,
            ManagedSqliteShmFailurePhase::LockRelease,
            known_mutation,
            |controller, target| {
                controller.record(target, request, LockEvent::NativeUnlockOutcome(outcome))
            },
        )
    }

    pub(super) fn record_test_local_lock_transition(
        &self,
        connection_id: u64,
        request: ManagedSqliteShmLockRequest,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_test_lock_event(
            connection_id,
            lock_phase(request.action()),
            false,
            |controller, target| controller.record(target, request, LockEvent::LocalTransition),
        )
    }

    fn record_test_lock_event(
        &self,
        connection_id: u64,
        phase: ManagedSqliteShmFailurePhase,
        known_mutation: bool,
        record: impl FnOnce(
            &mut ManagedSqliteShmTestLockController,
            ExactTarget,
        ) -> Result<(), &'static str>,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let target = (self.generation.get(), connection_id);
        self.test_lock_runtime
            .lock()
            .map_err(|_| self.test_lock_runtime_failure(phase, known_mutation))
            .and_then(|mut controller| {
                record(&mut controller, target)
                    .map_err(|_| self.test_lock_runtime_failure(phase, known_mutation))
            })
    }

    fn test_lock_runtime_failure(
        &self,
        phase: ManagedSqliteShmFailurePhase,
        known_mutation: bool,
    ) -> ManagedSqliteShmFailure {
        self.mark_domain_terminal();
        ManagedSqliteShmFailure::poisoned_code(
            phase,
            "NODE_MANAGED_SQLITE_SHM_TEST_LOCK_RUNTIME_INVALID",
            known_mutation,
            known_mutation,
        )
    }
}

fn lock_phase(action: ManagedSqliteShmLockAction) -> ManagedSqliteShmFailurePhase {
    match action {
        ManagedSqliteShmLockAction::LockShared | ManagedSqliteShmLockAction::LockExclusive => {
            ManagedSqliteShmFailurePhase::LockAcquire
        }
        ManagedSqliteShmLockAction::UnlockShared | ManagedSqliteShmLockAction::UnlockExclusive => {
            ManagedSqliteShmFailurePhase::LockRelease
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU8;

    const TARGET: ExactTarget = (7, 11);

    fn local_expectation() -> ManagedSqliteShmTestLockExpectation {
        ManagedSqliteShmTestLockExpectation {
            action: ManagedSqliteShmLockAction::LockShared,
            first: 2,
            count: 1,
            mask: 4,
            path: ManagedSqliteShmTestLockPath::Local,
        }
    }

    fn local_request(first: u8) -> ManagedSqliteShmLockRequest {
        ManagedSqliteShmLockRequest::new(
            first,
            NonZeroU8::new(1).unwrap(),
            ManagedSqliteShmLockAction::LockShared,
        )
        .unwrap()
    }

    fn arm_local(controller: &mut ManagedSqliteShmTestLockController) {
        controller.arm(TARGET, local_expectation()).unwrap();
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
}
