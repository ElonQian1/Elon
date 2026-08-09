use std::os::raw::{c_int, c_void};

use rusqlite::ffi;

use super::{boundary, raw_state, result_codes};

pub(super) unsafe extern "C" fn close(file: *mut ffi::sqlite3_file) -> c_int {
    boundary::catch_code(result_codes::CLOSE_UNAVAILABLE, || {
        // No production path installs state yet. A test or future partial adapter that reaches
        // this inert callback still loses its callback path before payload Drop fail-closes its
        // custody; a proper live xClose must instead take the typed state and call explicit close.
        // SAFETY: the callback contract supplies an initialized file allocation when non-null.
        unsafe {
            let _ = raw_state::abandon_installed_state(file);
        }
        result_codes::CLOSE_UNAVAILABLE
    })
}

pub(super) unsafe extern "C" fn read(
    _file: *mut ffi::sqlite3_file,
    output: *mut c_void,
    amount: c_int,
    _offset: ffi::sqlite3_int64,
) -> c_int {
    // SQLite requires the unread part of a short read to be zero. This callback never reads, so it
    // conservatively clears the complete requested range before reporting a hard read failure.
    // SAFETY: the SQLite callback contract supplies this byte range when non-null and positive.
    unsafe {
        let _ = boundary::zero_bytes(output, amount);
    }
    boundary::catch_code(result_codes::READ_UNAVAILABLE, || {
        result_codes::READ_UNAVAILABLE
    })
}

pub(super) unsafe extern "C" fn write(
    _file: *mut ffi::sqlite3_file,
    _input: *const c_void,
    _amount: c_int,
    _offset: ffi::sqlite3_int64,
) -> c_int {
    boundary::catch_code(result_codes::WRITE_UNAVAILABLE, || {
        result_codes::WRITE_UNAVAILABLE
    })
}

pub(super) unsafe extern "C" fn truncate(
    _file: *mut ffi::sqlite3_file,
    _size: ffi::sqlite3_int64,
) -> c_int {
    boundary::catch_code(result_codes::TRUNCATE_UNAVAILABLE, || {
        result_codes::TRUNCATE_UNAVAILABLE
    })
}

pub(super) unsafe extern "C" fn sync(_file: *mut ffi::sqlite3_file, _flags: c_int) -> c_int {
    boundary::catch_code(result_codes::SYNC_UNAVAILABLE, || {
        result_codes::SYNC_UNAVAILABLE
    })
}

pub(super) unsafe extern "C" fn file_size(
    _file: *mut ffi::sqlite3_file,
    output: *mut ffi::sqlite3_int64,
) -> c_int {
    // SAFETY: the SQLite callback contract supplies this output allocation when non-null.
    unsafe { boundary::write_i64_zero(output) };
    boundary::catch_code(result_codes::FILE_SIZE_UNAVAILABLE, || {
        result_codes::FILE_SIZE_UNAVAILABLE
    })
}

pub(super) unsafe extern "C" fn lock(_file: *mut ffi::sqlite3_file, _level: c_int) -> c_int {
    boundary::catch_code(result_codes::LOCK_UNAVAILABLE, || {
        result_codes::LOCK_UNAVAILABLE
    })
}

pub(super) unsafe extern "C" fn unlock(_file: *mut ffi::sqlite3_file, _level: c_int) -> c_int {
    boundary::catch_code(result_codes::UNLOCK_UNAVAILABLE, || {
        result_codes::UNLOCK_UNAVAILABLE
    })
}

pub(super) unsafe extern "C" fn check_reserved_lock(
    _file: *mut ffi::sqlite3_file,
    output: *mut c_int,
) -> c_int {
    // SAFETY: the SQLite callback contract supplies this output allocation when non-null.
    unsafe { boundary::write_int_zero(output) };
    boundary::catch_code(result_codes::RESERVED_LOCK_UNAVAILABLE, || {
        result_codes::RESERVED_LOCK_UNAVAILABLE
    })
}

pub(super) unsafe extern "C" fn file_control(
    _file: *mut ffi::sqlite3_file,
    _operation: c_int,
    _argument: *mut c_void,
) -> c_int {
    // SQLite requires unrecognized control operations to report SQLITE_NOTFOUND.
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
