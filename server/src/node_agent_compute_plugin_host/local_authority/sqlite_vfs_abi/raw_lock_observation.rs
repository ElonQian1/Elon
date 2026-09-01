//! Windows-test-only linear evidence for controlled raw-state xShmLock rejections.
//!
//! The fixture first proves one exact installed production callback, then replaces only the
//! memory-safe raw representation selected by the closed case enum. The saved callback is the
//! production `xShmLock` entry. Every admission, abandonment and Drop event is passive observation
//! of that production path; the synthetic premise is reported only as `ControlledFaultActual`.

use std::{
    marker::PhantomData,
    os::raw::c_int,
    rc::Rc,
    thread::{self, ThreadId},
};

use rusqlite::ffi;

use super::{file_state::HandleBoundSqliteFileState, raw_state};

mod events;
mod expected;
mod model;

pub(super) use events::{
    begin_abandon_drop, begin_payload_drop, record_abandon_empty, record_abandon_entry,
    record_abandon_rejection, record_entry, record_envelope_drop, record_envelope_snapshot,
    record_handle_file_missing, record_raw_rejection, record_returned, record_run_code_normal,
    record_run_code_rejection, record_run_code_unwind, record_scalar_admitted,
    record_slots_cleared, record_typed_operation_entry,
};
pub(in crate::node_agent_compute_plugin_host::local_authority) use model::{
    HandleBoundSqliteAbiRawLockEvidenceV1, HandleBoundSqliteAbiRawLockRejectionCaseV1,
    HandleBoundSqliteAbiRawLockRejectionReceiptV1,
};

use events::{cancel_observation, record_fixture_prepared};
use model::{ledger, ActiveObservation, EventCounts};

#[must_use = "the controlled raw Lock guard must invoke and finish exactly once"]
pub(in crate::node_agent_compute_plugin_host::local_authority) struct HandleBoundSqliteAbiRawLockRejectionGuardV1
{
    observation_id: u64,
    source_file: *mut ffi::sqlite3_file,
    invocation_file: *mut ffi::sqlite3_file,
    callback: unsafe extern "C" fn(*mut ffi::sqlite3_file, c_int, c_int, c_int) -> c_int,
    owner_thread: ThreadId,
    finished: bool,
    not_send_or_sync: PhantomData<Rc<()>>,
}

impl HandleBoundSqliteAbiRawLockRejectionGuardV1 {
    /// Invokes the saved production xShmLock entry over the prepared memory-safe fixture and
    /// consumes the one-shot observation. The raw scalars must themselves be valid; q11 starts
    /// strictly after q10's scalar gate.
    ///
    /// # Safety
    ///
    /// The owning Connection and its file allocation must remain live and callback-serialized.
    pub(in crate::node_agent_compute_plugin_host::local_authority) unsafe fn invoke(
        mut self,
        offset: c_int,
        count: c_int,
        flags: c_int,
    ) -> Result<HandleBoundSqliteAbiRawLockRejectionReceiptV1, &'static str> {
        // SAFETY: arming captured the production callback from the exact live allocation. The
        // selected case controls whether it receives that allocation or the safe null value.
        let result_code = unsafe { (self.callback)(self.invocation_file, offset, count, flags) };
        self.finished = true;
        finish_observation(
            self.observation_id,
            &self.owner_thread,
            self.source_file,
            result_code,
        )
    }
}

impl Drop for HandleBoundSqliteAbiRawLockRejectionGuardV1 {
    fn drop(&mut self) {
        if !self.finished {
            cancel_observation(self.observation_id);
        }
    }
}

/// Arms one of the eleven reviewed memory-safe raw representations. The two pointer-safety
/// exclusions intentionally have no enum variant and therefore cannot be constructed here.
///
/// # Safety
///
/// `file` must be this ABI module's live, exactly installed, callback-serialized allocation and
/// remain alive until the returned guard is consumed.
pub(in crate::node_agent_compute_plugin_host::local_authority) unsafe fn arm_test_x_shm_lock_raw_state_rejection_v1(
    file: *mut ffi::sqlite3_file,
    case_v1: HandleBoundSqliteAbiRawLockRejectionCaseV1,
) -> Result<HandleBoundSqliteAbiRawLockRejectionGuardV1, &'static str> {
    if !unsafe {
        raw_state::test_vfs_file_has_exact_installed_state::<HandleBoundSqliteFileState>(file)
    } {
        return Err("raw Lock rejection requires exact installed HandleBound state");
    }
    // SAFETY: exact installed-state validation above proves this live method-table read.
    let methods = unsafe { (*file).pMethods };
    if methods.is_null() {
        return Err("raw Lock rejection installed method table was missing");
    }
    // SAFETY: the exact table is the module-owned immutable table.
    let callback = unsafe { (*methods).xShmLock }
        .ok_or("raw Lock rejection installed xShmLock callback was missing")?;
    let slots_before = unsafe { raw_state::lock_raw_control::slot_tag(file) }?;
    if slots_before != 7 {
        return Err("raw Lock rejection did not start from exact installed slots");
    }

    let owner_thread = thread::current().id();
    let observation_id = {
        let mut ledger = ledger()
            .lock()
            .map_err(|_| "raw Lock rejection observation ledger was poisoned")?;
        if ledger.active.is_some() {
            return Err("raw Lock rejection observation already active");
        }
        let observation_id = ledger.next_observation_id;
        ledger.next_observation_id = observation_id
            .checked_add(1)
            .ok_or("raw Lock rejection observation identity exhausted")?;
        if observation_id == 0 {
            return Err("raw Lock rejection observation identity was zero");
        }
        ledger.active = Some(ActiveObservation {
            observation_id,
            case_v1,
            source_file_address: file as usize,
            invocation_file_address: if case_v1.invocation_file_is_null() {
                0
            } else {
                file as usize
            },
            owner_thread: owner_thread.clone(),
            slots_before,
            slots_prepared: None,
            slots_after: None,
            retained_fixture_tag: None,
            counts: EventCounts::default(),
            validation: None,
            type_matches: None,
            payload_present: None,
            run_code_outcome: None,
            abandon_outcome: None,
            result_code: None,
            violation: None,
        });
        observation_id
    };

    // SAFETY: the closed case enum permits only representations implemented by the memory-safe
    // q11 controller over the exact allocation established above.
    let prepared = unsafe { raw_state::lock_raw_control::prepare(file, case_v1) };
    let (slots_prepared, retained_fixture_tag) = match prepared {
        Ok(prepared) => prepared,
        Err(error) => {
            cancel_observation(observation_id);
            return Err(error);
        }
    };
    record_fixture_prepared(file, slots_prepared, retained_fixture_tag);

    Ok(HandleBoundSqliteAbiRawLockRejectionGuardV1 {
        observation_id,
        source_file: file,
        invocation_file: if case_v1.invocation_file_is_null() {
            std::ptr::null_mut()
        } else {
            file
        },
        callback,
        owner_thread,
        finished: false,
        not_send_or_sync: PhantomData,
    })
}

fn finish_observation(
    observation_id: u64,
    owner_thread: &ThreadId,
    source_file: *mut ffi::sqlite3_file,
    result_code: c_int,
) -> Result<HandleBoundSqliteAbiRawLockRejectionReceiptV1, &'static str> {
    let slots_after = unsafe { raw_state::lock_raw_control::slot_tag(source_file) }?;
    let mut ledger = ledger()
        .lock()
        .map_err(|_| "raw Lock rejection observation ledger was poisoned")?;
    let Some(mut active) = ledger.active.take() else {
        return Err("raw Lock rejection observation was missing at finish");
    };
    if active.observation_id != observation_id {
        ledger.active = Some(active);
        return Err("raw Lock rejection observation guard was stale");
    }
    if &active.owner_thread != owner_thread || thread::current().id() != owner_thread.clone() {
        return Err("raw Lock rejection observation finished on the wrong thread");
    }
    active.slots_after = Some(slots_after);
    if active.result_code != Some(result_code) {
        return Err("raw Lock rejection callback return did not match production ledger");
    }
    if let Some(violation) = active.violation {
        return Err(violation);
    }
    let values = expected::ordered_values(&active)?;
    expected::validate_exact_values(active.case_v1, values)?;
    Ok(HandleBoundSqliteAbiRawLockRejectionReceiptV1::new(
        active.case_v1,
        observation_id,
        result_code,
        values,
    ))
}
