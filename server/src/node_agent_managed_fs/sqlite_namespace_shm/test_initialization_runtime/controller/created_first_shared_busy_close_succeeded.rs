//! Independent Q18 controller for real DMS contention and exact target-close success.

use std::thread::ThreadId;

use crate::node_agent_managed_fs::ManagedSqliteFileKind;

use super::super::super::{
    test_lock_runtime::{ManagedSqliteShmTestLockPath, ManagedSqliteShmTestLockReceipt},
    test_snapshot::{ManagedSqliteShmTestDmsCustody, ManagedSqliteShmTestTargetSnapshot},
    types::{ManagedSqliteShmLockRequest, SHM_DMS_OFFSET},
};
use super::super::{
    created_first_shared_busy_close_succeeded::ManagedSqliteShmTestQ18DmsHolderLeaseV1,
    model::{
        lock_action_tag, ManagedSqliteShmTestCreatedFirstSharedBusyCloseSucceededReceiptV1,
        ManagedSqliteShmTestInitializationExpectationV1,
        ManagedSqliteShmTestInitializationFailureV1,
    },
};
use super::{ColdPrestateV1, ExactTarget, ManagedSqliteShmTestInitializationControllerV1};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Armed,
    Requested,
    OpenAttempted,
    OpenCreated,
    DmsExclusiveLockAttempted,
    DmsExclusiveAcquired,
    TruncateAttempted,
    Truncated,
    DmsExclusiveUnlockAttempted,
    DmsExclusiveUnlockSucceeded,
    HolderAcquired,
    TargetSharedContended,
    TargetCloseAttempted,
    TargetCloseSucceeded,
}

#[derive(Default)]
struct EventCounts {
    request: u8,
    open_attempt: u8,
    open_created: u8,
    exclusive_lock_attempt: u8,
    exclusive_lock_acquired: u8,
    truncate_attempt: u8,
    truncate_success: u8,
    exclusive_unlock_attempt: u8,
    exclusive_unlock_success: u8,
    target_shared_attempt: u8,
    target_shared_acquired: u8,
    target_shared_contended: u8,
    target_shared_errors: u8,
    target_close_attempt: u8,
    target_close_success: u8,
    target_close_failure: u8,
}

struct ArmedQ18ObservationV1 {
    target: ExactTarget,
    owner_thread: ThreadId,
    expectation: ManagedSqliteShmTestInitializationExpectationV1,
    cold: ColdPrestateV1,
    stage: Stage,
    counts: EventCounts,
    holder: Option<ManagedSqliteShmTestQ18DmsHolderLeaseV1>,
    close_kind: Option<ManagedSqliteFileKind>,
    pending: u8,
    consumed: bool,
    violation: Option<&'static str>,
}

#[derive(Default)]
pub(super) struct CreatedFirstSharedBusyCloseSucceededControllerV1 {
    active: Option<ArmedQ18ObservationV1>,
}

impl CreatedFirstSharedBusyCloseSucceededControllerV1 {
    pub(super) const fn is_armed(&self) -> bool {
        self.active.is_some()
    }

    pub(super) fn is_selected(&self, target: ExactTarget) -> Result<bool, &'static str> {
        let Some(active) = self.active.as_ref() else {
            return Ok(false);
        };
        validate_target(active, target)?;
        if active.violation.is_some() {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_PREVIOUS_VIOLATION");
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
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_ALREADY_ARMED");
        }
        if expectation.case_v1
            != ManagedSqliteShmTestInitializationFailureV1::CreatedFirstSharedBusyCloseSucceeded
        {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_CASE_MISMATCH");
        }
        self.active = Some(ArmedQ18ObservationV1 {
            target,
            owner_thread: std::thread::current().id(),
            expectation,
            cold,
            stage: Stage::Armed,
            counts: EventCounts::default(),
            holder: None,
            close_kind: None,
            pending: 1,
            consumed: false,
            violation: None,
        });
        Ok(())
    }

    pub(super) fn cancel_after_arm(&mut self, target: ExactTarget) -> Result<(), &'static str> {
        let active = self.require(target)?;
        if active.stage != Stage::Armed || active.pending != 1 || active.consumed {
            return fail(
                active,
                "NODE_MANAGED_SQLITE_SHM_TEST_Q18_CANCEL_AFTER_PROGRESS",
            );
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
            return fail(active, "NODE_MANAGED_SQLITE_SHM_TEST_Q18_REQUEST_MISMATCH");
        }
        advance(
            active,
            Stage::Armed,
            Stage::Requested,
            |counts| &mut counts.request,
            "NODE_MANAGED_SQLITE_SHM_TEST_Q18_REQUEST_SEQUENCE_INVALID",
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
            "NODE_MANAGED_SQLITE_SHM_TEST_Q18_OPEN_ATTEMPT_SEQUENCE_INVALID",
        )
    }

    pub(super) fn record_open_created(
        &mut self,
        target: ExactTarget,
        created: bool,
    ) -> Result<bool, &'static str> {
        let active = self.require(target)?;
        if !created {
            return fail(active, "NODE_MANAGED_SQLITE_SHM_TEST_Q18_NOT_CREATED_FIRST");
        }
        advance(
            active,
            Stage::OpenAttempted,
            Stage::OpenCreated,
            |counts| &mut counts.open_created,
            "NODE_MANAGED_SQLITE_SHM_TEST_Q18_OPEN_COMPLETION_SEQUENCE_INVALID",
        )?;
        Ok(true)
    }

    pub(super) fn record_dms_exclusive_lock_attempt(
        &mut self,
        target: ExactTarget,
    ) -> Result<bool, &'static str> {
        self.advance(
            target,
            Stage::OpenCreated,
            Stage::DmsExclusiveLockAttempted,
            |counts| &mut counts.exclusive_lock_attempt,
            "NODE_MANAGED_SQLITE_SHM_TEST_Q18_EXCLUSIVE_ATTEMPT_SEQUENCE_INVALID",
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
            "NODE_MANAGED_SQLITE_SHM_TEST_Q18_EXCLUSIVE_OUTCOME_SEQUENCE_INVALID",
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
            "NODE_MANAGED_SQLITE_SHM_TEST_Q18_TRUNCATE_ATTEMPT_SEQUENCE_INVALID",
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
            "NODE_MANAGED_SQLITE_SHM_TEST_Q18_TRUNCATE_OUTCOME_SEQUENCE_INVALID",
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
            "NODE_MANAGED_SQLITE_SHM_TEST_Q18_EXCLUSIVE_UNLOCK_SEQUENCE_INVALID",
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
            "NODE_MANAGED_SQLITE_SHM_TEST_Q18_EXCLUSIVE_UNLOCK_OUTCOME_INVALID",
        )
    }

    pub(super) fn store_holder(
        &mut self,
        target: ExactTarget,
        holder: ManagedSqliteShmTestQ18DmsHolderLeaseV1,
    ) -> Result<(), &'static str> {
        let active = match self.require(target) {
            Ok(active) => active,
            Err(code) => {
                holder.release_explicit()?;
                return Err(code);
            }
        };
        if active.stage != Stage::DmsExclusiveUnlockSucceeded || active.holder.is_some() {
            let code = "NODE_MANAGED_SQLITE_SHM_TEST_Q18_HOLDER_SEQUENCE_INVALID";
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
            return fail(
                active,
                "NODE_MANAGED_SQLITE_SHM_TEST_Q18_SHARED_SEQUENCE_INVALID",
            );
        }
        active
            .holder
            .as_mut()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_Q18_HOLDER_MISSING")?
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
            return fail(
                active,
                "NODE_MANAGED_SQLITE_SHM_TEST_Q18_CLOSE_ATTEMPT_SEQUENCE_INVALID",
            );
        }
        active
            .holder
            .as_mut()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_Q18_HOLDER_MISSING")?
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
            return fail(
                active,
                "NODE_MANAGED_SQLITE_SHM_TEST_Q18_CLOSE_KIND_INVALID",
            );
        }
        advance(
            active,
            Stage::TargetCloseAttempted,
            Stage::TargetCloseSucceeded,
            |counts| &mut counts.target_close_success,
            "NODE_MANAGED_SQLITE_SHM_TEST_Q18_CLOSE_OUTCOME_SEQUENCE_INVALID",
        )?;
        active.close_kind = Some(kind);
        active.pending = 0;
        active.consumed = true;
        Ok(())
    }

    pub(super) fn abort_and_release(&mut self, target: ExactTarget) -> Result<(), &'static str> {
        let active = self
            .active
            .as_ref()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_Q18_NOT_ARMED")?;
        validate_target(active, target)?;
        let mut active = self.active.take().expect("validated Q18 active state");
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
    ) -> Result<ManagedSqliteShmTestCreatedFirstSharedBusyCloseSucceededReceiptV1, &'static str>
    {
        let mut active = self
            .active
            .take()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_Q18_NOT_ARMED")?;
        let holder_values = active
            .holder
            .take()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_Q18_HOLDER_MISSING")?
            .release_explicit()?;
        validate_target(&active, target)?;
        if active.violation.is_some()
            || active.stage != Stage::TargetCloseSucceeded
            || active.pending != 0
            || !active.consumed
            || active.close_kind != Some(ManagedSqliteFileKind::Shm)
        {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_INCOMPLETE_OR_INVALID");
        }
        validate_counts(&active.counts)?;
        validate_terminal(terminal)?;
        validate_requested_lock(target, active.expectation, requested_lock)?;
        let initialization_values = initialization_values(&active);
        validate_holder_values(target, holder_values)?;
        Ok(
            ManagedSqliteShmTestCreatedFirstSharedBusyCloseSucceededReceiptV1::new(
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
        let active = self.require(target)?;
        advance(active, required, next, counter, code)?;
        Ok(true)
    }

    fn require(&mut self, target: ExactTarget) -> Result<&mut ArmedQ18ObservationV1, &'static str> {
        let active = self
            .active
            .as_mut()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_Q18_NOT_ARMED")?;
        if let Err(code) = validate_target(active, target) {
            active.violation = Some(code);
            return Err(code);
        }
        if active.violation.is_some() {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_PREVIOUS_VIOLATION");
        }
        Ok(active)
    }
}

impl ManagedSqliteShmTestInitializationControllerV1 {
    pub(super) fn q18_is_selected(&self, target: ExactTarget) -> Result<bool, &'static str> {
        self.q18.is_selected(target)
    }

    pub(super) fn q18_record_dms_exclusive_unlock_succeeded(
        &mut self,
        target: ExactTarget,
    ) -> Result<bool, &'static str> {
        if !self.q18.is_armed() {
            return Ok(false);
        }
        self.q18.record_dms_exclusive_unlock_succeeded(target)
    }

    pub(super) fn q18_store_holder(
        &mut self,
        target: ExactTarget,
        holder: ManagedSqliteShmTestQ18DmsHolderLeaseV1,
    ) -> Result<(), &'static str> {
        self.q18.store_holder(target, holder)
    }

    pub(super) fn q18_record_target_shared_contended(
        &mut self,
        target: ExactTarget,
    ) -> Result<(), &'static str> {
        self.q18.record_target_shared_contended(target)
    }

    pub(super) fn q18_record_target_close_attempt(
        &mut self,
        target: ExactTarget,
    ) -> Result<(), &'static str> {
        self.q18.record_target_close_attempt(target)
    }

    pub(super) fn q18_record_target_close_succeeded(
        &mut self,
        target: ExactTarget,
        kind: ManagedSqliteFileKind,
    ) -> Result<(), &'static str> {
        self.q18.record_target_close_succeeded(target, kind)
    }

    pub(super) fn q18_abort_and_release(
        &mut self,
        target: ExactTarget,
    ) -> Result<(), &'static str> {
        self.q18.abort_and_release(target)
    }

    pub(super) fn finish_q18(
        &mut self,
        target: ExactTarget,
        terminal: ManagedSqliteShmTestTargetSnapshot,
        requested_lock: ManagedSqliteShmTestLockReceipt,
    ) -> Result<ManagedSqliteShmTestCreatedFirstSharedBusyCloseSucceededReceiptV1, &'static str>
    {
        self.q18.finish(target, terminal, requested_lock)
    }
}

fn validate_target(
    active: &ArmedQ18ObservationV1,
    target: ExactTarget,
) -> Result<(), &'static str> {
    if active.target != target {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_TARGET_MISMATCH");
    }
    if active.owner_thread != std::thread::current().id() {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_THREAD_MISMATCH");
    }
    Ok(())
}

fn advance(
    active: &mut ArmedQ18ObservationV1,
    required: Stage,
    next: Stage,
    counter: impl FnOnce(&mut EventCounts) -> &mut u8,
    code: &'static str,
) -> Result<(), &'static str> {
    if active.stage != required {
        return fail(active, code);
    }
    let selected = counter(&mut active.counts);
    if *selected != 0 {
        return fail(active, code);
    }
    *selected = 1;
    active.stage = next;
    Ok(())
}

fn fail<T>(active: &mut ArmedQ18ObservationV1, code: &'static str) -> Result<T, &'static str> {
    active.violation = Some(code);
    Err(code)
}

fn validate_counts(value: &EventCounts) -> Result<(), &'static str> {
    if value.request != 1
        || value.open_attempt != 1
        || value.open_created != 1
        || value.exclusive_lock_attempt != 1
        || value.exclusive_lock_acquired != 1
        || value.truncate_attempt != 1
        || value.truncate_success != 1
        || value.exclusive_unlock_attempt != 1
        || value.exclusive_unlock_success != 1
        || value.target_shared_attempt != 1
        || value.target_shared_acquired != 0
        || value.target_shared_contended != 1
        || value.target_shared_errors != 0
        || value.target_close_attempt != 1
        || value.target_close_success != 1
        || value.target_close_failure != 0
    {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_EVENT_COUNTS_INVALID");
    }
    Ok(())
}

fn validate_terminal(value: ManagedSqliteShmTestTargetSnapshot) -> Result<(), &'static str> {
    let topology = value.topology;
    if !value.target_attached
        || value.shared_mask != 0
        || value.exclusive_mask != 0
        || topology.shm_connections != 1
        || topology.node_present
        || topology.views != 0
        || topology.mappings != 0
        || topology.dms != ManagedSqliteShmTestDmsCustody::Absent
        || topology.shm_file_present
        || topology.poisoned
        || topology.mutation_may_have_occurred
        || topology.lock_outcome_uncertain
        || topology.domain_terminal
        || topology.quarantined_file_closes != 0
    {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_TERMINAL_STATE_INVALID");
    }
    Ok(())
}

fn validate_requested_lock(
    target: ExactTarget,
    expectation: ManagedSqliteShmTestInitializationExpectationV1,
    receipt: ManagedSqliteShmTestLockReceipt,
) -> Result<(), &'static str> {
    if receipt.runtime_generation != target.0
        || receipt.shm_connection_id != target.1
        || receipt.expectation.path != ManagedSqliteShmTestLockPath::InitializationFailure
        || receipt.expectation.action != expectation.action
        || receipt.expectation.first != expectation.first
        || receipt.expectation.count != expectation.count
        || receipt.expectation.mask != expectation.mask
        || receipt.managed_attempts != 1
        || receipt.managed_successes != 0
        || receipt.native_lock_attempts != 0
        || receipt.native_lock_acquired != 0
        || receipt.native_lock_contended != 0
        || receipt.native_lock_errors != 0
        || receipt.native_unlock_attempts != 0
        || receipt.native_unlock_successes != 0
        || receipt.native_unlock_errors != 0
        || receipt.local_transitions != 0
        || !receipt.finished
    {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_REQUESTED_LOCK_LEDGER_INVALID");
    }
    Ok(())
}

fn validate_holder_values(target: ExactTarget, values: [u64; 15]) -> Result<(), &'static str> {
    if values
        != [
            target.0,
            target.1,
            SHM_DMS_OFFSET,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
            1,
        ]
    {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q18_HOLDER_RECEIPT_INVALID");
    }
    Ok(())
}

fn initialization_values(active: &ArmedQ18ObservationV1) -> [u64; 43] {
    let cold_flags = u64::from(active.cold.node_present)
        | (u64::from(active.cold.shm_file_present) << 1)
        | (u64::from(active.cold.poisoned) << 2)
        | (u64::from(active.cold.domain_terminal) << 3)
        | (u64::from(active.cold.shared_mask != 0) << 4)
        | (u64::from(active.cold.exclusive_mask != 0) << 5);
    [
        1,
        active.expectation.case_v1.tag(),
        1,
        active.target.0,
        active.target.1,
        lock_action_tag(active.expectation.action),
        u64::from(active.expectation.first),
        u64::from(active.expectation.count),
        u64::from(active.expectation.mask),
        u64::from(active.owner_thread == std::thread::current().id()),
        u64::from(active.cold.target_attached),
        u64::from(active.cold.shm_connections),
        cold_flags,
        u64::from(active.counts.request),
        u64::from(active.counts.open_attempt),
        u64::from(active.counts.open_created),
        u64::from(active.counts.exclusive_lock_attempt),
        u64::from(active.counts.exclusive_lock_acquired),
        u64::from(active.counts.truncate_attempt),
        u64::from(active.counts.truncate_success),
        u64::from(active.counts.exclusive_unlock_attempt),
        u64::from(active.counts.exclusive_unlock_success),
        u64::from(active.counts.target_shared_attempt),
        u64::from(active.counts.target_shared_acquired),
        u64::from(active.counts.target_shared_contended),
        u64::from(active.counts.target_shared_errors),
        u64::from(active.counts.target_close_attempt),
        u64::from(active.counts.target_close_success),
        u64::from(active.counts.target_close_failure),
        2,
        1,
        1,
        0,
        1,
        1,
        1,
        1,
        1,
        0,
        u64::from(active.close_kind == Some(ManagedSqliteFileKind::Shm)),
        u64::from(active.pending),
        u64::from(active.consumed),
        1,
    ]
}
