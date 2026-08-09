use std::os::raw::c_void;

use rusqlite::ffi;

/// SQLite allocates `szOsFile` bytes and passes that storage to `xOpen`. Keeping `base` first is
/// the only representation fact this ABI layer relies on. Production `xOpen` initializes `state`
/// to null; tests may install private callback state, but no production constructor exists.
#[repr(C)]
pub(super) struct InertHandleBoundSqliteFile {
    pub(super) base: ffi::sqlite3_file,
    pub(super) state: *mut c_void,
}

/// `sqlite3_vfs` contains raw pointers and therefore is not `Sync` by default. This wrapper is
/// permitted only for the immutable, private, never-registered table assembled by the parent
/// module.
#[repr(transparent)]
pub(super) struct InertSqliteVfs(pub(super) ffi::sqlite3_vfs);

// SAFETY: every data pointer in the wrapped table is either null or points at immutable static
// bytes. The table and its inner value are private, no mutable pointer/reference is exposed, and
// this module contains no registration call that could let SQLite mutate `pNext`.
unsafe impl Sync for InertSqliteVfs {}
