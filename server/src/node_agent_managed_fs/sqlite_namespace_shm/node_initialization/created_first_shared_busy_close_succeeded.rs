//! Q18 production seam: real DMS busy after successful CreatedFirst initialization and close.

use super::super::super::{platform, PinnedManagedSqliteFile, PlatformManagedSqliteLockAttempt};
use super::super::{
    coordinator::{
        ManagedSqliteShmCoordinator, ManagedSqliteShmCoordinatorState,
        ManagedSqliteShmFileCloseCustody,
    },
    test_initialization_runtime::ManagedSqliteShmTestQ18DmsHolderLeaseV1,
    types::{
        ManagedSqliteShmFailure, ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase,
        SHM_DMS_OFFSET,
    },
};

impl ManagedSqliteShmCoordinator {
    pub(super) fn record_test_initialization_q18_dms_unlock_succeeded_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
    ) -> Result<(), ManagedSqliteShmFailure> {
        self.record_initialization_event(
            state,
            connection_id,
            ManagedSqliteShmFailurePhase::DmsExclusiveRelease,
            true,
            false,
            |controller, target| {
                controller
                    .q18_record_dms_exclusive_unlock_succeeded(target)
                    .map(|_| ())
            },
        )
    }

    pub(super) fn execute_q18_created_first_shared_busy_close_succeeded_test_v1(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        connection_id: u64,
        file: PinnedManagedSqliteFile,
    ) -> Result<PinnedManagedSqliteFile, ManagedSqliteShmFailure> {
        let target = (self.generation.get(), connection_id);
        let selected = match self.test_initialization_runtime.lock() {
            Ok(controller) => controller.q18_is_selected(target),
            Err(_) => Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_CONTROLLER_POISONED"),
        };
        let selected = selected.map_err(|code| {
            self.initialization_controller_failure(
                state,
                ManagedSqliteShmFailurePhase::DmsSharedAcquire,
                true,
                false,
                code,
            )
        })?;
        if !selected {
            return Ok(file);
        }

        let holder = match ManagedSqliteShmTestQ18DmsHolderLeaseV1::acquire(target, &file) {
            Ok(holder) => holder,
            Err(code) => {
                return Err(self.abort_q18_and_close(
                    state,
                    target,
                    file,
                    ManagedSqliteShmFailurePhase::DmsSharedAcquire,
                    code,
                    false,
                ));
            }
        };
        if let Err(code) = self.store_q18_holder(target, holder) {
            return Err(self.abort_q18_and_close(
                state,
                target,
                file,
                ManagedSqliteShmFailurePhase::DmsSharedAcquire,
                code,
                false,
            ));
        }

        let target_shared =
            platform::try_lock_sqlite_byte_range(&file.file, SHM_DMS_OFFSET, 1, false);
        match target_shared {
            Ok(PlatformManagedSqliteLockAttempt::Contended) => {
                if let Err(code) = self.record_q18_target_shared_contended(target) {
                    return Err(self.abort_q18_and_close(
                        state,
                        target,
                        file,
                        ManagedSqliteShmFailurePhase::DmsSharedAcquire,
                        code,
                        false,
                    ));
                }
            }
            Ok(PlatformManagedSqliteLockAttempt::Acquired) => {
                let target_unlock_failed =
                    platform::unlock_sqlite_byte_range(&file.file, SHM_DMS_OFFSET, 1).is_err();
                return Err(self.abort_q18_and_close(
                    state,
                    target,
                    file,
                    ManagedSqliteShmFailurePhase::DmsSharedAcquire,
                    "NODE_MANAGED_SQLITE_SHM_TEST_Q18_TARGET_SHARED_NOT_CONTENDED",
                    target_unlock_failed,
                ));
            }
            Err(_) => {
                return Err(self.abort_q18_and_close(
                    state,
                    target,
                    file,
                    ManagedSqliteShmFailurePhase::DmsSharedAcquire,
                    "NODE_MANAGED_SQLITE_SHM_TEST_Q18_TARGET_SHARED_ERROR",
                    false,
                ));
            }
        }

        if let Err(code) = self.record_q18_target_close_attempt(target) {
            return Err(self.abort_q18_and_close(
                state,
                target,
                file,
                ManagedSqliteShmFailurePhase::FileClose,
                code,
                false,
            ));
        }
        match file.close() {
            Ok(receipt) => {
                if let Err(code) = self.record_q18_target_close_succeeded(target, receipt.kind()) {
                    let _ = self.release_q18_holder(target);
                    self.mark_poisoned(
                        state,
                        ManagedSqliteShmFailurePhase::FileClose,
                        true,
                        false,
                    );
                    return Err(ManagedSqliteShmFailure::poisoned_code(
                        ManagedSqliteShmFailurePhase::FileClose,
                        code,
                        true,
                        false,
                    ));
                }
            }
            Err(close_failure) => {
                let report = super::pinned_close_report(&close_failure);
                state
                    .quarantined_file_close
                    .push(ManagedSqliteShmFileCloseCustody::Pinned(close_failure));
                let _ = self.release_q18_holder(target);
                self.mark_poisoned(
                    state,
                    ManagedSqliteShmFailurePhase::FileClose,
                    true,
                    false,
                );
                return Err(ManagedSqliteShmFailure::poisoned(
                    ManagedSqliteShmFailurePhase::FileClose,
                    report,
                    true,
                    false,
                ));
            }
        }

        Err(ManagedSqliteShmFailure::code(
            ManagedSqliteShmFailurePhase::DmsSharedAcquire,
            ManagedSqliteShmFailureClass::BusyAfterKnownMutation,
            "NODE_MANAGED_SQLITE_SHM_DMS_BUSY",
        ))
    }

    fn store_q18_holder(
        &self,
        target: (u64, u64),
        holder: ManagedSqliteShmTestQ18DmsHolderLeaseV1,
    ) -> Result<(), &'static str> {
        match self.test_initialization_runtime.lock() {
            Ok(mut controller) => controller.q18_store_holder(target, holder),
            Err(poisoned) => {
                let release = holder.release_explicit();
                let mut controller = poisoned.into_inner();
                let _ = controller.q18_abort_and_release(target);
                release.map_err(|code| code)?;
                Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_CONTROLLER_POISONED")
            }
        }
    }

    fn record_q18_target_shared_contended(
        &self,
        target: (u64, u64),
    ) -> Result<(), &'static str> {
        self.test_initialization_runtime
            .lock()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_Q18_CONTROLLER_POISONED")?
            .q18_record_target_shared_contended(target)
    }

    fn record_q18_target_close_attempt(
        &self,
        target: (u64, u64),
    ) -> Result<(), &'static str> {
        self.test_initialization_runtime
            .lock()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_Q18_CONTROLLER_POISONED")?
            .q18_record_target_close_attempt(target)
    }

    fn record_q18_target_close_succeeded(
        &self,
        target: (u64, u64),
        kind: crate::node_agent_managed_fs::ManagedSqliteFileKind,
    ) -> Result<(), &'static str> {
        self.test_initialization_runtime
            .lock()
            .map_err(|_| "NODE_MANAGED_SQLITE_SHM_TEST_Q18_CONTROLLER_POISONED")?
            .q18_record_target_close_succeeded(target, kind)
    }

    fn release_q18_holder(&self, target: (u64, u64)) -> Result<(), &'static str> {
        match self.test_initialization_runtime.lock() {
            Ok(mut controller) => controller.q18_abort_and_release(target),
            Err(poisoned) => {
                let mut controller = poisoned.into_inner();
                let _ = controller.q18_abort_and_release(target);
                Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_CONTROLLER_POISONED")
            }
        }
    }

    fn abort_q18_and_close(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        target: (u64, u64),
        file: PinnedManagedSqliteFile,
        phase: ManagedSqliteShmFailurePhase,
        code: &'static str,
        lock_outcome_uncertain: bool,
    ) -> ManagedSqliteShmFailure {
        let release_failed = self.release_q18_holder(target).is_err();
        let uncertain = lock_outcome_uncertain || release_failed;
        self.mark_poisoned(state, phase, true, uncertain);
        let failure = ManagedSqliteShmFailure::poisoned_code(phase, code, true, uncertain);
        self.close_failed_open_file(state, file, failure)
    }
}
