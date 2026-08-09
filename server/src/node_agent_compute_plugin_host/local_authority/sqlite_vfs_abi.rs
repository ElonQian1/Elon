//! Inert C ABI front-half for the future handle-bound SQLite VFS.
//!
//! Both tables in this module are deliberately private and unreachable. The VFS table is never
//! registered, its application-data pointer is null, and `xOpen` can only report unavailable.
//! The I/O table is not installed into any `sqlite3_file`. This module therefore proves only the
//! fail-closed ABI shape; it grants no registry, filesystem, SQLite-open, or connection authority.

use std::{mem::size_of, os::raw::c_int, ptr};

use rusqlite::ffi;

mod boundary;
mod io_core;
mod io_shm;
mod result_codes;
mod types;
mod vfs_callbacks;

use types::{InertHandleBoundSqliteFile, InertSqliteVfs};

const INERT_VFS_NAME: &[u8] = b"elon-handle-bound-unavailable-v1\0";
const MAX_LOGICAL_NAME_BYTES: c_int = 64;

/// Version 2 advertises the WAL callback slots but deliberately omits mmap fetch/unfetch.
/// No callback in this table can observe live managed-file state in this batch.
static INERT_IO_METHODS: ffi::sqlite3_io_methods = ffi::sqlite3_io_methods {
    iVersion: 2,
    xClose: Some(io_core::close),
    xRead: Some(io_core::read),
    xWrite: Some(io_core::write),
    xTruncate: Some(io_core::truncate),
    xSync: Some(io_core::sync),
    xFileSize: Some(io_core::file_size),
    xLock: Some(io_core::lock),
    xUnlock: Some(io_core::unlock),
    xCheckReservedLock: Some(io_core::check_reserved_lock),
    xFileControl: Some(io_core::file_control),
    xSectorSize: Some(io_core::sector_size),
    xDeviceCharacteristics: Some(io_core::device_characteristics),
    xShmMap: Some(io_shm::map),
    xShmLock: Some(io_shm::lock),
    xShmBarrier: Some(io_shm::barrier),
    xShmUnmap: Some(io_shm::unmap),
    xFetch: None,
    xUnfetch: None,
};

/// Version 1 avoids claiming system-call or high-resolution-time providers. Core callbacks either
/// return an unavailable result or a zeroed conservative value. The wrapper is immutable and no
/// mutable pointer to it is exposed, so it cannot be passed to SQLite registration from Rust.
static INERT_VFS: InertSqliteVfs = InertSqliteVfs(ffi::sqlite3_vfs {
    iVersion: 1,
    szOsFile: size_of::<InertHandleBoundSqliteFile>() as c_int,
    mxPathname: MAX_LOGICAL_NAME_BYTES,
    pNext: ptr::null_mut(),
    zName: INERT_VFS_NAME.as_ptr().cast(),
    pAppData: ptr::null_mut(),
    xOpen: Some(vfs_callbacks::open),
    xDelete: Some(vfs_callbacks::delete),
    xAccess: Some(vfs_callbacks::access),
    xFullPathname: Some(vfs_callbacks::full_pathname),
    xDlOpen: Some(vfs_callbacks::dl_open),
    xDlError: Some(vfs_callbacks::dl_error),
    xDlSym: Some(vfs_callbacks::dl_sym),
    xDlClose: Some(vfs_callbacks::dl_close),
    xRandomness: Some(vfs_callbacks::randomness),
    xSleep: Some(vfs_callbacks::sleep),
    xCurrentTime: Some(vfs_callbacks::current_time),
    xGetLastError: None,
    xCurrentTimeInt64: None,
    xSetSystemCall: None,
    xGetSystemCall: None,
    xNextSystemCall: None,
});
