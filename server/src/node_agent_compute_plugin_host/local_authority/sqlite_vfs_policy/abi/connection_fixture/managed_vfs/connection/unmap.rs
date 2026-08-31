//! Direct, installed-ABI Unmap and SHM-lock calls for Windows dynamic evidence.

use std::{
    fmt,
    os::raw::{c_int, c_void},
    ptr,
};

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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ManagedTestShmMapCallbackObservation {
    region: c_int,
    region_size: c_int,
    raw_extend: c_int,
    result_code: c_int,
    output: *mut c_void,
    output_was_cleared: bool,
    before: HandleBoundSqliteAbiRawSlotSnapshot,
    after: HandleBoundSqliteAbiRawSlotSnapshot,
}

impl fmt::Debug for ManagedTestShmMapCallbackObservation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedTestShmMapCallbackObservation")
            .field("region", &self.region)
            .field("region_size", &self.region_size)
            .field("raw_extend", &self.raw_extend)
            .field("result_code", &self.result_code)
            .field(
                "output",
                &if self.output.is_null() {
                    "<null>"
                } else {
                    "<mapped>"
                },
            )
            .field("output_was_cleared", &self.output_was_cleared)
            .field("before", &self.before)
            .field("after", &self.after)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ManagedTestShmLockCallbackObservation {
    offset: c_int,
    count: c_int,
    raw_flags: c_int,
    result_code: c_int,
    before: HandleBoundSqliteAbiRawSlotSnapshot,
    after: HandleBoundSqliteAbiRawSlotSnapshot,
}

impl ManagedTestShmLockCallbackObservation {
    pub(in super::super) fn offset(self) -> c_int {
        self.offset
    }

    pub(in super::super) fn count(self) -> c_int {
        self.count
    }

    pub(in super::super) fn raw_flags(self) -> c_int {
        self.raw_flags
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

impl ManagedTestShmMapCallbackObservation {
    pub(in super::super) fn region(self) -> c_int {
        self.region
    }

    pub(in super::super) fn region_size(self) -> c_int {
        self.region_size
    }

    pub(in super::super) fn raw_extend(self) -> c_int {
        self.raw_extend
    }

    pub(in super::super) fn result_code(self) -> c_int {
        self.result_code
    }

    /// Returns the installed ABI's raw output for same-process equality checks only.
    /// The pointer must never be dereferenced, serialized, logged, or hashed by the harness.
    pub(in super::super) fn output_pointer(self) -> *mut c_void {
        self.output
    }

    pub(in super::super) fn output_was_cleared(self) -> bool {
        self.output_was_cleared
    }

    pub(in super::super) fn before(self) -> HandleBoundSqliteAbiRawSlotSnapshot {
        self.before
    }

    pub(in super::super) fn after(self) -> HandleBoundSqliteAbiRawSlotSnapshot {
        self.after
    }
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
    pub(in super::super) fn call_main_shm_map_raw(
        &self,
        region: c_int,
        region_size: c_int,
        raw_extend: c_int,
    ) -> Result<ManagedTestShmMapCallbackObservation, &'static str> {
        let file = self.main_file_pointer()?;
        // SAFETY: `main_file_pointer` returned this test VFS's live serialized allocation.
        let before = unsafe { super::super::observe_test_vfs_file_raw_slots(file) }
            .ok_or("managed SHM-map raw slots unavailable before callback")?;
        if !before.methods_installed || !before.state_installed {
            return Err("managed SHM-map raw state was not installed before callback");
        }
        // SAFETY: the live allocation owns the installed method table observed above.
        let methods = unsafe { (*file).pMethods };
        if methods.is_null() {
            return Err("managed SHM-map method table is unavailable");
        }
        // SAFETY: `methods` belongs to the same live main-file allocation.
        let map = unsafe { (*methods).xShmMap }.ok_or("managed SHM-map callback is unavailable")?;
        // A known non-null, never-dereferenced sentinel proves that the installed ABI entry clear
        // ran; starting with null would make a skipped callback indistinguishable from success.
        let mut output = ptr::without_provenance_mut::<c_void>(1);
        // SAFETY: the callback receives its owning live sqlite3_file and a writable local output
        // slot. The raw request deliberately reaches the installed ABI and production budget
        // validator instead of projecting a harness-only node-absent state.
        let result_code = unsafe { map(file, region, region_size, raw_extend, &mut output) };
        // SAFETY: the allocation remains owned by the live Connection after either installed Map
        // outcome; only the callback's output slot can carry a mapped address.
        let after = unsafe { super::super::observe_test_vfs_file_raw_slots(file) }
            .ok_or("managed SHM-map raw slots unavailable after callback")?;
        Ok(ManagedTestShmMapCallbackObservation {
            region,
            region_size,
            raw_extend,
            result_code,
            output,
            output_was_cleared: output.is_null(),
            before,
            after,
        })
    }

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
        self.observe_main_shm_lock_raw(offset, count, flags)
            .map(ManagedTestShmLockCallbackObservation::result_code)
    }

    pub(in super::super) fn observe_main_shm_lock_raw(
        &self,
        offset: c_int,
        count: c_int,
        raw_flags: c_int,
    ) -> Result<ManagedTestShmLockCallbackObservation, &'static str> {
        let file = self.main_file_pointer()?;
        // SAFETY: `main_file_pointer` returned this test VFS's live main-file allocation.
        let before = unsafe { super::super::observe_test_vfs_file_raw_slots(file) }
            .ok_or("managed SHM-lock raw slots unavailable before callback")?;
        if !before.methods_installed || !before.state_installed {
            return Err("managed SHM-lock raw state was not installed before callback");
        }
        // SAFETY: the live allocation owns the installed method table observed above.
        let methods = unsafe { (*file).pMethods };
        if methods.is_null() {
            return Err("managed SHM-lock method table is unavailable");
        }
        // SAFETY: `methods` belongs to the same live main-file allocation.
        let lock =
            unsafe { (*methods).xShmLock }.ok_or("managed SHM-lock callback is unavailable")?;
        // SAFETY: this calls SQLite's installed xShmLock with its owning live file. The installed
        // callback does not transfer or destroy that allocation on either success or failure.
        let result_code = unsafe { lock(file, offset, count, raw_flags) };
        // SAFETY: the same live Connection still owns the allocation after the callback.
        let after = unsafe { super::super::observe_test_vfs_file_raw_slots(file) }
            .ok_or("managed SHM-lock raw slots unavailable after callback")?;
        Ok(ManagedTestShmLockCallbackObservation {
            offset,
            count,
            raw_flags,
            result_code,
            before,
            after,
        })
    }

    pub(in super::super) fn call_main_file_lock_exclusive(&self) -> Result<c_int, &'static str> {
        let file = self.main_file_pointer()?;
        // SAFETY: `main_file_pointer` returned this fixture's live serialized main-file
        // allocation, and the installed method table belongs to that same allocation.
        let methods = unsafe { (*file).pMethods };
        if methods.is_null() {
            return Err("managed main-file lock method table is unavailable");
        }
        // SAFETY: the callback receives its owning live file and SQLite's canonical lock level.
        let lock = unsafe { (*methods).xLock }.ok_or("managed main-file xLock is unavailable")?;
        Ok(unsafe { lock(file, ffi::SQLITE_LOCK_EXCLUSIVE) })
    }

    pub(in super::super) fn quarantine_for_unmap_admission_test(&self) -> Result<(), &'static str> {
        self.route
            .retain_failure("managed Unmap admission rejection sentinel")
            .map_err(|()| "managed Unmap admission route quarantine failed")
    }
}
