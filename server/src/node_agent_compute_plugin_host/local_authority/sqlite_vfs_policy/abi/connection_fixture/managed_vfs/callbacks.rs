use std::{
    os::raw::{c_char, c_int, c_void},
    panic::{catch_unwind, AssertUnwindSafe},
    ptr,
};

use rusqlite::ffi;

use super::*;
use crate::{
    node_agent_compute_plugin_host::local_authority::{
        sqlite_vfs_abi::{initialize_test_vfs_file, install_test_vfs_file},
        sqlite_vfs_policy::{ManagedSqliteLogicalFileRole, ManagedSqliteVfsAccess},
    },
    node_agent_managed_fs::{ManagedSqliteAccess, ManagedSqliteFileKind, ManagedSqliteOpenMode},
};

pub(super) unsafe extern "C" fn open(
    vfs: *mut ffi::sqlite3_vfs,
    name: ffi::sqlite3_filename,
    file: *mut ffi::sqlite3_file,
    flags: c_int,
    output_flags: *mut c_int,
) -> c_int {
    if !output_flags.is_null() {
        // SAFETY: SQLite supplies this output allocation.
        unsafe { output_flags.write(0) };
    }
    // SAFETY: SQLite supplies fresh storage sized by this VFS.
    if !unsafe { initialize_test_vfs_file(file) } {
        return ffi::SQLITE_CANTOPEN;
    }
    catch_unwind(AssertUnwindSafe(|| {
        let context = unsafe { context(vfs) }.ok_or(())?;
        let callback = context.route.begin_open_callback()?;
        let request = context
            .route
            .project_x_open(unsafe { name_bytes(name) }, flags)?;
        let role = request.role();
        let access = match request.access() {
            ManagedSqliteVfsAccess::ReadOnly => ManagedSqliteAccess::ReadOnly,
            ManagedSqliteVfsAccess::ReadWrite => ManagedSqliteAccess::ReadWrite,
        };
        let mode = if request.create() {
            ManagedSqliteOpenMode::OpenOrCreate
        } else {
            ManagedSqliteOpenMode::Existing
        };
        let operations = match role {
            ManagedSqliteLogicalFileRole::Main => {
                let opened = context
                    .namespace
                    .open(ManagedSqliteFileKind::Main, access, mode)
                    .map_err(|failure| {
                        let _ = context.route.retain_failure(failure);
                    })?;
                let main = opened.into_main_file().map_err(|failure| {
                    let _ = context.route.retain_failure(failure);
                })?;
                context.route.bind_main(main)?
            }
            ManagedSqliteLogicalFileRole::Journal => {
                let opened = context
                    .namespace
                    .open(ManagedSqliteFileKind::Journal, access, mode)
                    .map_err(|failure| {
                        let _ = context.route.retain_failure(failure);
                    })?;
                context.route.bind_sidecar(opened, role)?
            }
            ManagedSqliteLogicalFileRole::Wal => {
                context.wal_open_attempts.fetch_add(1, Ordering::SeqCst);
                let _ = context
                    .route
                    .retain_failure("managed test VFS WAL promotion is unavailable");
                return Err(());
            }
        };
        // SAFETY: this is the initialized allocation owned by the current xOpen callback.
        unsafe { install_test_vfs_file(file, operations) }?;
        callback.complete()?;
        if role == ManagedSqliteLogicalFileRole::Main {
            context.route.activate_after_main_open()?;
            context.main_opens.fetch_add(1, Ordering::SeqCst);
        } else {
            context.journal_opens.fetch_add(1, Ordering::SeqCst);
        }
        if !output_flags.is_null() {
            let actual = match request.access() {
                ManagedSqliteVfsAccess::ReadOnly => ffi::SQLITE_OPEN_READONLY,
                ManagedSqliteVfsAccess::ReadWrite => ffi::SQLITE_OPEN_READWRITE,
            };
            // SAFETY: SQLite supplies this output allocation.
            unsafe { output_flags.write(actual) };
        }
        Ok::<(), ()>(())
    }))
    .map_or(ffi::SQLITE_CANTOPEN, |result| {
        result.map_or(ffi::SQLITE_CANTOPEN, |()| ffi::SQLITE_OK)
    })
}

pub(super) unsafe extern "C" fn access(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    flag: c_int,
    output: *mut c_int,
) -> c_int {
    if !output.is_null() {
        // SAFETY: SQLite supplies this output allocation.
        unsafe { output.write(0) };
    }
    catch_unwind(AssertUnwindSafe(|| {
        let context = unsafe { context(vfs) }.ok_or(())?;
        let callback = context.route.begin_access_callback()?;
        let request = context
            .route
            .project_x_access(unsafe { name_bytes(name) }, flag)?;
        let kind = sidecar_kind(request.role())?;
        let exists = context
            .namespace
            .access(kind, ManagedSqliteAccess::ReadWrite)
            .map_err(|failure| {
                let _ = context.route.retain_failure(failure);
            })?;
        if output.is_null() {
            return Err(());
        }
        // SAFETY: null was rejected above and SQLite owns this output allocation.
        unsafe { output.write(c_int::from(exists)) };
        callback.complete()?;
        Ok::<(), ()>(())
    }))
    .map_or(ffi::SQLITE_IOERR_ACCESS, |result| {
        result.map_or(ffi::SQLITE_IOERR_ACCESS, |()| ffi::SQLITE_OK)
    })
}

pub(super) unsafe extern "C" fn delete(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    sync_directory: c_int,
) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let context = unsafe { context(vfs) }.ok_or(())?;
        let callback = context.route.begin_delete_callback()?;
        let request = context
            .route
            .project_x_delete(unsafe { name_bytes(name) }, sync_directory)?;
        let kind = sidecar_kind(request.role())?;
        let _outcome = context
            .namespace
            .delete(kind, request.sync_parent())
            .map_err(|failure| {
                let _ = context.route.retain_failure(failure);
            })?;
        callback.complete()?;
        Ok::<(), ()>(())
    }))
    .map_or(ffi::SQLITE_IOERR_DELETE, |result| {
        result.map_or(ffi::SQLITE_IOERR_DELETE, |()| ffi::SQLITE_OK)
    })
}

pub(super) unsafe extern "C" fn full_pathname(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    output_size: c_int,
    output: *mut c_char,
) -> c_int {
    if output_size > 0 && !output.is_null() {
        // SAFETY: SQLite supplies this writable output range.
        unsafe { ptr::write_bytes(output.cast::<u8>(), 0, output_size as usize) };
    }
    catch_unwind(AssertUnwindSafe(|| {
        let context = unsafe { context(vfs) }.ok_or(())?;
        let callback = context.route.begin_full_pathname_callback()?;
        let projected = context
            .route
            .project_x_full_pathname(unsafe { name_bytes(name) }, output_size)?;
        if output.is_null() || output_size <= 0 {
            return Err(());
        }
        let bytes = projected.as_bytes_with_nul();
        if bytes.len() > output_size as usize {
            return Err(());
        }
        // SAFETY: capacity was checked and regions do not overlap.
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), output.cast::<u8>(), bytes.len()) };
        callback.complete()?;
        Ok::<(), ()>(())
    }))
    .map_or(ffi::SQLITE_CANTOPEN, |result| {
        result.map_or(ffi::SQLITE_CANTOPEN, |()| ffi::SQLITE_OK)
    })
}

pub(super) unsafe extern "C" fn dl_open(
    _vfs: *mut ffi::sqlite3_vfs,
    _name: *const c_char,
) -> *mut c_void {
    ptr::null_mut()
}

pub(super) unsafe extern "C" fn dl_error(
    _vfs: *mut ffi::sqlite3_vfs,
    output_size: c_int,
    output: *mut c_char,
) {
    if output_size > 0 && !output.is_null() {
        // SAFETY: SQLite supplies this writable output range.
        unsafe { ptr::write_bytes(output.cast::<u8>(), 0, output_size as usize) };
    }
}

pub(super) unsafe extern "C" fn dl_sym(
    _vfs: *mut ffi::sqlite3_vfs,
    _handle: *mut c_void,
    _symbol: *const c_char,
) -> Option<unsafe extern "C" fn(*mut ffi::sqlite3_vfs, *mut c_void, *const c_char)> {
    None
}

pub(super) unsafe extern "C" fn dl_close(_vfs: *mut ffi::sqlite3_vfs, _handle: *mut c_void) {}

pub(super) unsafe extern "C" fn randomness(
    vfs: *mut ffi::sqlite3_vfs,
    amount: c_int,
    output: *mut c_char,
) -> c_int {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        let Some(context) = context(vfs) else {
            return 0;
        };
        context
            .backing
            .as_ref()
            .and_then(|table| table.xRandomness)
            .map_or(0, |callback| callback(context.backing, amount, output))
    }))
    .unwrap_or(0)
}

pub(super) unsafe extern "C" fn sleep(vfs: *mut ffi::sqlite3_vfs, microseconds: c_int) -> c_int {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        let Some(context) = context(vfs) else {
            return 0;
        };
        context
            .backing
            .as_ref()
            .and_then(|table| table.xSleep)
            .map_or(0, |callback| callback(context.backing, microseconds))
    }))
    .unwrap_or(0)
}

pub(super) unsafe extern "C" fn current_time(
    vfs: *mut ffi::sqlite3_vfs,
    output: *mut f64,
) -> c_int {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        let Some(context) = context(vfs) else {
            return ffi::SQLITE_IOERR;
        };
        context
            .backing
            .as_ref()
            .and_then(|table| table.xCurrentTime)
            .map_or(ffi::SQLITE_IOERR, |callback| {
                callback(context.backing, output)
            })
    }))
    .unwrap_or(ffi::SQLITE_IOERR)
}

pub(super) unsafe extern "C" fn get_last_error(
    vfs: *mut ffi::sqlite3_vfs,
    output_size: c_int,
    output: *mut c_char,
) -> c_int {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        let Some(context) = context(vfs) else {
            return 0;
        };
        context
            .backing
            .as_ref()
            .and_then(|table| table.xGetLastError)
            .map_or(0, |callback| callback(context.backing, output_size, output))
    }))
    .unwrap_or(0)
}

fn sidecar_kind(role: ManagedSqliteLogicalFileRole) -> Result<ManagedSqliteFileKind, ()> {
    match role {
        ManagedSqliteLogicalFileRole::Journal => Ok(ManagedSqliteFileKind::Journal),
        ManagedSqliteLogicalFileRole::Wal => Ok(ManagedSqliteFileKind::Wal),
        ManagedSqliteLogicalFileRole::Main => Err(()),
    }
}

unsafe fn context<'a>(vfs: *mut ffi::sqlite3_vfs) -> Option<&'a ManagedTestVfsContext> {
    let context = unsafe { vfs.as_ref() }?
        .pAppData
        .cast::<ManagedTestVfsContext>();
    // SAFETY: registration owns this boxed context until successful unregister.
    unsafe { context.as_ref() }
}

unsafe fn name_bytes<'a>(name: *const c_char) -> Option<&'a [u8]> {
    if name.is_null() {
        None
    } else {
        // SAFETY: SQLite owns this NUL-terminated name for the callback duration.
        Some(unsafe { CStr::from_ptr(name) }.to_bytes())
    }
}
