use std::thread::ThreadId;

use super::super::{
    test_lock_runtime::{ManagedSqliteShmTestLockPath, ManagedSqliteShmTestLockReceipt},
    types::{
        ManagedSqliteShmLockAction, ManagedSqliteShmLockRequest, SHM_DMS_OFFSET, SHM_LOCK_COUNT,
    },
};
use super::model::{
    lock_action_tag, ManagedSqliteShmTestInitializationEvidenceV1,
    ManagedSqliteShmTestInitializationExpectationV1,
    ManagedSqliteShmTestInitializationNativeObservationV1,
    ManagedSqliteShmTestInitializationNativeReceiptV1, ManagedSqliteShmTestInitializationReceiptV1,
};

#[path = "controller/created_first_truncate_error_release_failed.rs"]
mod created_first_truncate_error_release_failed;
#[path = "controller/created_first_truncate_error_release_succeeded.rs"]
mod created_first_truncate_error_release_succeeded;
#[path = "controller/existing_first_truncate_error_release_failed.rs"]
mod existing_first_truncate_error_release_failed;
#[path = "controller/existing_first_truncate_error_release_succeeded.rs"]
mod existing_first_truncate_error_release_succeeded;

type ExactTarget = (u64, u64);

#[derive(Debug, Clone, Copy)]
pub(super) struct ColdPrestateV1 {
    pub(super) target_attached: bool,
    pub(super) shm_connections: u8,
    pub(super) node_present: bool,
    pub(super) shm_file_present: bool,
    pub(super) poisoned: bool,
    pub(super) domain_terminal: bool,
    pub(super) shared_mask: u8,
    pub(super) exclusive_mask: u8,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct TerminalStateV1 {
    pub(super) target_attached: bool,
    pub(super) shm_connections: u8,
    pub(super) node_present: bool,
    pub(super) shm_file_present: bool,
    pub(super) dms_exclusive_outcome_uncertain: bool,
    pub(super) dms_released: bool,
    pub(super) poisoned: bool,
    pub(super) mutation_may_have_occurred: bool,
    pub(super) lock_outcome_uncertain: bool,
    pub(super) domain_terminal: bool,
    pub(super) shared_mask: u8,
    pub(super) exclusive_mask: u8,
}

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
    ReturnReceiptUnavailable,
    CleanupReturnReceiptUnavailable,
    Poisoned,
}

#[derive(Default)]
struct EventCounts {
    request: u8,
    open_attempt: u8,
    open_created: u8,
    dms_lock_attempt: u8,
    dms_lock_acquired: u8,
    truncate_attempt: u8,
    truncate_success: u8,
    dms_unlock_attempt: u8,
    dms_unlock_success: u8,
    return_receipt_unavailable: u8,
    cleanup_return_receipt_unavailable: u8,
    poisoned: u8,
}

struct ArmedInitializationObservationV1 {
    target: ExactTarget,
    owner_thread: ThreadId,
    expectation: ManagedSqliteShmTestInitializationExpectationV1,
    cold: ColdPrestateV1,
    stage: Stage,
    counts: EventCounts,
    native: Option<ManagedSqliteShmTestInitializationNativeReceiptV1>,
    cleanup_native: Option<ManagedSqliteShmTestInitializationNativeReceiptV1>,
    pending: u8,
    consumed: bool,
    violation: Option<&'static str>,
}

#[derive(Default)]
pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) struct ManagedSqliteShmTestInitializationControllerV1
{
    armed: Option<ArmedInitializationObservationV1>,
}

impl ManagedSqliteShmTestInitializationControllerV1 {
    pub(super) fn arm(
        &mut self,
        target: ExactTarget,
        expectation: ManagedSqliteShmTestInitializationExpectationV1,
        cold: ColdPrestateV1,
    ) -> Result<(), &'static str> {
        validate_expectation(target, expectation)?;
        validate_cold_prestate(cold)?;
        if self.armed.is_some() {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_ALREADY_ARMED");
        }
        self.armed = Some(ArmedInitializationObservationV1 {
            target,
            owner_thread: std::thread::current().id(),
            expectation,
            cold,
            stage: Stage::Armed,
            counts: EventCounts::default(),
            native: None,
            cleanup_native: None,
            pending: 1,
            consumed: false,
            violation: None,
        });
        Ok(())
    }

    pub(super) fn cancel_after_arm(&mut self, target: ExactTarget) -> Result<(), &'static str> {
        let active = self
            .armed
            .as_ref()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_NOT_ARMED")?;
        if active.target != target {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_TARGET_MISMATCH");
        }
        if active.stage != Stage::Armed || active.consumed || active.pending != 1 {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_CANCEL_AFTER_PROGRESS");
        }
        self.armed.take();
        Ok(())
    }

    pub(super) fn record_request(
        &mut self,
        target: ExactTarget,
        request: ManagedSqliteShmLockRequest,
    ) -> Result<bool, &'static str> {
        let Some(active) = self.active_for_event(target)? else {
            return Ok(false);
        };
        if active.expectation.action != request.action()
            || active.expectation.first != request.first()
            || active.expectation.count != request.count()
            || active.expectation.mask != request.mask()
        {
            return fail(
                active,
                "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_REQUEST_MISMATCH",
            );
        }
        advance(
            active,
            Stage::Armed,
            Stage::Requested,
            |counts| &mut counts.request,
            "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_REQUEST_SEQUENCE_INVALID",
        )?;
        Ok(true)
    }

    pub(super) fn record_open_attempt(
        &mut self,
        target: ExactTarget,
    ) -> Result<bool, &'static str> {
        self.advance_if_armed(
            target,
            Stage::Requested,
            Stage::OpenAttempted,
            |counts| &mut counts.open_attempt,
            "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_OPEN_ATTEMPT_SEQUENCE_INVALID",
        )
    }

    pub(super) fn record_open_created(
        &mut self,
        target: ExactTarget,
        created: bool,
    ) -> Result<bool, &'static str> {
        let Some(active) = self.active_for_event(target)? else {
            return Ok(false);
        };
        let violation = match (active.expectation.case_v1, created) {
            (
                super::model::ManagedSqliteShmTestInitializationFailureV1::CreatedFirstExclusiveReleaseOutcomeUncertain
                | super::model::ManagedSqliteShmTestInitializationFailureV1::CreatedFirstTruncateOutcomeUncertainReleaseSucceeded
                | super::model::ManagedSqliteShmTestInitializationFailureV1::CreatedFirstTruncateOutcomeUncertainReleaseFailed,
                false,
            ) => Some("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_NOT_CREATED_FIRST"),
            (
                super::model::ManagedSqliteShmTestInitializationFailureV1::CreatedFirstExclusiveReleaseOutcomeUncertain
                | super::model::ManagedSqliteShmTestInitializationFailureV1::CreatedFirstTruncateOutcomeUncertainReleaseSucceeded
                | super::model::ManagedSqliteShmTestInitializationFailureV1::CreatedFirstTruncateOutcomeUncertainReleaseFailed,
                true,
            ) => None,
            (
                super::model::ManagedSqliteShmTestInitializationFailureV1::ExistingFirstExclusiveReleaseOutcomeUncertain
                | super::model::ManagedSqliteShmTestInitializationFailureV1::ExistingFirstTruncateOutcomeUncertainReleaseSucceeded
                | super::model::ManagedSqliteShmTestInitializationFailureV1::ExistingFirstTruncateOutcomeUncertainReleaseFailed,
                true,
            ) => Some("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_NOT_EXISTING_FIRST"),
            (
                super::model::ManagedSqliteShmTestInitializationFailureV1::ExistingFirstExclusiveReleaseOutcomeUncertain
                | super::model::ManagedSqliteShmTestInitializationFailureV1::ExistingFirstTruncateOutcomeUncertainReleaseSucceeded
                | super::model::ManagedSqliteShmTestInitializationFailureV1::ExistingFirstTruncateOutcomeUncertainReleaseFailed,
                false,
            ) => None,
        };
        if let Some(code) = violation {
            return fail(active, code);
        }
        advance(
            active,
            Stage::OpenAttempted,
            Stage::OpenCreated,
            |counts| &mut counts.open_created,
            "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_OPEN_COMPLETION_SEQUENCE_INVALID",
        )?;
        Ok(true)
    }

    pub(super) fn record_dms_exclusive_lock_attempt(
        &mut self,
        target: ExactTarget,
    ) -> Result<bool, &'static str> {
        self.advance_if_armed(
            target,
            Stage::OpenCreated,
            Stage::DmsExclusiveLockAttempted,
            |counts| &mut counts.dms_lock_attempt,
            "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_DMS_LOCK_ATTEMPT_SEQUENCE_INVALID",
        )
    }

    pub(super) fn record_dms_exclusive_acquired(
        &mut self,
        target: ExactTarget,
    ) -> Result<bool, &'static str> {
        self.advance_if_armed(
            target,
            Stage::DmsExclusiveLockAttempted,
            Stage::DmsExclusiveAcquired,
            |counts| &mut counts.dms_lock_acquired,
            "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_DMS_LOCK_OUTCOME_INVALID",
        )
    }

    pub(super) fn record_truncate_attempt(
        &mut self,
        target: ExactTarget,
    ) -> Result<bool, &'static str> {
        self.advance_if_armed(
            target,
            Stage::DmsExclusiveAcquired,
            Stage::TruncateAttempted,
            |counts| &mut counts.truncate_attempt,
            "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_TRUNCATE_ATTEMPT_SEQUENCE_INVALID",
        )
    }

    pub(super) fn record_truncate_success(
        &mut self,
        target: ExactTarget,
    ) -> Result<bool, &'static str> {
        self.reject_q14_truncate_success_if_selected(target)?;
        self.reject_q15_truncate_success_if_selected(target)?;
        self.reject_q16_truncate_success_if_selected(target)?;
        self.reject_q17_truncate_success_if_selected(target)?;
        self.advance_if_armed(
            target,
            Stage::TruncateAttempted,
            Stage::Truncated,
            |counts| &mut counts.truncate_success,
            "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_TRUNCATE_OUTCOME_INVALID",
        )
    }

    pub(super) fn begin_dms_exclusive_unlock(
        &mut self,
        target: ExactTarget,
    ) -> Result<bool, &'static str> {
        self.reject_q14_release_path_if_selected(target)?;
        self.reject_q15_release_path_if_selected(target)?;
        self.reject_q16_release_path_if_selected(target)?;
        self.reject_q17_release_path_if_selected(target)?;
        self.advance_if_armed(
            target,
            Stage::Truncated,
            Stage::DmsExclusiveUnlockAttempted,
            |counts| &mut counts.dms_unlock_attempt,
            "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_DMS_UNLOCK_SEQUENCE_INVALID",
        )
    }

    pub(super) fn record_return_receipt_unavailable(
        &mut self,
        target: ExactTarget,
        native: ManagedSqliteShmTestInitializationNativeReceiptV1,
    ) -> Result<(), &'static str> {
        let active = self.require_active_for_event(target)?;
        created_first_truncate_error_release_succeeded::reject_release_receipt_if_selected(active)?;
        existing_first_truncate_error_release_succeeded::reject_release_receipt_if_selected(
            active,
        )?;
        created_first_truncate_error_release_failed::reject_release_receipt_if_selected(active)?;
        existing_first_truncate_error_release_failed::reject_release_receipt_if_selected(active)?;
        if native.observation
            != ManagedSqliteShmTestInitializationNativeObservationV1::ReturnReceiptUnavailable
            || native.length != 1
            || native.offset != SHM_DMS_OFFSET
            || native.exact_call_occurrence != 1
            || active.native.is_some()
        {
            return fail(
                active,
                "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_NATIVE_RECEIPT_INVALID",
            );
        }
        advance(
            active,
            Stage::DmsExclusiveUnlockAttempted,
            Stage::ReturnReceiptUnavailable,
            |counts| &mut counts.return_receipt_unavailable,
            "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_NATIVE_RECEIPT_SEQUENCE_INVALID",
        )?;
        active.native = Some(native);
        active.pending = 0;
        active.consumed = true;
        Ok(())
    }

    pub(super) fn record_poisoned(&mut self, target: ExactTarget) -> Result<(), &'static str> {
        let active = self.require_active_for_event(target)?;
        if created_first_truncate_error_release_failed::record_poisoned_if_selected(active)? {
            return Ok(());
        }
        if existing_first_truncate_error_release_failed::record_poisoned_if_selected(active)? {
            return Ok(());
        }
        if created_first_truncate_error_release_succeeded::record_poisoned_if_selected(active)? {
            return Ok(());
        }
        if existing_first_truncate_error_release_succeeded::record_poisoned_if_selected(active)? {
            return Ok(());
        }
        advance(
            active,
            Stage::ReturnReceiptUnavailable,
            Stage::Poisoned,
            |counts| &mut counts.poisoned,
            "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_POISON_SEQUENCE_INVALID",
        )
    }

    pub(super) fn reject_if_armed(
        &mut self,
        target: ExactTarget,
        code: &'static str,
    ) -> Result<(), &'static str> {
        let Some(active) = self.active_for_event(target)? else {
            return Ok(());
        };
        fail(active, code)
    }

    pub(super) fn finish(
        &mut self,
        target: ExactTarget,
        terminal: TerminalStateV1,
        requested_lock: ManagedSqliteShmTestLockReceipt,
    ) -> Result<ManagedSqliteShmTestInitializationReceiptV1, &'static str> {
        let active = self
            .armed
            .take()
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_NOT_ARMED")?;
        if active.target != target
            || active.owner_thread != std::thread::current().id()
            || active.violation.is_some()
            || active.stage != Stage::Poisoned
            || active.pending != 0
            || !active.consumed
        {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_INCOMPLETE_OR_INVALID");
        }
        let dual_native_receipts_selected =
            created_first_truncate_error_release_failed::is_selected(&active)
                || existing_first_truncate_error_release_failed::is_selected(&active);
        if dual_native_receipts_selected {
            if active.cleanup_native.is_none() {
                return Err(
                    "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_CLEANUP_NATIVE_RECEIPT_MISSING",
                );
            }
        } else if active.cleanup_native.is_some() {
            return Err(
                "NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_UNEXPECTED_CLEANUP_NATIVE_RECEIPT",
            );
        }
        if !existing_first_truncate_error_release_failed::validate_finish_if_selected(
            &active, terminal,
        )? && !created_first_truncate_error_release_failed::validate_finish_if_selected(
            &active, terminal,
        )? && !existing_first_truncate_error_release_succeeded::validate_finish_if_selected(
            &active, terminal,
        )? {
            created_first_truncate_error_release_succeeded::validate_finish(&active, terminal)?;
        }
        validate_requested_lock(active.target, active.expectation, requested_lock)?;
        let native = active
            .native
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_NATIVE_RECEIPT_MISSING")?;
        let ordered_values =
            existing_first_truncate_error_release_failed::ordered_values_if_selected(
                &active, native, terminal,
            )
            .or_else(|| {
                created_first_truncate_error_release_failed::ordered_values_if_selected(
                    &active, native, terminal,
                )
            })
            .or_else(|| {
                existing_first_truncate_error_release_succeeded::ordered_values_if_selected(
                    &active, native, terminal,
                )
            })
            .unwrap_or_else(|| {
                created_first_truncate_error_release_succeeded::ordered_values_for_case(
                    &active, native, terminal,
                )
            });
        Ok(ManagedSqliteShmTestInitializationReceiptV1::new(
            active.expectation,
            native,
            active.cleanup_native,
            requested_lock,
            ordered_values,
        ))
    }

    fn advance_if_armed(
        &mut self,
        target: ExactTarget,
        required: Stage,
        next: Stage,
        counter: impl FnOnce(&mut EventCounts) -> &mut u8,
        code: &'static str,
    ) -> Result<bool, &'static str> {
        let Some(active) = self.active_for_event(target)? else {
            return Ok(false);
        };
        advance(active, required, next, counter, code)?;
        Ok(true)
    }

    fn require_active_for_event(
        &mut self,
        target: ExactTarget,
    ) -> Result<&mut ArmedInitializationObservationV1, &'static str> {
        self.active_for_event(target)?
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_NOT_ARMED")
    }

    fn active_for_event(
        &mut self,
        target: ExactTarget,
    ) -> Result<Option<&mut ArmedInitializationObservationV1>, &'static str> {
        let Some(active) = self.armed.as_mut() else {
            return Ok(None);
        };
        if active.violation.is_some() {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_PREVIOUS_VIOLATION");
        }
        if active.target != target {
            active.violation = Some("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_TARGET_MISMATCH");
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_TARGET_MISMATCH");
        }
        if active.owner_thread != std::thread::current().id() {
            active.violation = Some("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_THREAD_MISMATCH");
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_THREAD_MISMATCH");
        }
        Ok(Some(active))
    }
}

fn advance(
    active: &mut ArmedInitializationObservationV1,
    required: Stage,
    next: Stage,
    counter: impl FnOnce(&mut EventCounts) -> &mut u8,
    code: &'static str,
) -> Result<(), &'static str> {
    if active.stage != required {
        return fail(active, code);
    }
    let duplicate = {
        let selected = counter(&mut active.counts);
        if *selected == 0 {
            *selected = 1;
            false
        } else {
            true
        }
    };
    if duplicate {
        return fail(active, code);
    }
    active.stage = next;
    Ok(())
}

fn fail<T>(
    active: &mut ArmedInitializationObservationV1,
    code: &'static str,
) -> Result<T, &'static str> {
    active.violation = Some(code);
    Err(code)
}

fn validate_expectation(
    target: ExactTarget,
    expectation: ManagedSqliteShmTestInitializationExpectationV1,
) -> Result<(), &'static str> {
    if target.0 == 0 || target.1 == 0 {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_TARGET_ZERO");
    }
    if !matches!(
        expectation.case_v1,
        super::model::ManagedSqliteShmTestInitializationFailureV1::CreatedFirstExclusiveReleaseOutcomeUncertain
            | super::model::ManagedSqliteShmTestInitializationFailureV1::ExistingFirstExclusiveReleaseOutcomeUncertain
            | super::model::ManagedSqliteShmTestInitializationFailureV1::CreatedFirstTruncateOutcomeUncertainReleaseSucceeded
            | super::model::ManagedSqliteShmTestInitializationFailureV1::ExistingFirstTruncateOutcomeUncertainReleaseSucceeded
            | super::model::ManagedSqliteShmTestInitializationFailureV1::CreatedFirstTruncateOutcomeUncertainReleaseFailed
            | super::model::ManagedSqliteShmTestInitializationFailureV1::ExistingFirstTruncateOutcomeUncertainReleaseFailed
    ) {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_CASE_INVALID");
    }
    if !matches!(
        expectation.action,
        ManagedSqliteShmLockAction::LockShared | ManagedSqliteShmLockAction::LockExclusive
    ) {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_ACTION_INVALID");
    }
    let end = expectation
        .first
        .checked_add(expectation.count)
        .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_RANGE_OVERFLOW")?;
    if expectation.count == 0
        || end > SHM_LOCK_COUNT
        || (expectation.action == ManagedSqliteShmLockAction::LockShared && expectation.count != 1)
    {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_RANGE_INVALID");
    }
    let low = 1u16 << expectation.first;
    let high = 1u16 << end;
    if expectation.mask != (high - low) as u8 {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_MASK_MISMATCH");
    }
    Ok(())
}

fn validate_cold_prestate(cold: ColdPrestateV1) -> Result<(), &'static str> {
    if !cold.target_attached
        || cold.shm_connections != 1
        || cold.node_present
        || cold.shm_file_present
        || cold.poisoned
        || cold.domain_terminal
        || cold.shared_mask != 0
        || cold.exclusive_mask != 0
    {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_COLD_PRESTATE_INVALID");
    }
    Ok(())
}

fn validate_exact_counts(counts: &EventCounts) -> Result<(), &'static str> {
    if counts.request != 1
        || counts.open_attempt != 1
        || counts.open_created != 1
        || counts.dms_lock_attempt != 1
        || counts.dms_lock_acquired != 1
        || counts.truncate_attempt != 1
        || counts.truncate_success != 1
        || counts.dms_unlock_attempt != 1
        || counts.dms_unlock_success != 0
        || counts.return_receipt_unavailable != 1
        || counts.cleanup_return_receipt_unavailable != 0
        || counts.poisoned != 1
    {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_EVENT_COUNTS_INVALID");
    }
    Ok(())
}

fn validate_terminal(terminal: TerminalStateV1) -> Result<(), &'static str> {
    if !terminal.target_attached
        || terminal.shm_connections != 1
        || !terminal.node_present
        || !terminal.shm_file_present
        || !terminal.dms_exclusive_outcome_uncertain
        || terminal.dms_released
        || !terminal.poisoned
        || !terminal.mutation_may_have_occurred
        || !terminal.lock_outcome_uncertain
        || !terminal.domain_terminal
        || terminal.shared_mask != 0
        || terminal.exclusive_mask != 0
    {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_TERMINAL_STATE_INVALID");
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
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_INITIALIZATION_REQUESTED_LOCK_LEDGER_INVALID");
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
        1,
        1,
        u64::from(terminal.dms_exclusive_outcome_uncertain),
        terminal_flags,
        u64::from(active.pending),
        u64::from(active.consumed),
        1,
    ]
}
