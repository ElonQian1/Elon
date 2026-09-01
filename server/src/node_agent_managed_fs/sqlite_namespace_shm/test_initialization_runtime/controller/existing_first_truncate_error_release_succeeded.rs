//! Case-specific Q15 controller for existing-first unread truncate and known cleanup release.

use super::super::model::{
    lock_action_tag, ManagedSqliteShmTestInitializationEvidenceV1,
    ManagedSqliteShmTestInitializationFailureV1,
    ManagedSqliteShmTestInitializationNativeObservationV1,
    ManagedSqliteShmTestInitializationNativeReceiptV1,
};
use super::{
    advance, fail, ArmedInitializationObservationV1, EventCounts, ExactTarget,
    ManagedSqliteShmTestInitializationControllerV1, Stage, TerminalStateV1,
};

impl ManagedSqliteShmTestInitializationControllerV1 {
    pub(super) fn reject_q15_truncate_success_if_selected(
        &mut self,
        target: ExactTarget,
    ) -> Result<(), &'static str> {
        let Some(active) = self.active_for_event(target)? else {
            return Ok(());
        };
        if is_q15(active) {
            return fail(
                active,
                "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_Q15_TRUNCATE_SUCCESS_FORBIDDEN",
            );
        }
        Ok(())
    }

    pub(super) fn reject_q15_release_path_if_selected(
        &mut self,
        target: ExactTarget,
    ) -> Result<(), &'static str> {
        let Some(active) = self.active_for_event(target)? else {
            return Ok(());
        };
        if is_q15(active) {
            return fail(
                active,
                "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_Q15_RELEASE_PATH_INVALID",
            );
        }
        Ok(())
    }

    pub(super) fn begin_existing_first_truncate_outcome_unavailable(
        &mut self,
        target: ExactTarget,
    ) -> Result<bool, &'static str> {
        let Some(active) = self.active_for_event(target)? else {
            return Ok(false);
        };
        if !is_q15(active) {
            return Ok(false);
        }
        if active.stage != Stage::TruncateAttempted
            || active.counts.truncate_attempt != 1
            || active.native.is_some()
            || active.pending != 1
            || active.consumed
        {
            return fail(
                active,
                "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_Q15_TRUNCATE_SEQUENCE_INVALID",
            );
        }
        Ok(true)
    }

    pub(super) fn record_existing_first_truncate_return_receipt_unavailable(
        &mut self,
        target: ExactTarget,
        native: ManagedSqliteShmTestInitializationNativeReceiptV1,
    ) -> Result<(), &'static str> {
        let active = self.require_active_for_event(target)?;
        if !is_q15(active)
            || native.observation
                != ManagedSqliteShmTestInitializationNativeObservationV1::ReturnReceiptUnavailable
            || native.offset != 0
            || native.length != 0
            || native.exact_call_occurrence != 1
            || active.native.is_some()
        {
            return fail(
                active,
                "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_Q15_NATIVE_RECEIPT_INVALID",
            );
        }
        advance(
            active,
            Stage::TruncateAttempted,
            Stage::ReturnReceiptUnavailable,
            |counts| &mut counts.return_receipt_unavailable,
            "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_Q15_NATIVE_RECEIPT_SEQUENCE_INVALID",
        )?;
        active.native = Some(native);
        Ok(())
    }

    pub(super) fn begin_existing_first_truncate_cleanup_unlock(
        &mut self,
        target: ExactTarget,
    ) -> Result<(), &'static str> {
        let active = self.require_active_for_event(target)?;
        if !is_q15(active) {
            return fail(
                active,
                "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_Q15_CLEANUP_CASE_INVALID",
            );
        }
        advance(
            active,
            Stage::ReturnReceiptUnavailable,
            Stage::DmsExclusiveUnlockAttempted,
            |counts| &mut counts.dms_unlock_attempt,
            "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_Q15_CLEANUP_ATTEMPT_SEQUENCE_INVALID",
        )
    }

    pub(super) fn record_existing_first_truncate_cleanup_unlock_succeeded(
        &mut self,
        target: ExactTarget,
    ) -> Result<(), &'static str> {
        let active = self.require_active_for_event(target)?;
        if !is_q15(active) {
            return fail(
                active,
                "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_Q15_CLEANUP_CASE_INVALID",
            );
        }
        advance(
            active,
            Stage::DmsExclusiveUnlockAttempted,
            Stage::DmsExclusiveUnlockSucceeded,
            |counts| &mut counts.dms_unlock_success,
            "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_Q15_CLEANUP_OUTCOME_SEQUENCE_INVALID",
        )?;
        active.pending = 0;
        active.consumed = true;
        Ok(())
    }
}

pub(super) fn reject_release_receipt_if_selected(
    active: &mut ArmedInitializationObservationV1,
) -> Result<(), &'static str> {
    if is_q15(active) {
        return fail(
            active,
            "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_Q15_RELEASE_RECEIPT_INVALID",
        );
    }
    Ok(())
}

pub(super) fn record_poisoned_if_selected(
    active: &mut ArmedInitializationObservationV1,
) -> Result<bool, &'static str> {
    if !is_q15(active) {
        return Ok(false);
    }
    record_poisoned(active)?;
    Ok(true)
}

pub(super) fn validate_finish_if_selected(
    active: &ArmedInitializationObservationV1,
    terminal: TerminalStateV1,
) -> Result<bool, &'static str> {
    if !is_q15(active) {
        return Ok(false);
    }
    validate_exact_counts(&active.counts)?;
    validate_terminal(terminal)?;
    Ok(true)
}

pub(super) fn ordered_values_if_selected(
    active: &ArmedInitializationObservationV1,
    native: ManagedSqliteShmTestInitializationNativeReceiptV1,
    terminal: TerminalStateV1,
) -> Option<[u64; 32]> {
    is_q15(active).then(|| ordered_values(active, native, terminal))
}

fn is_q15(active: &ArmedInitializationObservationV1) -> bool {
    active.expectation.case_v1
        == ManagedSqliteShmTestInitializationFailureV1::ExistingFirstTruncateOutcomeUncertainReleaseSucceeded
}

fn record_poisoned(active: &mut ArmedInitializationObservationV1) -> Result<(), &'static str> {
    advance(
        active,
        Stage::DmsExclusiveUnlockSucceeded,
        Stage::Poisoned,
        |counts| &mut counts.poisoned,
        "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_Q15_POISON_SEQUENCE_INVALID",
    )
}

fn validate_exact_counts(counts: &EventCounts) -> Result<(), &'static str> {
    if counts.request != 1
        || counts.open_attempt != 1
        || counts.open_created != 1
        || counts.dms_lock_attempt != 1
        || counts.dms_lock_acquired != 1
        || counts.truncate_attempt != 1
        || counts.truncate_success != 0
        || counts.return_receipt_unavailable != 1
        || counts.dms_unlock_attempt != 1
        || counts.dms_unlock_success != 1
        || counts.poisoned != 1
    {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_Q15_EVENT_COUNTS_INVALID");
    }
    Ok(())
}

fn validate_terminal(terminal: TerminalStateV1) -> Result<(), &'static str> {
    if !terminal.target_attached
        || terminal.shm_connections != 1
        || !terminal.node_present
        || !terminal.shm_file_present
        || terminal.dms_exclusive_outcome_uncertain
        || !terminal.dms_released
        || !terminal.poisoned
        || !terminal.mutation_may_have_occurred
        || terminal.lock_outcome_uncertain
        || !terminal.domain_terminal
        || terminal.shared_mask != 0
        || terminal.exclusive_mask != 0
    {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_Q15_TERMINAL_STATE_INVALID");
    }
    Ok(())
}

fn ordered_values(
    active: &ArmedInitializationObservationV1,
    native: ManagedSqliteShmTestInitializationNativeReceiptV1,
    terminal: TerminalStateV1,
) -> [u64; 32] {
    let cold_flags = u64::from(active.cold.node_present)
        | (u64::from(active.cold.shm_file_present) << 1)
        | (u64::from(active.cold.poisoned) << 2)
        | (u64::from(active.cold.domain_terminal) << 3)
        | (u64::from(active.cold.shared_mask != 0) << 4)
        | (u64::from(active.cold.exclusive_mask != 0) << 5);
    let terminal_flags = u64::from(terminal.target_attached)
        | (u64::from(terminal.shm_connections == 1) << 1)
        | (u64::from(terminal.node_present) << 2)
        | (u64::from(terminal.shm_file_present) << 3)
        | (u64::from(terminal.poisoned) << 4)
        | (u64::from(terminal.mutation_may_have_occurred) << 5)
        | (u64::from(terminal.lock_outcome_uncertain) << 6)
        | (u64::from(terminal.domain_terminal) << 7)
        | (u64::from(terminal.shared_mask == 0 && terminal.exclusive_mask == 0) << 8);
    [
        1,
        active.expectation.case_v1.tag(),
        ManagedSqliteShmTestInitializationEvidenceV1::ControlledFaultActual.tag(),
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
        u64::from(active.counts.dms_lock_attempt),
        u64::from(active.counts.dms_lock_acquired),
        u64::from(active.counts.truncate_attempt),
        u64::from(active.counts.truncate_success),
        u64::from(active.counts.dms_unlock_attempt),
        native.observation.tag(),
        native.offset,
        native.length,
        u64::from(native.exact_call_occurrence),
        u64::from(active.counts.return_receipt_unavailable),
        u64::from(active.counts.dms_unlock_success),
        u64::from(terminal.dms_exclusive_outcome_uncertain),
        terminal_flags,
        u64::from(active.pending),
        u64::from(active.consumed),
        1,
    ]
}
