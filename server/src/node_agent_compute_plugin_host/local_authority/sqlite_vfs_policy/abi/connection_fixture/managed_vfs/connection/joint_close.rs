//! Exact installed xClose capture for process-isolated JointClose evidence.

use std::os::raw::c_int;

use rusqlite::ffi;

use super::ManagedSqliteRoutedConnectionFixture;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi::{
    arm_test_vfs_file_raw_state_take_rejection, observe_test_vfs_file_raw_close_witness,
    observe_test_vfs_file_raw_slots, HandleBoundSqliteAbiRawCloseWitness,
    HandleBoundSqliteAbiRawSlotSnapshot,
};

/// Saved real callback and its exact still-live SQLite allocation.
///
/// The child retains the owning fixture until process exit after the first invocation. That keeps
/// the allocation valid even though real xClose clears `pMethods` and the typed state slot.
pub(in super::super) struct ManagedTestCapturedMainCloseCall {
    file: *mut ffi::sqlite3_file,
    close: unsafe extern "C" fn(*mut ffi::sqlite3_file) -> c_int,
}

impl ManagedTestCapturedMainCloseCall {
    /// # Safety
    ///
    /// The captured allocation must still be owned by its original fixture, the call must be
    /// serialized, and the caller must retain that owner after xClose clears the installed slots.
    pub(in super::super) unsafe fn invoke(&self) -> c_int {
        unsafe { (self.close)(self.file) }
    }

    /// # Safety
    ///
    /// The original fixture must still exclusively own this installed allocation.
    pub(in super::super) unsafe fn arm_raw_state_take_rejection(
        &self,
    ) -> Result<HandleBoundSqliteAbiRawCloseWitness, &'static str> {
        unsafe { arm_test_vfs_file_raw_state_take_rejection(self.file) }
            .ok_or("JointClose exact raw-state-take rejection arm was refused")
    }

    /// # Safety
    ///
    /// The original fixture must still own the allocation even if its installed slots are clear.
    pub(in super::super) unsafe fn observe_raw_slots(
        &self,
    ) -> Result<HandleBoundSqliteAbiRawSlotSnapshot, &'static str> {
        unsafe { observe_test_vfs_file_raw_slots(self.file) }
            .ok_or("JointClose exact raw-slot observation is unavailable")
    }

    /// # Safety
    ///
    /// The original fixture must still exclusively own this installed allocation.
    pub(in super::super) unsafe fn raw_close_witness(
        &self,
    ) -> Result<HandleBoundSqliteAbiRawCloseWitness, &'static str> {
        unsafe { observe_test_vfs_file_raw_close_witness(self.file) }
            .ok_or("JointClose exact raw-close witness is unavailable")
    }

    /// # Safety
    ///
    /// The original fixture must still exclusively own this installed allocation before xClose.
    pub(in super::super) unsafe fn acquire_main_lock_prestate(
        &self,
        reserved: bool,
    ) -> Result<(), &'static str> {
        // SAFETY: this runs before xClose while the captured allocation still owns its table.
        let methods = unsafe { (*self.file).pMethods };
        if methods.is_null() {
            return Err("JointClose main-file methods cleared before lock prestate");
        }
        // SAFETY: the installed table belongs to this exact allocation.
        let lock = unsafe { (*methods).xLock }
            .ok_or("JointClose main-file xLock callback is unavailable")?;
        // SAFETY: the owning fixture serializes these canonical SQLite lock transitions.
        if unsafe { lock(self.file, ffi::SQLITE_LOCK_SHARED) } != ffi::SQLITE_OK {
            return Err("JointClose failed to acquire Shared main-file prestate");
        }
        if reserved
            // SAFETY: Shared was acquired above and SQLite permits the Reserved transition.
            && unsafe { lock(self.file, ffi::SQLITE_LOCK_RESERVED) } != ffi::SQLITE_OK
        {
            return Err("JointClose failed to acquire Reserved main-file prestate");
        }
        Ok(())
    }
}

impl ManagedSqliteRoutedConnectionFixture {
    pub(in super::super) fn capture_main_close_call(
        &self,
    ) -> Result<ManagedTestCapturedMainCloseCall, &'static str> {
        let file = self.main_file_pointer()?;
        // SAFETY: `main_file_pointer` returned the live allocation owned by this fixture.
        let methods = unsafe { (*file).pMethods };
        if methods.is_null() {
            return Err("JointClose main-file methods are not installed");
        }
        // SAFETY: the installed table belongs to this same live allocation.
        let close = unsafe { (*methods).xClose }
            .ok_or("JointClose main-file xClose callback is unavailable")?;
        Ok(ManagedTestCapturedMainCloseCall { file, close })
    }
}
