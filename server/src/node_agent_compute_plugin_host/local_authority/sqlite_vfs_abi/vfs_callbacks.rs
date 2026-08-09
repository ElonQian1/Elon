use std::os::raw::{c_char, c_int, c_void};

use rusqlite::ffi;

use super::{
    boundary, raw_state,
    result_codes::{
        ACCESS_UNAVAILABLE, CURRENT_TIME_UNAVAILABLE, DELETE_UNAVAILABLE,
        FULL_PATHNAME_UNAVAILABLE, VFS_UNAVAILABLE,
    },
};

pub(super) unsafe extern "C" fn open(
    _vfs: *mut ffi::sqlite3_vfs,
    _name: ffi::sqlite3_filename,
    file: *mut ffi::sqlite3_file,
    _flags: c_int,
    output_flags: *mut c_int,
) -> c_int {
    // SQLite may call xClose after a failed xOpen only when pMethods is non-null. Clear it before
    // any fallible work, and never install the inert I/O table.
    // SAFETY: the SQLite callback contract supplies these output allocations when non-null.
    unsafe {
        let _ = raw_state::initialize_fresh_file(file);
        boundary::write_int_zero(output_flags);
    }
    boundary::catch_code(VFS_UNAVAILABLE, || VFS_UNAVAILABLE)
}

pub(super) unsafe extern "C" fn delete(
    _vfs: *mut ffi::sqlite3_vfs,
    _name: *const c_char,
    _sync_directory: c_int,
) -> c_int {
    boundary::catch_code(DELETE_UNAVAILABLE, || DELETE_UNAVAILABLE)
}

pub(super) unsafe extern "C" fn access(
    _vfs: *mut ffi::sqlite3_vfs,
    _name: *const c_char,
    _flags: c_int,
    result: *mut c_int,
) -> c_int {
    // SAFETY: the SQLite callback contract supplies this output allocation when non-null.
    unsafe { boundary::write_int_zero(result) };
    boundary::catch_code(ACCESS_UNAVAILABLE, || ACCESS_UNAVAILABLE)
}

pub(super) unsafe extern "C" fn full_pathname(
    _vfs: *mut ffi::sqlite3_vfs,
    _name: *const c_char,
    output_size: c_int,
    output: *mut c_char,
) -> c_int {
    // A failed canonicalization never leaves a stale or unterminated path in caller storage.
    // SAFETY: the SQLite callback contract supplies this byte range when non-null and positive.
    unsafe {
        let _ = boundary::zero_bytes(output.cast::<c_void>(), output_size);
    }
    boundary::catch_code(FULL_PATHNAME_UNAVAILABLE, || FULL_PATHNAME_UNAVAILABLE)
}

pub(super) unsafe extern "C" fn dl_open(
    _vfs: *mut ffi::sqlite3_vfs,
    _name: *const c_char,
) -> *mut c_void {
    boundary::catch_value(std::ptr::null_mut(), || std::ptr::null_mut())
}

pub(super) unsafe extern "C" fn dl_error(
    _vfs: *mut ffi::sqlite3_vfs,
    output_size: c_int,
    output: *mut c_char,
) {
    // Never expose a stale loader message when dynamic loading is unavailable.
    // SAFETY: the SQLite callback contract supplies this byte range when non-null and positive.
    unsafe {
        let _ = boundary::zero_bytes(output.cast::<c_void>(), output_size);
    }
    boundary::catch_void(|| {});
}

pub(super) unsafe extern "C" fn dl_sym(
    _vfs: *mut ffi::sqlite3_vfs,
    _handle: *mut c_void,
    _symbol: *const c_char,
) -> Option<unsafe extern "C" fn(*mut ffi::sqlite3_vfs, *mut c_void, *const c_char)> {
    boundary::catch_value(None, || None)
}

pub(super) unsafe extern "C" fn dl_close(_vfs: *mut ffi::sqlite3_vfs, _handle: *mut c_void) {
    boundary::catch_void(|| {});
}

pub(super) unsafe extern "C" fn randomness(
    _vfs: *mut ffi::sqlite3_vfs,
    amount: c_int,
    output: *mut c_char,
) -> c_int {
    // Returning zero reports that no random bytes were produced; zeroing prevents stale data from
    // being mistaken for provider output if this unreachable callback is invoked directly.
    // SAFETY: the SQLite callback contract supplies this byte range when non-null and positive.
    unsafe {
        let _ = boundary::zero_bytes(output.cast::<c_void>(), amount);
    }
    boundary::catch_value(0, || 0)
}

pub(super) unsafe extern "C" fn sleep(_vfs: *mut ffi::sqlite3_vfs, _microseconds: c_int) -> c_int {
    boundary::catch_value(0, || 0)
}

pub(super) unsafe extern "C" fn current_time(
    _vfs: *mut ffi::sqlite3_vfs,
    output: *mut f64,
) -> c_int {
    // SAFETY: the SQLite callback contract supplies this output allocation when non-null.
    unsafe { boundary::write_f64_zero(output) };
    boundary::catch_code(CURRENT_TIME_UNAVAILABLE, || CURRENT_TIME_UNAVAILABLE)
}
