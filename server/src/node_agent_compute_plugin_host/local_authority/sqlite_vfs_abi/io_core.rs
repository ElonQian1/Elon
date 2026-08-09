use std::{
    os::raw::{c_int, c_void},
    slice,
};

use rusqlite::ffi;

use super::{boundary, file_state, result_codes};
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::{
    HandleBoundSqliteAbiAttempt, HandleBoundSqliteAbiLockLevel, HandleBoundSqliteAbiUnlockLevel,
};

pub(super) unsafe extern "C" fn close(file: *mut ffi::sqlite3_file) -> c_int {
    // SAFETY: xClose has exclusive consuming access to this SQLite file allocation.
    unsafe { file_state::close(file, result_codes::CLOSE_UNAVAILABLE) }
}

pub(super) unsafe extern "C" fn read(
    file: *mut ffi::sqlite3_file,
    output: *mut c_void,
    amount: c_int,
    offset: ffi::sqlite3_int64,
) -> c_int {
    // SQLite requires the unread suffix to be zero even when the underlying operation fails.
    // SAFETY: SQLite supplies this writable range for non-negative amounts.
    if !unsafe { boundary::zero_bytes(output, amount) } {
        return result_codes::READ_UNAVAILABLE;
    }
    let (Ok(length), Ok(offset)) = (usize::try_from(amount), u64::try_from(offset)) else {
        return result_codes::READ_UNAVAILABLE;
    };
    // SAFETY: the callback contract serializes this file and supplies `length` writable bytes.
    unsafe {
        file_state::run_code(file, result_codes::READ_UNAVAILABLE, |state| {
            let buffer = if length == 0 {
                &mut []
            } else {
                slice::from_raw_parts_mut(output.cast::<u8>(), length)
            };
            match state.read_at_zero_filled(offset, buffer) {
                Ok(read) if read == length => ffi::SQLITE_OK,
                Ok(read) if read < length => ffi::SQLITE_IOERR_SHORT_READ,
                Ok(_) | Err(()) => result_codes::READ_UNAVAILABLE,
            }
        })
    }
}

pub(super) unsafe extern "C" fn write(
    file: *mut ffi::sqlite3_file,
    input: *const c_void,
    amount: c_int,
    offset: ffi::sqlite3_int64,
) -> c_int {
    let (Ok(length), Ok(offset)) = (usize::try_from(amount), u64::try_from(offset)) else {
        return result_codes::WRITE_UNAVAILABLE;
    };
    if length != 0 && input.is_null() {
        return result_codes::WRITE_UNAVAILABLE;
    }
    // SAFETY: the callback contract serializes this file and supplies `length` readable bytes.
    unsafe {
        file_state::run_code(file, result_codes::WRITE_UNAVAILABLE, |state| {
            let bytes = if length == 0 {
                &[]
            } else {
                slice::from_raw_parts(input.cast::<u8>(), length)
            };
            match state.write_all_at(offset, bytes) {
                Ok(()) => ffi::SQLITE_OK,
                Err(()) => result_codes::WRITE_UNAVAILABLE,
            }
        })
    }
}

pub(super) unsafe extern "C" fn truncate(
    file: *mut ffi::sqlite3_file,
    size: ffi::sqlite3_int64,
) -> c_int {
    let Ok(size) = u64::try_from(size) else {
        return result_codes::TRUNCATE_UNAVAILABLE;
    };
    // SAFETY: the callback contract serializes this exact file allocation.
    unsafe {
        file_state::run_code(
            file,
            result_codes::TRUNCATE_UNAVAILABLE,
            |state| match state.truncate(size) {
                Ok(()) => ffi::SQLITE_OK,
                Err(()) => result_codes::TRUNCATE_UNAVAILABLE,
            },
        )
    }
}

pub(super) unsafe extern "C" fn sync(file: *mut ffi::sqlite3_file, flags: c_int) -> c_int {
    if !valid_sync_flags(flags) {
        return result_codes::SYNC_UNAVAILABLE;
    }
    // Full flushing safely satisfies both SQLite NORMAL/FULL and DATAONLY requests.
    // SAFETY: the callback contract serializes this exact file allocation.
    unsafe {
        file_state::run_code(file, result_codes::SYNC_UNAVAILABLE, |state| {
            match state.full_sync() {
                Ok(()) => ffi::SQLITE_OK,
                Err(()) => result_codes::SYNC_UNAVAILABLE,
            }
        })
    }
}

pub(super) unsafe extern "C" fn file_size(
    file: *mut ffi::sqlite3_file,
    output: *mut ffi::sqlite3_int64,
) -> c_int {
    // SAFETY: SQLite supplies this output allocation when non-null.
    unsafe { boundary::write_i64_zero(output) };
    if output.is_null() {
        return result_codes::FILE_SIZE_UNAVAILABLE;
    }
    // SAFETY: the callback contract serializes this file and the output remains writable.
    unsafe {
        file_state::run_code(file, result_codes::FILE_SIZE_UNAVAILABLE, |state| {
            let Ok(size) = state
                .size()
                .and_then(|size| i64::try_from(size).map_err(drop))
            else {
                return result_codes::FILE_SIZE_UNAVAILABLE;
            };
            output.write(size);
            ffi::SQLITE_OK
        })
    }
}

pub(super) unsafe extern "C" fn lock(file: *mut ffi::sqlite3_file, level: c_int) -> c_int {
    let level = match level {
        ffi::SQLITE_LOCK_SHARED => HandleBoundSqliteAbiLockLevel::Shared,
        ffi::SQLITE_LOCK_RESERVED => HandleBoundSqliteAbiLockLevel::Reserved,
        ffi::SQLITE_LOCK_EXCLUSIVE => HandleBoundSqliteAbiLockLevel::Exclusive,
        _ => return result_codes::LOCK_UNAVAILABLE,
    };
    // SAFETY: the callback contract serializes this exact file allocation.
    unsafe {
        file_state::run_code(file, result_codes::LOCK_UNAVAILABLE, |state| {
            match state.lock_to(level) {
                Ok(HandleBoundSqliteAbiAttempt::Acquired) => ffi::SQLITE_OK,
                Ok(HandleBoundSqliteAbiAttempt::Busy) => ffi::SQLITE_BUSY,
                Err(()) => result_codes::LOCK_UNAVAILABLE,
            }
        })
    }
}

pub(super) unsafe extern "C" fn unlock(file: *mut ffi::sqlite3_file, level: c_int) -> c_int {
    let level = match level {
        ffi::SQLITE_LOCK_NONE => HandleBoundSqliteAbiUnlockLevel::None,
        ffi::SQLITE_LOCK_SHARED => HandleBoundSqliteAbiUnlockLevel::Shared,
        _ => return result_codes::UNLOCK_UNAVAILABLE,
    };
    // SAFETY: the callback contract serializes this exact file allocation.
    unsafe {
        file_state::run_code(file, result_codes::UNLOCK_UNAVAILABLE, |state| match state
            .unlock_to(level)
        {
            Ok(()) => ffi::SQLITE_OK,
            Err(()) => result_codes::UNLOCK_UNAVAILABLE,
        })
    }
}

pub(super) unsafe extern "C" fn check_reserved_lock(
    file: *mut ffi::sqlite3_file,
    output: *mut c_int,
) -> c_int {
    // SAFETY: SQLite supplies this output allocation when non-null.
    unsafe { boundary::write_int_zero(output) };
    if output.is_null() {
        return result_codes::RESERVED_LOCK_UNAVAILABLE;
    }
    // SAFETY: the callback contract serializes this file and the output remains writable.
    unsafe {
        file_state::run_code(
            file,
            result_codes::RESERVED_LOCK_UNAVAILABLE,
            |state| match state.check_reserved_lock() {
                Ok(held) => {
                    output.write(c_int::from(held));
                    ffi::SQLITE_OK
                }
                Err(()) => result_codes::RESERVED_LOCK_UNAVAILABLE,
            },
        )
    }
}

pub(super) unsafe extern "C" fn file_control(
    _file: *mut ffi::sqlite3_file,
    _operation: c_int,
    _argument: *mut c_void,
) -> c_int {
    boundary::catch_code(result_codes::FILE_CONTROL_UNSUPPORTED, || {
        result_codes::FILE_CONTROL_UNSUPPORTED
    })
}

pub(super) unsafe extern "C" fn sector_size(_file: *mut ffi::sqlite3_file) -> c_int {
    boundary::catch_value(0, || 0)
}

pub(super) unsafe extern "C" fn device_characteristics(_file: *mut ffi::sqlite3_file) -> c_int {
    boundary::catch_value(0, || 0)
}

fn valid_sync_flags(flags: c_int) -> bool {
    [
        ffi::SQLITE_SYNC_NORMAL,
        ffi::SQLITE_SYNC_FULL,
        ffi::SQLITE_SYNC_NORMAL | ffi::SQLITE_SYNC_DATAONLY,
        ffi::SQLITE_SYNC_FULL | ffi::SQLITE_SYNC_DATAONLY,
    ]
    .contains(&flags)
}
