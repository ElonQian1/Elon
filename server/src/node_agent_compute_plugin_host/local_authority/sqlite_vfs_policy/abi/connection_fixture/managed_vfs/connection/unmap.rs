//! Direct, installed-ABI Unmap and SHM-lock calls for Windows dynamic evidence.

use std::os::raw::c_int;

use rusqlite::ffi;

use super::ManagedSqliteRoutedConnectionFixture;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi::HandleBoundSqliteAbiRawSlotSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ManagedTestUnmapCallbackObservation {
    raw_delete: c_int,
    result_code: c_int,
    before: HandleBoundSqliteAbiRawSlotSnapshot,
    after: HandleBoundSqliteAbiRawSlotSnapshot,
}

impl ManagedTestUnmapCallbackObservation {
    pub(in super::super) fn raw_delete(self) -> c_int {
        self.raw_delete
    }

    pub(in super::super) fn result_code(self) -> c_int {
        self.result_code
    }

    pub(in super::super) fn before(self) -> HandleBoundSqliteAbiRawSlotSnapshot {
        self.before
    }

    pub(in super::super) fn after(self) -> HandleBoundSqliteAbiRawSlotSnapshot {
        self.after
    }
}

impl ManagedSqliteRoutedConnectionFixture {
    pub(in super::super) fn call_main_shm_unmap_raw(
        &self,
        raw_delete: c_int,
    ) -> Result<ManagedTestUnmapCallbackObservation, &'static str> {
        let file = self.main_file_pointer()?;
        // SAFETY: `main_file_pointer` returned this test VFS's live serialized allocation.
        let before = unsafe { super::super::observe_test_vfs_file_raw_slots(file) }
            .ok_or("managed Unmap raw slots unavailable before callback")?;
        if !before.methods_installed || !before.state_installed {
            return Err("managed Unmap raw state was not installed before callback");
        }
        // SAFETY: the live allocation owns the installed method table observed above.
        let methods = unsafe { (*file).pMethods };
        if methods.is_null() {
            return Err("managed Unmap method table is unavailable");
        }
        // SAFETY: `methods` belongs to the same live main-file allocation.
        let unmap =
            unsafe { (*methods).xShmUnmap }.ok_or("managed Unmap callback is unavailable")?;
        // SAFETY: the callback receives its owning live sqlite3_file. `raw_delete` deliberately
        // remains unnormalized so 0, 1, and invalid 2 all traverse the installed ABI validator.
        let result_code = unsafe { unmap(file, raw_delete) };
        // SAFETY: the allocation remains owned by SQLite even when the callback fail-closes and
        // clears its installed ownership slots.
        let after = unsafe { super::super::observe_test_vfs_file_raw_slots(file) }
            .ok_or("managed Unmap raw slots unavailable after callback")?;
        Ok(ManagedTestUnmapCallbackObservation {
            raw_delete,
            result_code,
            before,
            after,
        })
    }

    pub(in super::super) fn call_main_shm_unmap_keep(&self) -> c_int {
        self.call_main_shm_unmap_raw(0)
            .map(ManagedTestUnmapCallbackObservation::result_code)
            .unwrap_or(ffi::SQLITE_IOERR)
    }

    pub(in super::super) fn call_main_shm_lock_raw(
        &self,
        offset: c_int,
        count: c_int,
        flags: c_int,
    ) -> Result<c_int, &'static str> {
        let file = self.main_file_pointer()?;
        // SAFETY: `main_file_pointer` returned this test VFS's live main-file allocation.
        let methods = unsafe { (*file).pMethods };
        if methods.is_null() {
            return Err("managed SHM-lock method table is unavailable");
        }
        // SAFETY: `methods` belongs to the same live main-file allocation.
        let lock =
            unsafe { (*methods).xShmLock }.ok_or("managed SHM-lock callback is unavailable")?;
        // SAFETY: this calls SQLite's installed xShmLock with its owning live file. Callers use
        // canonical SQLite flags and inspect the coordinator snapshot after the call.
        Ok(unsafe { lock(file, offset, count, flags) })
    }

    pub(in super::super) fn quarantine_for_unmap_admission_test(&self) -> Result<(), &'static str> {
        self.route
            .retain_failure("managed Unmap admission rejection sentinel")
            .map_err(|()| "managed Unmap admission route quarantine failed")
    }
}
