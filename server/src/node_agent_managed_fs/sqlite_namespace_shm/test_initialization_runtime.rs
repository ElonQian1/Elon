//! Exact-target controlled evidence for initialization failures reached by a managed Lock request.

use std::fs::File;

use super::{
    coordinator::{ManagedSqliteShmCoordinator, ManagedSqliteShmCoordinatorState},
    test_faults::ManagedSqliteShmTestTargetObserver,
    test_lock_runtime::{ManagedSqliteShmTestLockExpectation, ManagedSqliteShmTestLockPath},
    test_snapshot::ManagedSqliteShmTestDmsCustody,
    types::{
        ManagedSqliteShmFailure, ManagedSqliteShmFailurePhase, ManagedSqliteShmLockRequest,
        SHM_DMS_OFFSET,
    },
};

#[path = "test_initialization_runtime/controller.rs"]
mod controller;
#[path = "test_initialization_runtime/created_first_truncate_error_release_failed.rs"]
mod created_first_truncate_error_release_failed;
#[path = "test_initialization_runtime/created_first_truncate_error_release_succeeded.rs"]
mod created_first_truncate_error_release_succeeded;
#[path = "test_initialization_runtime/existing_first_truncate_error_release_failed.rs"]
mod existing_first_truncate_error_release_failed;
#[path = "test_initialization_runtime/existing_first_truncate_error_release_succeeded.rs"]
mod existing_first_truncate_error_release_succeeded;
#[path = "test_initialization_runtime/model.rs"]
mod model;

pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) use controller::ManagedSqliteShmTestInitializationControllerV1;
use controller::{ColdPrestateV1, TerminalStateV1};
pub(crate) use model::{
    ManagedSqliteShmTestInitializationEvidenceV1, ManagedSqliteShmTestInitializationExpectationV1,
    ManagedSqliteShmTestInitializationFailureV1,
    ManagedSqliteShmTestInitializationNativeObservationV1,
    ManagedSqliteShmTestInitializationNativeReceiptV1, ManagedSqliteShmTestInitializationReceiptV1,
};

const CONTROLLER_POISONED: &str = "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_CONTROLLER_POISONED";

impl ManagedSqliteShmTestTargetObserver {
    pub(crate) fn begin_lock_initialization_failure_observation_v1(
        &self,
        expectation: ManagedSqliteShmTestInitializationExpectationV1,
    ) -> Result<(), &'static str> {
        let snapshot = self
            .snapshot()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_SNAPSHOT_FAILED")?;
        let cold = ColdPrestateV1 {
            target_attached: snapshot.target_attached,
            shm_connections: snapshot.topology.shm_connections,
            node_present: snapshot.topology.node_present,
            shm_file_present: snapshot.topology.shm_file_present,
            poisoned: snapshot.topology.poisoned,
            domain_terminal: snapshot.topology.domain_terminal,
            shared_mask: snapshot.shared_mask,
            exclusive_mask: snapshot.exclusive_mask,
        };
        let (coordinator, target) = self.initialization_authority_v1();
        coordinator
            .test_initialization_runtime
            .lock()
            .map_err(|_| CONTROLLER_POISONED)?
            .arm(target, expectation, cold)?;

        let lock_expectation = ManagedSqliteShmTestLockExpectation {
            action: expectation.action,
            first: expectation.first,
            count: expectation.count,
            mask: expectation.mask,
            path: ManagedSqliteShmTestLockPath::InitializationFailure,
        };
        let lock_result = match coordinator.test_lock_runtime.lock() {
            Ok(mut runtime) => runtime.arm(target, lock_expectation),
            Err(_) => Err("NODE_MANAGED_SQLITE_SHM_TEST_LOCK_RUNTIME_POISONED"),
        };
        if let Err(error) = lock_result {
            coordinator
                .test_initialization_runtime
                .lock()
                .map_err(|_| CONTROLLER_POISONED)?
                .cancel_after_arm(target)?;
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn finish_lock_initialization_failure_observation_v1(
        &self,
    ) -> Result<ManagedSqliteShmTestInitializationReceiptV1, &'static str> {
        let snapshot = self
            .snapshot()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_SNAPSHOT_FAILED")?;
        let terminal = TerminalStateV1 {
            target_attached: snapshot.target_attached,
            shm_connections: snapshot.topology.shm_connections,
            node_present: snapshot.topology.node_present,
            shm_file_present: snapshot.topology.shm_file_present,
            dms_exclusive_outcome_uncertain: snapshot.topology.dms
                == ManagedSqliteShmTestDmsCustody::ExclusiveOutcomeUncertain,
            dms_released: snapshot.topology.dms == ManagedSqliteShmTestDmsCustody::Released,
            poisoned: snapshot.topology.poisoned,
            mutation_may_have_occurred: snapshot.topology.mutation_may_have_occurred,
            lock_outcome_uncertain: snapshot.topology.lock_outcome_uncertain,
            domain_terminal: snapshot.topology.domain_terminal,
            shared_mask: snapshot.shared_mask,
            exclusive_mask: snapshot.exclusive_mask,
        };
        let (coordinator, target) = self.initialization_authority_v1();
        let requested_lock = coordinator
            .test_lock_runtime
            .lock()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_LOCK_RUNTIME_POISONED")?
            .finish_initialization_failure_after_managed_attempt(target)?;
        coordinator
            .test_initialization_runtime
            .lock()
            .map_err(|_| CONTROLLER_POISONED)?
            .finish(target, terminal, requested_lock)
    }
}

impl ManagedSqliteShmCoordinator {
    pub(super) fn record_test_initialization_request_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
        request: ManagedSqliteShmLockRequest,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_initialization_event(
            state,
            connection_id,
            ManagedSqliteShmFailurePhase::RequestValidation,
            false,
            false,
            |controller, target| controller.record_request(target, request).map(|_| ()),
        )
    }

    pub(super) fn record_test_initialization_open_attempt_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_initialization_event(
            state,
            connection_id,
            ManagedSqliteShmFailurePhase::ExactSiblingOpen,
            false,
            false,
            |controller, target| controller.record_open_attempt(target).map(|_| ()),
        )
    }

    pub(super) fn record_test_initialization_open_created_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
        created: bool,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_initialization_event(
            state,
            connection_id,
            ManagedSqliteShmFailurePhase::ExactSiblingOpen,
            created,
            false,
            |controller, target| controller.record_open_created(target, created).map(|_| ()),
        )
    }

    pub(super) fn record_test_initialization_dms_lock_attempt_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_initialization_event(
            state,
            connection_id,
            ManagedSqliteShmFailurePhase::DmsExclusiveAcquire,
            true,
            false,
            |controller, target| {
                controller
                    .record_dms_exclusive_lock_attempt(target)
                    .map(|_| ())
            },
        )
    }

    pub(super) fn record_test_initialization_dms_acquired_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_initialization_event(
            state,
            connection_id,
            ManagedSqliteShmFailurePhase::DmsExclusiveAcquire,
            true,
            false,
            |controller, target| controller.record_dms_exclusive_acquired(target).map(|_| ()),
        )
    }

    pub(super) fn record_test_initialization_truncate_attempt_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_initialization_event(
            state,
            connection_id,
            ManagedSqliteShmFailurePhase::DmsTruncate,
            true,
            false,
            |controller, target| controller.record_truncate_attempt(target).map(|_| ()),
        )
    }

    pub(super) fn record_test_initialization_truncated_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_initialization_event(
            state,
            connection_id,
            ManagedSqliteShmFailurePhase::DmsTruncate,
            true,
            false,
            |controller, target| controller.record_truncate_success(target).map(|_| ()),
        )
    }

    pub(super) fn reject_test_initialization_path_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
        phase: ManagedSqliteShmFailurePhase,
        mutation_may_have_occurred: bool,
        lock_outcome_uncertain: bool,
        code: &'static str,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_initialization_event(
            state,
            connection_id,
            phase,
            mutation_may_have_occurred,
            lock_outcome_uncertain,
            |controller, target| controller.reject_if_armed(target, code),
        )
    }

    pub(super) fn execute_test_initialization_dms_unlock_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
        file: &File,
    ) -> Result<Option<std::io::Error>, ManagedSqliteShmFailure> {
        let target = (self.generation.get(), connection_id);
        let mut controller = match self.test_initialization_runtime.lock() {
            Ok(controller) => controller,
            Err(_) => {
                return Err(self.initialization_controller_failure(
                    state,
                    ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
                    true,
                    false,
                    CONTROLLER_POISONED,
                ));
            }
        };
        let armed = match controller.begin_dms_exclusive_unlock(target) {
            Ok(armed) => armed,
            Err(code) => {
                drop(controller);
                return Err(self.initialization_controller_failure(
                    state,
                    ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
                    true,
                    false,
                    code,
                ));
            }
        };
        if !armed {
            return Ok(None);
        }

        let platform_receipt =
            super::super::platform::unlock_sqlite_byte_range_outcome_uncertain_for_initialization_test(
                file,
                SHM_DMS_OFFSET,
                1,
            );
        let native = ManagedSqliteShmTestInitializationNativeReceiptV1 {
            observation:
                ManagedSqliteShmTestInitializationNativeObservationV1::ReturnReceiptUnavailable,
            offset: platform_receipt.offset,
            length: platform_receipt.length,
            exact_call_occurrence: platform_receipt.exact_call_occurrence.get(),
        };
        if let Err(code) = controller.record_return_receipt_unavailable(target, native) {
            drop(controller);
            return Err(self.initialization_controller_failure(
                state,
                ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
                true,
                true,
                code,
            ));
        }
        Ok(Some(platform_receipt.error))
    }

    pub(super) fn record_test_initialization_poisoned_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_initialization_event(
            state,
            connection_id,
            ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
            true,
            true,
            |controller, target| controller.record_poisoned(target),
        )
    }

    fn record_initialization_event(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
        phase: ManagedSqliteShmFailurePhase,
        mutation_may_have_occurred: bool,
        lock_outcome_uncertain: bool,
        record: impl FnOnce(
            &mut ManagedSqliteShmTestInitializationControllerV1,
            (u64, u64),
        ) -> Result<(), &'static str>,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let target = (self.generation.get(), connection_id);
        let result = self
            .test_initialization_runtime
            .lock()
            .map_err(|_| CONTROLLER_POISONED)
            .and_then(|mut controller| record(&mut controller, target));
        result.map_err(|code| {
            self.initialization_controller_failure(
                state,
                phase,
                mutation_may_have_occurred,
                lock_outcome_uncertain,
                code,
            )
        })
    }

    fn initialization_controller_failure(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        phase: ManagedSqliteShmFailurePhase,
        mutation_may_have_occurred: bool,
        lock_outcome_uncertain: bool,
        code: &'static str,
    ) -> ManagedSqliteShmFailure {
        self.mark_poisoned(
            state,
            phase,
            mutation_may_have_occurred,
            lock_outcome_uncertain,
        );
        ManagedSqliteShmFailure::poisoned_code(
            phase,
            code,
            mutation_may_have_occurred,
            lock_outcome_uncertain,
        )
    }
}

#[cfg(test)]
#[path = "test_initialization_runtime/tests.rs"]
mod tests;
