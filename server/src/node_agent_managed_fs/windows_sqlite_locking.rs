use std::{fs::File, os::windows::io::AsRawHandle};

use windows_sys::Win32::{
    Foundation::{ERROR_LOCK_VIOLATION, HANDLE},
    Storage::FileSystem::{
        LockFileEx, UnlockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
    },
    System::IO::{OVERLAPPED, OVERLAPPED_0, OVERLAPPED_0_0},
};

use crate::node_agent_managed_fs::PlatformManagedSqliteLockAttempt;

pub(super) fn try_lock_sqlite_byte_range(
    file: &File,
    offset: u64,
    length: u64,
    exclusive: bool,
) -> std::io::Result<PlatformManagedSqliteLockAttempt> {
    let mut overlapped = overlapped_at(offset);
    let mut flags = LOCKFILE_FAIL_IMMEDIATELY;
    if exclusive {
        flags |= LOCKFILE_EXCLUSIVE_LOCK;
    }
    let result = unsafe {
        // SAFETY: the borrowed File owns a live handle; OVERLAPPED is initialized with an explicit
        // byte offset and remains valid for this nonblocking synchronous lock attempt.
        LockFileEx(
            file.as_raw_handle() as HANDLE,
            flags,
            0,
            length as u32,
            (length >> 32) as u32,
            &mut overlapped,
        )
    };
    if result != 0 {
        return Ok(PlatformManagedSqliteLockAttempt::Acquired);
    }
    let error = std::io::Error::last_os_error();
    match error.raw_os_error().map(|code| code as u32) {
        Some(ERROR_LOCK_VIOLATION) => Ok(PlatformManagedSqliteLockAttempt::Contended),
        _ => Err(error),
    }
}

pub(super) fn unlock_sqlite_byte_range(
    file: &File,
    offset: u64,
    length: u64,
) -> std::io::Result<()> {
    let mut overlapped = overlapped_at(offset);
    let result = unsafe {
        // SAFETY: the borrowed File owns a live handle and the explicit range is one previously
        // selected by the sealed SQLite locking state machine.
        UnlockFileEx(
            file.as_raw_handle() as HANDLE,
            0,
            length as u32,
            (length >> 32) as u32,
            &mut overlapped,
        )
    };
    if result == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn overlapped_at(offset: u64) -> OVERLAPPED {
    OVERLAPPED {
        Internal: 0,
        InternalHigh: 0,
        Anonymous: OVERLAPPED_0 {
            Anonymous: OVERLAPPED_0_0 {
                Offset: offset as u32,
                OffsetHigh: (offset >> 32) as u32,
            },
        },
        hEvent: std::ptr::null_mut(),
    }
}
