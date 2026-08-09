use std::{
    num::{NonZeroU32, NonZeroU8},
    os::raw::{c_int, c_void},
};

use rusqlite::ffi;

use super::{boundary, file_state, result_codes};
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::{
    HandleBoundSqliteAbiAttempt, HandleBoundSqliteAbiShmLockAction, HandleBoundSqliteAbiShmMap,
};

pub(super) unsafe extern "C" fn map(
    file: *mut ffi::sqlite3_file,
    region: c_int,
    region_size: c_int,
    extend: c_int,
    output: *mut *mut c_void,
) -> c_int {
    // SAFETY: SQLite supplies this output allocation when non-null.
    unsafe { boundary::write_pointer_null(output) };
    let (Ok(region), Some(region_size), Some(extend)) = (
        u32::try_from(region),
        u32::try_from(region_size).ok().and_then(NonZeroU32::new),
        sqlite_bool(extend),
    ) else {
        return result_codes::SHM_MAP_UNAVAILABLE;
    };
    if output.is_null() {
        return result_codes::SHM_MAP_UNAVAILABLE;
    }
    // SAFETY: the callback contract serializes this file and the output remains writable.
    unsafe {
        file_state::run_code(
            file,
            result_codes::SHM_MAP_UNAVAILABLE,
            |state| match state.shm_map(region, region_size, extend) {
                Ok(HandleBoundSqliteAbiShmMap::NotPresent) => ffi::SQLITE_OK,
                Ok(HandleBoundSqliteAbiShmMap::Mapped(pointer)) => {
                    output.write(pointer.as_ptr());
                    ffi::SQLITE_OK
                }
                Err(()) => result_codes::SHM_MAP_UNAVAILABLE,
            },
        )
    }
}

pub(super) unsafe extern "C" fn lock(
    file: *mut ffi::sqlite3_file,
    offset: c_int,
    count: c_int,
    flags: c_int,
) -> c_int {
    let (Ok(offset), Some(count), Some(action)) = (
        u8::try_from(offset),
        u8::try_from(count).ok().and_then(NonZeroU8::new),
        shm_lock_action(flags),
    ) else {
        return result_codes::SHM_LOCK_UNAVAILABLE;
    };
    // SAFETY: the callback contract serializes this exact file allocation.
    unsafe {
        file_state::run_code(
            file,
            result_codes::SHM_LOCK_UNAVAILABLE,
            |state| match state.shm_lock(offset, count, action) {
                Ok(HandleBoundSqliteAbiAttempt::Acquired) => ffi::SQLITE_OK,
                Ok(HandleBoundSqliteAbiAttempt::Busy) => ffi::SQLITE_BUSY,
                Err(()) => result_codes::SHM_LOCK_UNAVAILABLE,
            },
        )
    }
}

pub(super) unsafe extern "C" fn barrier(file: *mut ffi::sqlite3_file) {
    // xShmBarrier has no result-code channel. Failure therefore removes the callback table and
    // drops the state into its existing terminal-custody path.
    // SAFETY: the callback contract serializes this exact file allocation.
    unsafe { file_state::run_void(file, |state| state.shm_barrier()) };
}

pub(super) unsafe extern "C" fn unmap(file: *mut ffi::sqlite3_file, delete: c_int) -> c_int {
    let Some(delete) = sqlite_bool(delete) else {
        return result_codes::SHM_UNMAP_UNAVAILABLE;
    };
    // SAFETY: the callback contract serializes this exact file allocation.
    unsafe {
        file_state::run_code(
            file,
            result_codes::SHM_UNMAP_UNAVAILABLE,
            |state| match state.shm_unmap(delete) {
                Ok(()) => ffi::SQLITE_OK,
                Err(()) => result_codes::SHM_UNMAP_UNAVAILABLE,
            },
        )
    }
}

fn sqlite_bool(value: c_int) -> Option<bool> {
    match value {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

fn shm_lock_action(flags: c_int) -> Option<HandleBoundSqliteAbiShmLockAction> {
    match flags {
        value if value == ffi::SQLITE_SHM_LOCK | ffi::SQLITE_SHM_SHARED => {
            Some(HandleBoundSqliteAbiShmLockAction::LockShared)
        }
        value if value == ffi::SQLITE_SHM_LOCK | ffi::SQLITE_SHM_EXCLUSIVE => {
            Some(HandleBoundSqliteAbiShmLockAction::LockExclusive)
        }
        value if value == ffi::SQLITE_SHM_UNLOCK | ffi::SQLITE_SHM_SHARED => {
            Some(HandleBoundSqliteAbiShmLockAction::UnlockShared)
        }
        value if value == ffi::SQLITE_SHM_UNLOCK | ffi::SQLITE_SHM_EXCLUSIVE => {
            Some(HandleBoundSqliteAbiShmLockAction::UnlockExclusive)
        }
        _ => None,
    }
}
