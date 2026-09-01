//! Test-only linear observation of the installed production xShmLock ABI entry.
//!
//! The observer never changes callback control flow. It binds one exact live file, calling
//! thread, and raw tuple, then records the production entry's ordered validation events. Any
//! mismatch, duplicate, stale guard, or replay makes the one-shot receipt unavailable.

use std::{
    marker::PhantomData,
    os::raw::c_int,
    rc::Rc,
    sync::{Mutex, OnceLock},
    thread::{self, ThreadId},
};

use rusqlite::ffi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RawTuple {
    offset: c_int,
    count: c_int,
    flags: c_int,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Armed,
    Entered,
    ScalarRejected,
    RunCodeEntered,
    Returned,
}

struct ActiveObservation {
    observation_id: u64,
    file_address: usize,
    owner_thread: ThreadId,
    raw: RawTuple,
    stage: Stage,
    entry_count: u64,
    scalar_rejection_count: u64,
    offset_valid: Option<bool>,
    count_valid: Option<bool>,
    flags_valid: Option<bool>,
    run_code_entry_count: u64,
    return_count: u64,
    result_code: Option<c_int>,
    violation: Option<&'static str>,
}

struct ObservationLedger {
    next_observation_id: u64,
    active: Option<ActiveObservation>,
}

impl Default for ObservationLedger {
    fn default() -> Self {
        Self {
            next_observation_id: 1,
            active: None,
        }
    }
}

fn ledger() -> &'static Mutex<ObservationLedger> {
    static LEDGER: OnceLock<Mutex<ObservationLedger>> = OnceLock::new();
    LEDGER.get_or_init(|| Mutex::new(ObservationLedger::default()))
}

#[must_use = "the guard must be finished to obtain the linear ABI observation receipt"]
pub(in crate::node_agent_compute_plugin_host::local_authority) struct HandleBoundSqliteAbiShmLockObservationGuard
{
    observation_id: u64,
    owner_thread: ThreadId,
    finished: bool,
    not_send_or_sync: PhantomData<Rc<()>>,
}

pub(in crate::node_agent_compute_plugin_host::local_authority) struct HandleBoundSqliteAbiShmLockObservationReceipt
{
    observation_id: u64,
    raw: RawTuple,
    entry_count: u64,
    scalar_rejection_count: u64,
    offset_valid: bool,
    count_valid: bool,
    flags_valid: bool,
    run_code_entry_count: u64,
    return_count: u64,
    result_code: c_int,
}

impl HandleBoundSqliteAbiShmLockObservationReceipt {
    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn observation_id(
        &self,
    ) -> u64 {
        self.observation_id
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn offset(&self) -> c_int {
        self.raw.offset
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn count(&self) -> c_int {
        self.raw.count
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn flags(&self) -> c_int {
        self.raw.flags
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn entry_count(
        &self,
    ) -> u64 {
        self.entry_count
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn scalar_rejection_count(
        &self,
    ) -> u64 {
        self.scalar_rejection_count
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn offset_valid(
        &self,
    ) -> bool {
        self.offset_valid
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn count_valid(
        &self,
    ) -> bool {
        self.count_valid
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn flags_valid(
        &self,
    ) -> bool {
        self.flags_valid
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn run_code_entry_count(
        &self,
    ) -> u64 {
        self.run_code_entry_count
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn return_count(
        &self,
    ) -> u64 {
        self.return_count
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) const fn result_code(
        &self,
    ) -> c_int {
        self.result_code
    }
}

impl HandleBoundSqliteAbiShmLockObservationGuard {
    pub(in crate::node_agent_compute_plugin_host::local_authority) fn finish(
        mut self,
    ) -> Result<HandleBoundSqliteAbiShmLockObservationReceipt, &'static str> {
        self.finished = true;
        let mut ledger = ledger()
            .lock()
            .map_err(|_| "installed xShmLock ABI observation ledger was poisoned")?;
        let Some(active) = ledger.active.take() else {
            return Err("installed xShmLock ABI observation was missing at finish");
        };
        if active.observation_id != self.observation_id {
            ledger.active = Some(active);
            return Err("installed xShmLock ABI observation guard was stale");
        }
        if active.owner_thread != self.owner_thread || thread::current().id() != self.owner_thread {
            return Err("installed xShmLock ABI observation finished on the wrong thread");
        }
        if let Some(violation) = active.violation {
            return Err(violation);
        }
        if active.stage != Stage::Returned
            || active.entry_count != 1
            || active.return_count != 1
            || active.scalar_rejection_count + active.run_code_entry_count != 1
        {
            return Err("installed xShmLock ABI observation sequence was incomplete");
        }
        let result_code = active
            .result_code
            .ok_or("installed xShmLock ABI observation return was missing")?;
        let validation = (active.offset_valid, active.count_valid, active.flags_valid);
        if active.scalar_rejection_count == 1
            && (!matches!(validation, (Some(_), Some(_), Some(_)))
                || validation == (Some(true), Some(true), Some(true)))
        {
            return Err("installed xShmLock scalar rejection validity vector was invalid");
        }
        Ok(HandleBoundSqliteAbiShmLockObservationReceipt {
            observation_id: active.observation_id,
            raw: active.raw,
            entry_count: active.entry_count,
            scalar_rejection_count: active.scalar_rejection_count,
            offset_valid: active.offset_valid.unwrap_or(true),
            count_valid: active.count_valid.unwrap_or(true),
            flags_valid: active.flags_valid.unwrap_or(true),
            run_code_entry_count: active.run_code_entry_count,
            return_count: active.return_count,
            result_code,
        })
    }
}

impl Drop for HandleBoundSqliteAbiShmLockObservationGuard {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        let Ok(mut ledger) = ledger().lock() else {
            return;
        };
        if ledger.active.as_ref().map(|active| active.observation_id) == Some(self.observation_id) {
            ledger.active = None;
        }
    }
}

/// Arms one observation only after proving exact `INERT_IO_METHODS` and typed state identity.
///
/// # Safety
///
/// `file` must be a live, serialized SQLite allocation for the duration of the returned guard.
pub(in crate::node_agent_compute_plugin_host::local_authority) unsafe fn arm_test_x_shm_lock_observation(
    file: *mut ffi::sqlite3_file,
    offset: c_int,
    count: c_int,
    flags: c_int,
) -> Result<HandleBoundSqliteAbiShmLockObservationGuard, &'static str> {
    // SAFETY: the caller provides the serialized live allocation required by this helper.
    if !unsafe {
        super::raw_state::test_vfs_file_has_exact_installed_state::<
            super::file_state::HandleBoundSqliteFileState,
        >(file)
    } {
        return Err("installed xShmLock ABI observation file identity was not exact");
    }
    let mut ledger = ledger()
        .lock()
        .map_err(|_| "installed xShmLock ABI observation ledger was poisoned")?;
    if ledger.active.is_some() {
        return Err("installed xShmLock ABI observation already active");
    }
    let observation_id = ledger.next_observation_id;
    ledger.next_observation_id = observation_id
        .checked_add(1)
        .ok_or("installed xShmLock ABI observation identity exhausted")?;
    if observation_id == 0 {
        return Err("installed xShmLock ABI observation identity was zero");
    }
    let owner_thread = thread::current().id();
    ledger.active = Some(ActiveObservation {
        observation_id,
        file_address: file as usize,
        owner_thread: owner_thread.clone(),
        raw: RawTuple {
            offset,
            count,
            flags,
        },
        stage: Stage::Armed,
        entry_count: 0,
        scalar_rejection_count: 0,
        offset_valid: None,
        count_valid: None,
        flags_valid: None,
        run_code_entry_count: 0,
        return_count: 0,
        result_code: None,
        violation: None,
    });
    Ok(HandleBoundSqliteAbiShmLockObservationGuard {
        observation_id,
        owner_thread,
        finished: false,
        not_send_or_sync: PhantomData,
    })
}

pub(super) fn record_entry(
    file: *mut ffi::sqlite3_file,
    offset: c_int,
    count: c_int,
    flags: c_int,
) {
    with_matching_active(file, offset, count, flags, |active| {
        if active.stage != Stage::Armed || active.entry_count != 0 {
            mark_violation(
                active,
                "installed xShmLock ABI entry was duplicated or reordered",
            );
            return;
        }
        active.entry_count = 1;
        active.stage = Stage::Entered;
    });
}

pub(super) fn record_scalar_rejected(
    file: *mut ffi::sqlite3_file,
    offset: c_int,
    count: c_int,
    flags: c_int,
    offset_valid: bool,
    count_valid: bool,
    flags_valid: bool,
) {
    with_matching_active(file, offset, count, flags, |active| {
        if active.stage != Stage::Entered || active.scalar_rejection_count != 0 {
            mark_violation(
                active,
                "installed xShmLock scalar rejection was duplicated or reordered",
            );
            return;
        }
        active.scalar_rejection_count = 1;
        active.offset_valid = Some(offset_valid);
        active.count_valid = Some(count_valid);
        active.flags_valid = Some(flags_valid);
        active.stage = Stage::ScalarRejected;
    });
}

pub(super) fn record_run_code_entry(
    file: *mut ffi::sqlite3_file,
    offset: c_int,
    count: c_int,
    flags: c_int,
) {
    with_matching_active(file, offset, count, flags, |active| {
        if active.stage != Stage::Entered || active.run_code_entry_count != 0 {
            mark_violation(
                active,
                "installed xShmLock run_code entry was duplicated or reordered",
            );
            return;
        }
        active.run_code_entry_count = 1;
        active.stage = Stage::RunCodeEntered;
    });
}

pub(super) fn record_returned(
    file: *mut ffi::sqlite3_file,
    offset: c_int,
    count: c_int,
    flags: c_int,
    result_code: c_int,
) {
    with_matching_active(file, offset, count, flags, |active| {
        if !matches!(active.stage, Stage::ScalarRejected | Stage::RunCodeEntered)
            || active.return_count != 0
            || active.result_code.is_some()
        {
            mark_violation(
                active,
                "installed xShmLock return was duplicated or reordered",
            );
            return;
        }
        active.return_count = 1;
        active.result_code = Some(result_code);
        active.stage = Stage::Returned;
    });
}

fn with_matching_active(
    file: *mut ffi::sqlite3_file,
    offset: c_int,
    count: c_int,
    flags: c_int,
    event: impl FnOnce(&mut ActiveObservation),
) {
    let Ok(mut ledger) = ledger().lock() else {
        return;
    };
    let Some(active) = ledger.active.as_mut() else {
        return;
    };
    if active.violation.is_some() {
        return;
    }
    if active.file_address != file as usize {
        mark_violation(active, "installed xShmLock ABI observation file mismatch");
        return;
    }
    if active.owner_thread != thread::current().id() {
        mark_violation(active, "installed xShmLock ABI observation thread mismatch");
        return;
    }
    if active.raw
        != (RawTuple {
            offset,
            count,
            flags,
        })
    {
        mark_violation(
            active,
            "installed xShmLock ABI observation raw tuple mismatch",
        );
        return;
    }
    event(active);
}

fn mark_violation(active: &mut ActiveObservation, violation: &'static str) {
    if active.violation.is_none() {
        active.violation = Some(violation);
    }
}
