use std::{fs::File, os::windows::io::AsRawHandle};

use windows_sys::Win32::{
    Foundation::{ERROR_LOCK_VIOLATION, HANDLE},
    Storage::FileSystem::{
        LockFileEx, UnlockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
    },
    System::IO::{OVERLAPPED, OVERLAPPED_0, OVERLAPPED_0_0},
};

#[cfg(all(test, windows))]
use crate::node_agent_managed_fs::ManagedSqliteShmTestUnmapNativeObservation;
use crate::node_agent_managed_fs::PlatformManagedSqliteLockAttempt;

#[cfg(all(test, windows))]
use super::test_native_return_receipt_unavailable_error;

#[cfg(all(test, windows))]
pub(in crate::node_agent_managed_fs) struct PlatformManagedSqliteUnlockReturnReceiptUnavailable {
    pub(in crate::node_agent_managed_fs) error: std::io::Error,
    pub(in crate::node_agent_managed_fs) offset: u64,
    pub(in crate::node_agent_managed_fs) length: u64,
    pub(in crate::node_agent_managed_fs) exact_call_occurrence: std::num::NonZeroU32,
}

#[cfg(all(test, windows))]
pub(in crate::node_agent_managed_fs) struct PlatformManagedSqliteInitializationUnlockReturnReceiptUnavailableV1 {
    pub(in crate::node_agent_managed_fs) error: std::io::Error,
    pub(in crate::node_agent_managed_fs) offset: u64,
    pub(in crate::node_agent_managed_fs) length: u64,
    pub(in crate::node_agent_managed_fs) exact_call_occurrence: std::num::NonZeroU32,
}

pub(in crate::node_agent_managed_fs) fn try_lock_sqlite_byte_range(
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

pub(in crate::node_agent_managed_fs) fn unlock_sqlite_byte_range(
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

/// Executes the real UnlockFileEx call once without inspecting its return receipt.
#[cfg(all(test, windows))]
pub(in crate::node_agent_managed_fs) fn unlock_sqlite_byte_range_outcome_uncertain_for_test(
    file: &File,
    offset: u64,
    length: u64,
) -> (
    std::io::Result<()>,
    Option<ManagedSqliteShmTestUnmapNativeObservation>,
) {
    unlock_sqlite_byte_range_without_return_receipt(file, offset, length);
    (
        Err(test_native_return_receipt_unavailable_error(
            "UnlockFileEx(SQLite SHM DMS)",
        )),
        Some(ManagedSqliteShmTestUnmapNativeObservation::ReturnReceiptUnavailable),
    )
}

/// Executes the production CreatedFirst DMS UnlockFileEx call exactly once while deliberately
/// leaving its BOOL return receipt unread. Only the initialization controller can consume this
/// typed witness.
#[cfg(all(test, windows))]
pub(in crate::node_agent_managed_fs) fn unlock_sqlite_byte_range_outcome_uncertain_for_initialization_test(
    file: &File,
    offset: u64,
    length: u64,
) -> PlatformManagedSqliteInitializationUnlockReturnReceiptUnavailableV1 {
    unlock_sqlite_byte_range_without_return_receipt(file, offset, length);
    PlatformManagedSqliteInitializationUnlockReturnReceiptUnavailableV1 {
        error: test_native_return_receipt_unavailable_error(
            "UnlockFileEx(SQLite SHM initialization DMS)",
        ),
        offset,
        length,
        exact_call_occurrence: std::num::NonZeroU32::new(1)
            .expect("one exact initialization UnlockFileEx call is non-zero"),
    }
}

/// Executes the production-parameter UnlockFileEx call once and returns only a typed witness that
/// its BOOL receipt was deliberately not read.
#[cfg(all(test, windows))]
pub(in crate::node_agent_managed_fs) fn unlock_sqlite_byte_range_return_receipt_unavailable_for_main_close_test(
    file: &File,
    offset: u64,
    length: u64,
) -> PlatformManagedSqliteUnlockReturnReceiptUnavailable {
    unlock_sqlite_byte_range_without_return_receipt(file, offset, length);
    PlatformManagedSqliteUnlockReturnReceiptUnavailable {
        error: test_native_return_receipt_unavailable_error("UnlockFileEx(SQLite main lock)"),
        offset,
        length,
        exact_call_occurrence: std::num::NonZeroU32::new(1)
            .expect("one exact UnlockFileEx call is non-zero"),
    }
}

#[cfg(all(test, windows))]
fn unlock_sqlite_byte_range_without_return_receipt(file: &File, offset: u64, length: u64) {
    let mut overlapped = overlapped_at(offset);
    // SAFETY: callers pass a range held by their live managed SQLite state. This is the same legal
    // UnlockFileEx(handle, 0, low_len, high_len, overlapped) signature as production; only its BOOL
    // return receipt is deliberately left unread.
    unsafe {
        UnlockFileEx(
            file.as_raw_handle() as HANDLE,
            0,
            length as u32,
            (length >> 32) as u32,
            &mut overlapped,
        );
    }
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

#[cfg(all(test, windows))]
mod tests {
    use std::{
        fs::OpenOptions,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;

    static NEXT_TEST_FILE: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn main_close_unavailable_receipt_uses_real_shared_and_reserved_ranges_once() {
        const PENDING_BYTE: u64 = 0x4000_0000;
        const RESERVED_BYTE: u64 = PENDING_BYTE + 1;
        const SHARED_FIRST: u64 = PENDING_BYTE + 2;
        const SHARED_SIZE: u64 = 510;

        for (label, offset, length, exclusive) in [
            ("shared", SHARED_FIRST, SHARED_SIZE, false),
            ("reserved", RESERVED_BYTE, 1, true),
        ] {
            let serial = NEXT_TEST_FILE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "elon-managed-sqlite-unlock-{label}-{}-{serial}.db",
                std::process::id()
            ));
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&path)
                .expect("create isolated UnlockFileEx test file");
            assert_eq!(
                try_lock_sqlite_byte_range(&file, offset, length, exclusive)
                    .expect("acquire selected range"),
                PlatformManagedSqliteLockAttempt::Acquired
            );

            let unavailable =
                unlock_sqlite_byte_range_return_receipt_unavailable_for_main_close_test(
                    &file, offset, length,
                );
            assert_eq!(unavailable.offset, offset);
            assert_eq!(unavailable.length, length);
            assert_eq!(unavailable.exact_call_occurrence.get(), 1);
            assert!(unavailable.error.get_ref().is_some());

            assert_eq!(
                try_lock_sqlite_byte_range(&file, offset, length, true)
                    .expect("reacquire range after exact UnlockFileEx"),
                PlatformManagedSqliteLockAttempt::Acquired
            );
            unlock_sqlite_byte_range(&file, offset, length).expect("release reacquired test range");
            drop(file);
            std::fs::remove_file(path).expect("remove UnlockFileEx test file");
        }
    }
}
