//! Q19 state and sequence primitives.

use std::thread::ThreadId;

use crate::node_agent_managed_fs::ManagedSqliteFileKind;

use super::super::{ColdPrestateV1, ExactTarget};
use super::super::super::{
    existing_first_shared_busy_close_succeeded::ManagedSqliteShmTestQ19DmsHolderLeaseV1,
    model::ManagedSqliteShmTestInitializationExpectationV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Stage {
    Armed,
    Requested,
    OpenAttempted,
    OpenObservedExisting,
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
pub(super) struct EventCounts {
    pub(super) request: u8,
    pub(super) open_attempt: u8,
    pub(super) open_existing: u8,
    pub(super) exclusive_lock_attempt: u8,
    pub(super) exclusive_lock_acquired: u8,
    pub(super) truncate_attempt: u8,
    pub(super) truncate_success: u8,
    pub(super) exclusive_unlock_attempt: u8,
    pub(super) exclusive_unlock_success: u8,
    pub(super) target_shared_attempt: u8,
    pub(super) target_shared_acquired: u8,
    pub(super) target_shared_contended: u8,
    pub(super) target_shared_errors: u8,
    pub(super) target_close_attempt: u8,
    pub(super) target_close_success: u8,
    pub(super) target_close_failure: u8,
}

pub(super) struct ArmedQ19ObservationV1 {
    pub(super) target: ExactTarget,
    pub(super) owner_thread: ThreadId,
    pub(super) expectation: ManagedSqliteShmTestInitializationExpectationV1,
    pub(super) cold: ColdPrestateV1,
    pub(super) stage: Stage,
    pub(super) counts: EventCounts,
    pub(super) holder: Option<ManagedSqliteShmTestQ19DmsHolderLeaseV1>,
    pub(super) close_kind: Option<ManagedSqliteFileKind>,
    pub(super) pending: u8,
    pub(super) consumed: bool,
    pub(super) violation: Option<&'static str>,
}

impl ArmedQ19ObservationV1 {
    pub(super) fn new(
        target: ExactTarget,
        expectation: ManagedSqliteShmTestInitializationExpectationV1,
        cold: ColdPrestateV1,
    ) -> Self {
        Self {
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
        }
    }

    pub(super) fn validate_target(&self, target: ExactTarget) -> Result<(), &'static str> {
        if self.target != target {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q19_TARGET_MISMATCH");
        }
        if self.owner_thread != std::thread::current().id() {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_Q19_THREAD_MISMATCH");
        }
        Ok(())
    }

    pub(super) fn advance(
        &mut self,
        required: Stage,
        next: Stage,
        counter: impl FnOnce(&mut EventCounts) -> &mut u8,
        code: &'static str,
    ) -> Result<(), &'static str> {
        if self.stage != required {
            return self.fail(code);
        }
        let selected = counter(&mut self.counts);
        if *selected != 0 {
            return self.fail(code);
        }
        *selected = 1;
        self.stage = next;
        Ok(())
    }

    pub(super) fn fail<T>(&mut self, code: &'static str) -> Result<T, &'static str> {
        self.violation = Some(code);
        Err(code)
    }
}
