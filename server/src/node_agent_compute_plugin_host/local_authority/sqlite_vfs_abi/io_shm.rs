use std::os::raw::{c_int, c_void};

use rusqlite::ffi;

use super::{boundary, result_codes};

pub(super) unsafe extern "C" fn map(
    _file: *mut ffi::sqlite3_file,
    _region: c_int,
    _region_size: c_int,
    _extend: c_int,
    output: *mut *mut c_void,
) -> c_int {
    // SAFETY: the SQLite callback contract supplies this output allocation when non-null.
    unsafe { boundary::write_pointer_null(output) };
    boundary::catch_code(result_codes::SHM_MAP_UNAVAILABLE, || {
        result_codes::SHM_MAP_UNAVAILABLE
    })
}

pub(super) unsafe extern "C" fn lock(
    _file: *mut ffi::sqlite3_file,
    _offset: c_int,
    _count: c_int,
    _flags: c_int,
) -> c_int {
    boundary::catch_code(result_codes::SHM_LOCK_UNAVAILABLE, || {
        result_codes::SHM_LOCK_UNAVAILABLE
    })
}

pub(super) unsafe extern "C" fn barrier(_file: *mut ffi::sqlite3_file) {
    // A void callback has no result-code channel. Catching here is the complete behavior because
    // this inert table has no live state that could be mutated or poisoned.
    boundary::catch_void(|| {});
}

pub(super) unsafe extern "C" fn unmap(_file: *mut ffi::sqlite3_file, _delete: c_int) -> c_int {
    boundary::catch_code(result_codes::SHM_UNMAP_UNAVAILABLE, || {
        result_codes::SHM_UNMAP_UNAVAILABLE
    })
}
