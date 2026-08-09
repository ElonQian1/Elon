//! Test-only named VFS transport used to exercise a real SQLite connection lifecycle.
//!
//! This is an alias over SQLite's default VFS, not the managed authority VFS. It exists only in
//! unit-test builds, is never selected as the default, and is leaked after registration because
//! SQLite retains the table pointer for process lifetime.

use std::{
    os::raw::{c_char, c_int, c_void},
    ptr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        OnceLock,
    },
};

use rusqlite::ffi;

pub(in crate::node_agent_compute_plugin_host::local_authority) const TEST_TRANSPORT_VFS_NAME: &str =
    "elon-test-transport-v1";
const TEST_TRANSPORT_VFS_NAME_C: &[u8] = b"elon-test-transport-v1\0";

static REGISTERED_VFS: OnceLock<Result<usize, c_int>> = OnceLock::new();
static OPEN_COUNT: AtomicUsize = AtomicUsize::new(0);

pub(in crate::node_agent_compute_plugin_host::local_authority) fn ensure_test_transport_vfs(
) -> Result<(), c_int> {
    match *REGISTERED_VFS.get_or_init(register) {
        Ok(_) => Ok(()),
        Err(code) => Err(code),
    }
}

pub(in crate::node_agent_compute_plugin_host::local_authority) fn test_transport_open_count(
) -> usize {
    OPEN_COUNT.load(Ordering::SeqCst)
}

fn register() -> Result<usize, c_int> {
    // SAFETY: SQLite owns the default VFS for process lifetime. The wrapper is leaked after a
    // successful registration because SQLite stores its pointer globally.
    unsafe {
        let backing = ffi::sqlite3_vfs_find(ptr::null());
        if backing.is_null() {
            return Err(ffi::SQLITE_CANTOPEN);
        }
        let table = Box::new(ffi::sqlite3_vfs {
            iVersion: 1,
            szOsFile: (*backing).szOsFile,
            mxPathname: (*backing).mxPathname,
            pNext: ptr::null_mut(),
            zName: TEST_TRANSPORT_VFS_NAME_C.as_ptr().cast(),
            pAppData: backing.cast(),
            xOpen: Some(open),
            xDelete: Some(delete),
            xAccess: Some(access),
            xFullPathname: Some(full_pathname),
            xDlOpen: Some(dl_open),
            xDlError: Some(dl_error),
            xDlSym: Some(dl_sym),
            xDlClose: Some(dl_close),
            xRandomness: Some(randomness),
            xSleep: Some(sleep),
            xCurrentTime: Some(current_time),
            xGetLastError: Some(get_last_error),
            xCurrentTimeInt64: None,
            xSetSystemCall: None,
            xGetSystemCall: None,
            xNextSystemCall: None,
        });
        let table = Box::into_raw(table);
        let code = ffi::sqlite3_vfs_register(table, 0);
        if code != ffi::SQLITE_OK {
            drop(Box::from_raw(table));
            return Err(code);
        }
        Ok(table as usize)
    }
}

unsafe fn backing(vfs: *mut ffi::sqlite3_vfs) -> *mut ffi::sqlite3_vfs {
    if vfs.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: every registered test table stores the process-owned default VFS in pAppData.
    unsafe { (*vfs).pAppData.cast() }
}

pub(super) unsafe extern "C" fn open(
    vfs: *mut ffi::sqlite3_vfs,
    name: ffi::sqlite3_filename,
    file: *mut ffi::sqlite3_file,
    flags: c_int,
    output_flags: *mut c_int,
) -> c_int {
    // SAFETY: all arguments are forwarded unchanged to the process-owned backing VFS.
    unsafe {
        let backing = backing(vfs);
        let Some(callback) = backing.as_ref().and_then(|table| table.xOpen) else {
            return ffi::SQLITE_CANTOPEN;
        };
        let code = callback(backing, name, file, flags, output_flags);
        if code == ffi::SQLITE_OK {
            OPEN_COUNT.fetch_add(1, Ordering::SeqCst);
        }
        code
    }
}

pub(super) unsafe extern "C" fn delete(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    sync_directory: c_int,
) -> c_int {
    // SAFETY: callback arguments are forwarded unchanged to the backing VFS.
    unsafe {
        let backing = backing(vfs);
        match backing.as_ref().and_then(|table| table.xDelete) {
            Some(callback) => callback(backing, name, sync_directory),
            None => ffi::SQLITE_IOERR_DELETE,
        }
    }
}

pub(super) unsafe extern "C" fn access(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    flags: c_int,
    output: *mut c_int,
) -> c_int {
    // SAFETY: callback arguments are forwarded unchanged to the backing VFS.
    unsafe {
        let backing = backing(vfs);
        match backing.as_ref().and_then(|table| table.xAccess) {
            Some(callback) => callback(backing, name, flags, output),
            None => ffi::SQLITE_IOERR_ACCESS,
        }
    }
}

pub(super) unsafe extern "C" fn full_pathname(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    output_size: c_int,
    output: *mut c_char,
) -> c_int {
    // SAFETY: callback arguments are forwarded unchanged to the backing VFS.
    unsafe {
        let backing = backing(vfs);
        match backing.as_ref().and_then(|table| table.xFullPathname) {
            Some(callback) => callback(backing, name, output_size, output),
            None => ffi::SQLITE_CANTOPEN,
        }
    }
}

pub(super) unsafe extern "C" fn dl_open(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
) -> *mut c_void {
    // SAFETY: callback arguments are forwarded unchanged to the backing VFS.
    unsafe {
        let backing = backing(vfs);
        backing
            .as_ref()
            .and_then(|table| table.xDlOpen)
            .map_or(ptr::null_mut(), |callback| callback(backing, name))
    }
}

pub(super) unsafe extern "C" fn dl_error(
    vfs: *mut ffi::sqlite3_vfs,
    output_size: c_int,
    output: *mut c_char,
) {
    // SAFETY: callback arguments are forwarded unchanged to the backing VFS.
    unsafe {
        let backing = backing(vfs);
        if let Some(callback) = backing.as_ref().and_then(|table| table.xDlError) {
            callback(backing, output_size, output);
        }
    }
}

pub(super) unsafe extern "C" fn dl_sym(
    vfs: *mut ffi::sqlite3_vfs,
    handle: *mut c_void,
    symbol: *const c_char,
) -> Option<unsafe extern "C" fn(*mut ffi::sqlite3_vfs, *mut c_void, *const c_char)> {
    // SAFETY: callback arguments are forwarded unchanged to the backing VFS.
    unsafe {
        let backing = backing(vfs);
        backing
            .as_ref()
            .and_then(|table| table.xDlSym)
            .and_then(|callback| callback(backing, handle, symbol))
    }
}

pub(super) unsafe extern "C" fn dl_close(vfs: *mut ffi::sqlite3_vfs, handle: *mut c_void) {
    // SAFETY: callback arguments are forwarded unchanged to the backing VFS.
    unsafe {
        let backing = backing(vfs);
        if let Some(callback) = backing.as_ref().and_then(|table| table.xDlClose) {
            callback(backing, handle);
        }
    }
}

pub(super) unsafe extern "C" fn randomness(
    vfs: *mut ffi::sqlite3_vfs,
    amount: c_int,
    output: *mut c_char,
) -> c_int {
    // SAFETY: callback arguments are forwarded unchanged to the backing VFS.
    unsafe {
        let backing = backing(vfs);
        backing
            .as_ref()
            .and_then(|table| table.xRandomness)
            .map_or(0, |callback| callback(backing, amount, output))
    }
}

pub(super) unsafe extern "C" fn sleep(vfs: *mut ffi::sqlite3_vfs, microseconds: c_int) -> c_int {
    // SAFETY: callback arguments are forwarded unchanged to the backing VFS.
    unsafe {
        let backing = backing(vfs);
        backing
            .as_ref()
            .and_then(|table| table.xSleep)
            .map_or(0, |callback| callback(backing, microseconds))
    }
}

pub(super) unsafe extern "C" fn current_time(
    vfs: *mut ffi::sqlite3_vfs,
    output: *mut f64,
) -> c_int {
    // SAFETY: callback arguments are forwarded unchanged to the backing VFS.
    unsafe {
        let backing = backing(vfs);
        match backing.as_ref().and_then(|table| table.xCurrentTime) {
            Some(callback) => callback(backing, output),
            None => ffi::SQLITE_IOERR,
        }
    }
}

pub(super) unsafe extern "C" fn get_last_error(
    vfs: *mut ffi::sqlite3_vfs,
    output_size: c_int,
    output: *mut c_char,
) -> c_int {
    // SAFETY: callback arguments are forwarded unchanged to the backing VFS.
    unsafe {
        let backing = backing(vfs);
        backing
            .as_ref()
            .and_then(|table| table.xGetLastError)
            .map_or(0, |callback| callback(backing, output_size, output))
    }
}
