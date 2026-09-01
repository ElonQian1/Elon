//! Independent Q19 controller for ExistingFirst real DMS contention and target-close success.

use crate::node_agent_managed_fs::ManagedSqliteFileKind;

use super::super::super::{
    test_lock_runtime::ManagedSqliteShmTestLockReceipt,
    test_snapshot::ManagedSqliteShmTestTargetSnapshot, types::ManagedSqliteShmLockRequest,
};
use super::super::{
    existing_first_shared_busy_close_succeeded::ManagedSqliteShmTestQ19DmsHolderLeaseV1,
    model::{
        ManagedSqliteShmTestExistingFirstSharedBusyCloseSucceededReceiptV1,
        ManagedSqliteShmTestInitializationExpectationV1,
        ManagedSqliteShmTestInitializationFailureV1,
    },
};
use super::{ColdPrestateV1, ExactTarget, ManagedSqliteShmTestInitializationControllerV1};

#[path = "existing_first_shared_busy_close_succeeded/state.rs"]
mod state;
#[path = "existing_first_shared_busy_close_succeeded/validation.rs"]
mod validation;

use state::{ArmedQ19ObservationV1, EventCounts, Stage};

#[derive(Default)]
pub(super) struct ExistingFirstSharedBusyCloseSucceededControllerV1 {
    active: Option<ArmedQ19ObservationV1>,
}

impl ExistingFirstSharedBusyCloseSucceededControllerV1 {
    pub(super) const fn is_armed(&self) -> bool {
        self.active.is_some()
    }

    pub(super) fn is_selected(&self, target: ExactTarget) -> Result<bool, &'static str> {
        let Some(active) = self.active.as_ref() else {
            return Ok(false);
        };
        active.validate_target(target)?;
        if active.violation.is_some() {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q19_PREVIOUS_VIOLATION");
        }
        Ok(true)
    }

    pub(super) fn arm(
        &mut self,
        target: ExactTarget,
        expectation: ManagedSqliteShmTestInitializationExpectationV1,
        cold: ColdPrestateV1,
    ) -> Result<(), &'static str> {
        if self.active.is_some() {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q19_ALREADY_ARMED");
        }
        if expectation.case_v1
            != ManagedSqliteShmTestInitializationFailureV1::ExistingFirstSharedBusyCloseSucceeded
        {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q19_CASE_MISMATCH");
        }
        self.active = Some(ArmedQ19ObservationV1::new(target, expectation, cold));
        Ok(())
    }

    pub(super) fn cancel_after_arm(&mut self, target: ExactTarget) -> Result<(), &'static str> {
        let active = self.require(target)?;
        if active.stage != Stage::Armed || active.pending != 1 || active.consumed {
            return active.fail("NODE_MANAGED_SQLITE_SHM_TEST_Q19_CANCEL_AFTER_PROGRESS");
        }
        self.active.take();
        Ok(())
    }

    pub(super) fn record_request(
        &mut self,
        target: ExactTarget,
        request: ManagedSqliteShmLockRequest,
    ) -> Result<bool, &'static str> {
        let active = self.require(target)?;
        if request.action() != active.expectation.action
            || request.first() != active.expectation.first
            || request.count() != active.expectation.count
            || request.mask() != active.expectation.mask
        {
            return active.fail("NODE_MANAGED_SQLITE_SHM_TEST_Q19_REQUEST_MISMATCH");
        }
        active.advance(
            Stage::Armed,
            Stage::Requested,
            |counts| &mut counts.request,
            "NODE_MANAGED_SQLITE_SHM_TEST_Q19_REQUEST_SEQUENCE_INVALID",
        )?;
        Ok(true)
    }

    pub(super) fn record_open_attempt(
        &mut self,
        target: ExactTarget,
    ) -> Result<bool, &'static str> {
        self.advance(
            target,
            Stage::Requested,
            Stage::OpenAttempted,
            |counts| &mut counts.open_attempt,
            "NODE_MANAGED_SQLITE_SHM_TEST_Q19_OPEN_ATTEMPT_SEQUENCE_INVALID",
        )
    }

    pub(super) fn record_open_created(
        &mut self,
        target: ExactTarget,
        created: bool,
    ) -> Result<bool, &'static str> {
        let active = self.require(target)?;
        if created {
            return active.fail("NODE_MANAGED_SQLITE_SHM_TEST_Q19_NOT_EXISTING_FIRST");
        }
        active.advance(
            Stage::OpenAttempted,
            Stage::OpenObservedExisting,
            |counts| &mut counts.open_existing,
            "NODE_MANAGED_SQLITE_SHM_TEST_Q19_OPEN_COMPLETION_SEQUENCE_INVALID",
        )?;
        Ok(true)
    }

    pub(super) fn record_dms_exclusive_lock_attempt(
        &mut self,
        target: ExactTarget,
    ) -> Result<bool, &'static str> {
        self.advance(
            target,
            Stage::OpenObservedExisting,
            Stage::DmsExclusiveLockAttempted,
            |counts| &mut counts.exclusive_lock_attempt,
            "NODE_MANAGED_SQLITE_SHM_TEST_Q19_EXCLUSIVE_ATTEMPT_SEQUENCE_INVALID",
        )
    }

    pub(super) fn record_dms_exclusive_acquired(
        &mut self,
        target: ExactTarget,
    ) -> Result<bool, &'static str> {
        self.advance(
            target,
            Stage::DmsExclusiveLockAttempted,
            Stage::DmsExclusiveAcquired,
            |counts| &mut counts.exclusive_lock_acquired,
            "NODE_MANAGED_SQLITE_SHM_TEST_Q19_EXCLUSIVE_OUTCOME_SEQUENCE_INVALID",
        )
    }

    pub(super) fn record_truncate_attempt(
        &mut self,
        target: ExactTarget,
    ) -> Result<bool, &'static str> {
        self.advance(
            target,
            Stage::DmsExclusiveAcquired,
            Stage::TruncateAttempted,
            |counts| &mut counts.truncate_attempt,
            "NODE_MANAGED_SQLITE_SHM_TEST_Q19_TRUNCATE_ATTEMPT_SEQUENCE_INVALID",
        )
    }

    pub(super) fn record_truncate_success(
        &mut self,
        target: ExactTarget,
    ) -> Result<bool, &'static str> {
        self.advance(
            target,
            Stage::TruncateAttempted,
            Stage::Truncated,
            |counts| &mut counts.truncate_success,
            "NODE_MANAGED_SQLITE_SHM_TEST_Q19_TRUNCATE_OUTCOME_SEQUENCE_INVALID",
        )
    }

    pub(super) fn begin_dms_exclusive_unlock(
        &mut self,
        target: ExactTarget,
    ) -> Result<bool, &'static str> {
        self.advance(
            target,
            Stage::Truncated,
            Stage::DmsExclusiveUnlockAttempted,
            |counts| &mut counts.exclusive_unlock_attempt,
            "NODE_MANAGED_SQLITE_SHM_TEST_Q19_EXCLUSIVE_UNLOCK_SEQUENCE_INVALID",
        )
    }

    pub(super) fn record_dms_exclusive_unlock_succeeded(
        &mut self,
        target: ExactTarget,
    ) -> Result<bool, &'static str> {
        self.advance(
            target,
            Stage::DmsExclusiveUnlockAttempted,
            Stage::DmsExclusiveUnlockSucceeded,
            |counts| &mut counts.exclusive_unlock_success,
            "NODE_MANAGED_SQLITE_SHM_TEST_Q19_EXCLUSIVE_UNLOCK_OUTCOME_INVALID",
        )
    }

    pub(super) fn store_holder(
        &mut self,
        target: ExactTarget,
        holder: ManagedSqliteShmTestQ19DmsHolderLeaseV1,
    ) -> Result<(), &'static str> {
        let active = match self.require(target) {
            Ok(active) => active,
            Err(code) => {
                holder.release_explicit()?;
                return Err(code);
            }
        };
        if active.stage != Stage::DmsExclusiveUnlockSucceeded || active.holder.is_some() {
            let code = "NODE_MANAGED_SQLITE_SHM_TEST_Q19_HOLDER_SEQUENCE_INVALID";
            active.violation = Some(code);
            holder.release_explicit()?;
            return Err(code);
        }
        active.holder = Some(holder);
        active.stage = Stage::HolderAcquired;
        Ok(())
    }

    pub(super) fn record_target_shared_contended(
        &mut self,
        target: ExactTarget,
    ) -> Result<(), &'static str> {
        let active = self.require(target)?;
        if active.stage != Stage::HolderAcquired || active.counts.target_shared_attempt != 0 {
            return active.fail("NODE_MANAGED_SQLITE_SHM_TEST_Q19_SHARED_SEQUENCE_INVALID");
        }
        active
            .holder
            .as_mut()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_Q19_HOLDER_MISSING")?
            .mark_held_during_target_shared();
        active.counts.target_shared_attempt = 1;
        active.counts.target_shared_contended = 1;
        active.stage = Stage::TargetSharedContended;
        Ok(())
    }

    pub(super) fn record_target_close_attempt(
        &mut self,
        target: ExactTarget,
    ) -> Result<(), &'static str> {
        let active = self.require(target)?;
        if active.stage != Stage::TargetSharedContended || active.counts.target_close_attempt != 0 {
            return active.fail("NODE_MANAGED_SQLITE_SHM_TEST_Q19_CLOSE_ATTEMPT_SEQUENCE_INVALID");
        }
        active
            .holder
            .as_mut()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_Q19_HOLDER_MISSING")?
            .mark_held_during_target_close();
        active.counts.target_close_attempt = 1;
        active.stage = Stage::TargetCloseAttempted;
        Ok(())
    }

    pub(super) fn record_target_close_succeeded(
        &mut self,
        target: ExactTarget,
        kind: ManagedSqliteFileKind,
    ) -> Result<(), &'static str> {
        let active = self.require(target)?;
        if kind != ManagedSqliteFileKind::Shm {
            return active.fail("NODE_MANAGED_SQLITE_SHM_TEST_Q19_CLOSE_KIND_INVALID");
        }
        active.advance(
            Stage::TargetCloseAttempted,
            Stage::TargetCloseSucceeded,
            |counts| &mut counts.target_close_success,
            "NODE_MANAGED_SQLITE_SHM_TEST_Q19_CLOSE_OUTCOME_SEQUENCE_INVALID",
        )?;
        active.close_kind = Some(kind);
        active.pending = 0;
        active.consumed = true;
        Ok(())
    }

    pub(super) fn abort_and_release(&mut self, target: ExactTarget) -> Result<(), &'static str> {
        self.active
            .as_ref()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_Q19_NOT_ARMED")?
            .validate_target(target)?;
        let mut active = self.active.take().expect("validated Q19 active state");
        if let Some(holder) = active.holder.take() {
            holder.release_explicit().map(|_| ())?;
        }
        Ok(())
    }

    pub(super) fn finish(
        &mut self,
        target: ExactTarget,
        terminal: ManagedSqliteShmTestTargetSnapshot,
        requested_lock: ManagedSqliteShmTestLockReceipt,
    ) -> Result<ManagedSqliteShmTestExistingFirstSharedBusyCloseSucceededReceiptV1, &'static str>
    {
        let mut active = self
            .active
            .take()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_Q19_NOT_ARMED")?;
        let holder_values = active
            .holder
            .take()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_Q19_HOLDER_MISSING")?
            .release_explicit()?;
        active.validate_target(target)?;
        validation::validate_completion(&active)?;
        validation::validate_terminal(terminal)?;
        validation::validate_requested_lock(&active, requested_lock)?;
        validation::validate_holder_values(target, holder_values)?;
        let initialization_values = validation::initialization_values(&active);
        Ok(
            ManagedSqliteShmTestExistingFirstSharedBusyCloseSucceededReceiptV1::new(
                active.expectation,
                requested_lock,
                initialization_values,
                holder_values,
            ),
        )
    }

    fn advance(
        &mut self,
        target: ExactTarget,
        required: Stage,
        next: Stage,
        counter: impl FnOnce(&mut EventCounts) -> &mut u8,
        code: &'static str,
    ) -> Result<bool, &'static str> {
        self.require(target)?
            .advance(required, next, counter, code)?;
        Ok(true)
    }

    fn require(&mut self, target: ExactTarget) -> Result<&mut ArmedQ19ObservationV1, &'static str> {
        let active = self
            .active
            .as_mut()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_Q19_NOT_ARMED")?;
        if let Err(code) = active.validate_target(target) {
            active.violation = Some(code);
            return Err(code);
        }
        if active.violation.is_some() {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q19_PREVIOUS_VIOLATION");
        }
        Ok(active)
    }
}

impl ManagedSqliteShmTestInitializationControllerV1 {
    pub(super) fn q19_is_selected(&self, target: ExactTarget) -> Result<bool, &'static str> {
        self.q19.is_selected(target)
    }

    pub(super) fn q19_record_dms_exclusive_unlock_succeeded(
        &mut self,
        target: ExactTarget,
    ) -> Result<bool, &'static str> {
        if !self.q19.is_armed() {
            return Ok(false);
        }
        self.q19.record_dms_exclusive_unlock_succeeded(target)
    }

    pub(super) fn q19_store_holder(
        &mut self,
        target: ExactTarget,
        holder: ManagedSqliteShmTestQ19DmsHolderLeaseV1,
    ) -> Result<(), &'static str> {
        self.q19.store_holder(target, holder)
    }

    pub(super) fn q19_record_target_shared_contended(
        &mut self,
        target: ExactTarget,
    ) -> Result<(), &'static str> {
        self.q19.record_target_shared_contended(target)
    }

    pub(super) fn q19_record_target_close_attempt(
        &mut self,
        target: ExactTarget,
    ) -> Result<(), &'static str> {
        self.q19.record_target_close_attempt(target)
    }

    pub(super) fn q19_record_target_close_succeeded(
        &mut self,
        target: ExactTarget,
        kind: ManagedSqliteFileKind,
    ) -> Result<(), &'static str> {
        self.q19.record_target_close_succeeded(target, kind)
    }

    pub(super) fn q19_abort_and_release(
        &mut self,
        target: ExactTarget,
    ) -> Result<(), &'static str> {
        self.q19.abort_and_release(target)
    }

    pub(super) fn finish_q19(
        &mut self,
        target: ExactTarget,
        terminal: ManagedSqliteShmTestTargetSnapshot,
        requested_lock: ManagedSqliteShmTestLockReceipt,
    ) -> Result<ManagedSqliteShmTestExistingFirstSharedBusyCloseSucceededReceiptV1, &'static str>
    {
        self.q19.finish(target, terminal, requested_lock)
    }
}
