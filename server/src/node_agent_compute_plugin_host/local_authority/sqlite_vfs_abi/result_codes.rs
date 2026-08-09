use std::os::raw::c_int;

use rusqlite::ffi;

pub(super) const VFS_UNAVAILABLE: c_int = ffi::SQLITE_CANTOPEN;
pub(super) const DELETE_UNAVAILABLE: c_int = ffi::SQLITE_IOERR_DELETE;
pub(super) const ACCESS_UNAVAILABLE: c_int = ffi::SQLITE_IOERR_ACCESS;
pub(super) const FULL_PATHNAME_UNAVAILABLE: c_int = ffi::SQLITE_CANTOPEN_FULLPATH;
pub(super) const CURRENT_TIME_UNAVAILABLE: c_int = ffi::SQLITE_IOERR;

pub(super) const CLOSE_UNAVAILABLE: c_int = ffi::SQLITE_IOERR_CLOSE;
pub(super) const READ_UNAVAILABLE: c_int = ffi::SQLITE_IOERR_READ;
pub(super) const WRITE_UNAVAILABLE: c_int = ffi::SQLITE_IOERR_WRITE;
pub(super) const TRUNCATE_UNAVAILABLE: c_int = ffi::SQLITE_IOERR_TRUNCATE;
pub(super) const SYNC_UNAVAILABLE: c_int = ffi::SQLITE_IOERR_FSYNC;
pub(super) const FILE_SIZE_UNAVAILABLE: c_int = ffi::SQLITE_IOERR_FSTAT;
pub(super) const LOCK_UNAVAILABLE: c_int = ffi::SQLITE_IOERR_LOCK;
pub(super) const UNLOCK_UNAVAILABLE: c_int = ffi::SQLITE_IOERR_UNLOCK;
pub(super) const RESERVED_LOCK_UNAVAILABLE: c_int = ffi::SQLITE_IOERR_CHECKRESERVEDLOCK;
pub(super) const FILE_CONTROL_UNSUPPORTED: c_int = ffi::SQLITE_NOTFOUND;

pub(super) const SHM_MAP_UNAVAILABLE: c_int = ffi::SQLITE_IOERR_SHMMAP;
pub(super) const SHM_LOCK_UNAVAILABLE: c_int = ffi::SQLITE_IOERR_SHMLOCK;
// SQLite defines no dedicated SHM-unmap extended code. The generic I/O error avoids claiming a
// more specific failure whose phase this inert callback cannot observe.
pub(super) const SHM_UNMAP_UNAVAILABLE: c_int = ffi::SQLITE_IOERR;
