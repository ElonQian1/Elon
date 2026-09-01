//! Ordered event capture for the single active q11 raw Lock observation.

use std::thread;

use rusqlite::ffi;

use super::model::{
    ledger, AbandonOutcome, ActiveObservation, RawValidation, RunCodeOutcome,
};
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi::raw_state::RawSqliteFileStateRejection;

pub(super) fn cancel_observation(observation_id: u64) {
    let Ok(mut ledger) = ledger().lock() else {
        return;
    };
    if ledger.active.as_ref().map(|active| active.observation_id) == Some(observation_id) {
        ledger.active = None;
    }
}

fn with_active(file: *mut ffi::sqlite3_file, event: impl FnOnce(&mut ActiveObservation)) {
    let Ok(mut ledger) = ledger().lock() else {
        return;
    };
    let Some(active) = ledger.active.as_mut() else {
        return;
    };
    if active.violation.is_some() {
        return;
    }
    if active.owner_thread != thread::current().id() {
        mark_violation(active, "raw Lock rejection event occurred on the wrong thread");
        return;
    }
    if active.invocation_file_address != file as usize {
        mark_violation(active, "raw Lock rejection event file did not match invocation");
        return;
    }
    event(active);
}

fn with_active_without_file(event: impl FnOnce(&mut ActiveObservation)) -> bool {
    let Ok(mut ledger) = ledger().lock() else {
        return false;
    };
    let Some(active) = ledger.active.as_mut() else {
        return false;
    };
    if active.violation.is_some() {
        return false;
    }
    if active.owner_thread != thread::current().id() {
        mark_violation(active, "raw Lock rejection Drop occurred on the wrong thread");
        return false;
    }
    event(active);
    true
}

fn set_once<T: Copy>(slot: &mut Option<T>, value: T) -> bool {
    if slot.is_some() {
        false
    } else {
        *slot = Some(value);
        true
    }
}

fn mark_violation(active: &mut ActiveObservation, violation: &'static str) {
    if active.violation.is_none() {
        active.violation = Some(violation);
    }
}

pub(super) fn record_fixture_prepared(
    file: *mut ffi::sqlite3_file,
    slots: u64,
    retained: u64,
) {
    let Ok(mut ledger) = ledger().lock() else {
        return;
    };
    let Some(active) = ledger.active.as_mut() else {
        return;
    };
    if active.source_file_address != file as usize
        || active.owner_thread != thread::current().id()
        || active.counts.fixture_prepare != 0
    {
        mark_violation(active, "raw Lock rejection fixture preparation was mismatched or repeated");
        return;
    }
    active.counts.fixture_prepare = 1;
    active.slots_prepared = Some(slots);
    active.retained_fixture_tag = Some(retained);
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn record_entry(file: *mut ffi::sqlite3_file) {
    with_active(file, |active| {
        if active.counts.fixture_prepare != 1 || active.counts.entry != 0 {
            mark_violation(active, "raw Lock rejection xShmLock entry was reordered or repeated");
        } else {
            active.counts.entry = 1;
        }
    });
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn record_scalar_admitted(file: *mut ffi::sqlite3_file) {
    with_active(file, |active| {
        if active.counts.entry != 1 || active.counts.scalar_admitted != 0 {
            mark_violation(active, "raw Lock rejection scalar admission was reordered or repeated");
        } else {
            active.counts.scalar_admitted = 1;
        }
    });
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn record_raw_rejection(
    file: *mut ffi::sqlite3_file,
    rejection: RawSqliteFileStateRejection,
) {
    let validation = match rejection {
        RawSqliteFileStateRejection::NullFile => RawValidation::NullFile,
        RawSqliteFileStateRejection::Uninstalled => RawValidation::Uninstalled,
        RawSqliteFileStateRejection::ForeignMethods => RawValidation::ForeignMethods,
        RawSqliteFileStateRejection::StateMissing => RawValidation::StateMissing,
        RawSqliteFileStateRejection::TypeMismatch => RawValidation::TypeMismatch,
        _ => return,
    };
    with_active(file, |active| {
        if active.counts.scalar_admitted != 1 || active.counts.raw_validation != 0 {
            mark_violation(active, "raw Lock rejection validation was reordered or repeated");
            return;
        }
        active.counts.raw_validation = 1;
        active.validation = Some(validation);
    });
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn record_envelope_snapshot(
    file: *mut ffi::sqlite3_file,
    type_matches: bool,
    payload_present: bool,
) {
    with_active(file, |active| {
        if active.counts.scalar_admitted != 1 || active.counts.raw_validation != 0 {
            mark_violation(active, "raw Lock envelope snapshot was reordered or repeated");
            return;
        }
        active.counts.raw_validation = 1;
        active.validation = Some(if type_matches {
            RawValidation::Accepted
        } else {
            RawValidation::TypeMismatch
        });
        active.counts.type_check = 1;
        active.counts.payload_snapshot = 1;
        active.type_matches = Some(type_matches);
        active.payload_present = Some(payload_present);
    });
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn record_typed_operation_entry(file: *mut ffi::sqlite3_file) {
    with_active(file, |active| {
        if active.counts.raw_validation != 1 || active.counts.typed_operation_entry != 0 {
            mark_violation(active, "raw Lock typed operation entry was reordered or repeated");
        } else {
            active.counts.typed_operation_entry = 1;
        }
    });
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn record_handle_file_missing() {
    with_active_without_file(|active| {
        if active.counts.typed_operation_entry != 1 || active.counts.handle_file_missing != 0 {
            mark_violation(active, "raw Lock missing HandleBound file was reordered or repeated");
        } else {
            active.counts.handle_file_missing = 1;
        }
    });
}

fn record_run_code_outcome(file: *mut ffi::sqlite3_file, outcome: RunCodeOutcome) {
    with_active(file, |active| {
        if !set_once(&mut active.run_code_outcome, outcome) {
            mark_violation(active, "raw Lock run_code outcome was repeated");
        }
    });
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn record_run_code_normal(
    file: *mut ffi::sqlite3_file,
) {
    record_run_code_outcome(file, RunCodeOutcome::Normal);
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn record_run_code_rejection(
    file: *mut ffi::sqlite3_file,
) {
    record_run_code_outcome(file, RunCodeOutcome::Rejection);
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn record_run_code_unwind(
    file: *mut ffi::sqlite3_file,
) {
    record_run_code_outcome(file, RunCodeOutcome::Unwind);
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn record_abandon_entry(file: *mut ffi::sqlite3_file) {
    with_active(file, |active| {
        if active.counts.abandon_entry != 0 {
            mark_violation(active, "raw Lock abandonment entry was repeated");
        } else {
            active.counts.abandon_entry = 1;
        }
    });
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn record_abandon_rejection(
    file: *mut ffi::sqlite3_file,
    rejection: RawSqliteFileStateRejection,
) {
    let outcome = match rejection {
        RawSqliteFileStateRejection::NullFile => AbandonOutcome::NullFileRejected,
        RawSqliteFileStateRejection::ForeignMethods => AbandonOutcome::ForeignMethodsRejected,
        RawSqliteFileStateRejection::StateMissing => AbandonOutcome::StateMissingRejected,
        _ => return,
    };
    with_active(file, |active| {
        if !set_once(&mut active.abandon_outcome, outcome) {
            mark_violation(active, "raw Lock abandonment rejection was repeated");
        }
    });
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn record_abandon_empty(file: *mut ffi::sqlite3_file) {
    with_active(file, |active| {
        if !set_once(&mut active.abandon_outcome, AbandonOutcome::Empty) {
            mark_violation(active, "raw Lock empty abandonment was repeated");
        }
    });
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn record_slots_cleared(file: *mut ffi::sqlite3_file) {
    with_active(file, |active| {
        if active.counts.slots_clear != 0 {
            mark_violation(active, "raw Lock slot clear was repeated");
        } else {
            active.counts.slots_clear = 1;
        }
    });
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn record_envelope_drop(payload_present: bool) {
    with_active_without_file(|active| {
        if active.counts.slots_clear != 1 || active.counts.envelope_drop != 0 {
            mark_violation(active, "raw Lock envelope Drop was reordered or repeated");
            return;
        }
        active.counts.envelope_drop = 1;
        if active.payload_present != Some(payload_present) {
            mark_violation(active, "raw Lock envelope Drop payload shape drifted");
        }
    });
}

#[derive(Clone, Copy)]
enum DropKind {
    Payload,
    Abandon,
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) struct RawLockDropCompletionGuard {
    kind: DropKind,
    active: bool,
    completed: bool,
}

impl RawLockDropCompletionGuard {
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn complete(mut self) {
        self.completed = true;
        if !self.active {
            return;
        }
        with_active_without_file(|active| match self.kind {
            DropKind::Payload => active.counts.payload_drop_completed += 1,
            DropKind::Abandon => {
                active.counts.abandon_drop_completed += 1;
                if !set_once(
                    &mut active.abandon_outcome,
                    AbandonOutcome::InstalledDropCompleted,
                ) {
                    mark_violation(active, "raw Lock installed abandonment outcome was repeated");
                }
            }
        });
    }
}

impl Drop for RawLockDropCompletionGuard {
    fn drop(&mut self) {
        if !self.active || self.completed || !thread::panicking() {
            return;
        }
        with_active_without_file(|active| match self.kind {
            DropKind::Payload => active.counts.payload_drop_unwind += 1,
            DropKind::Abandon => {
                active.counts.abandon_drop_unwind += 1;
                if !set_once(
                    &mut active.abandon_outcome,
                    AbandonOutcome::InstalledDropUnwindCaught,
                ) {
                    mark_violation(
                        active,
                        "raw Lock installed abandonment unwind outcome was repeated",
                    );
                }
            }
        });
    }
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn begin_payload_drop() -> RawLockDropCompletionGuard {
    let active = with_active_without_file(|active| active.counts.payload_drop_attempt += 1);
    RawLockDropCompletionGuard {
        kind: DropKind::Payload,
        active,
        completed: false,
    }
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn begin_abandon_drop(file: *mut ffi::sqlite3_file) -> RawLockDropCompletionGuard {
    let mut active_guard = false;
    with_active(file, |active| {
        if active.counts.abandon_entry != 1 {
            mark_violation(active, "raw Lock installed abandonment began before entry");
        } else {
            active_guard = true;
        }
    });
    RawLockDropCompletionGuard {
        kind: DropKind::Abandon,
        active: active_guard,
        completed: false,
    }
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) fn record_returned(file: *mut ffi::sqlite3_file, result_code: i32) {
    with_active(file, |active| {
        if active.counts.returned != 0 || active.result_code.is_some() {
            mark_violation(active, "raw Lock callback return was repeated");
        } else {
            active.counts.returned = 1;
            active.result_code = Some(result_code);
        }
    });
}
