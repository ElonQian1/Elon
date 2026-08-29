use std::{
    ffi::OsStr,
    fs::File,
    mem::{size_of, MaybeUninit},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, IntoRawHandle, RawHandle},
    },
};

#[cfg(all(test, windows))]
use windows_sys::Win32::Foundation::{SetHandleInformation, HANDLE_FLAG_PROTECT_FROM_CLOSE};
use windows_sys::{
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        Storage::FileSystem::{
            NtCreateFile, NtReadFile, NtWriteFile, FILE_NON_DIRECTORY_FILE, FILE_OPEN,
            FILE_OPEN_IF, FILE_OPEN_REPARSE_POINT as NT_FILE_OPEN_REPARSE_POINT,
            FILE_SYNCHRONOUS_IO_NONALERT,
        },
    },
    Win32::{
        Foundation::{
            CloseHandle, HANDLE, INVALID_HANDLE_VALUE, STATUS_END_OF_FILE, STATUS_SUCCESS,
            UNICODE_STRING,
        },
        Storage::FileSystem::{
            FlushFileBuffers, DELETE, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
            FILE_READ_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, SYNCHRONIZE,
        },
        System::IO::IO_STATUS_BLOCK,
    },
};

use super::ntstatus_error;
#[cfg(all(test, windows))]
use super::test_native_return_receipt_unavailable_error;
#[cfg(all(test, windows))]
use crate::node_agent_managed_fs::ManagedSqliteShmTestUnmapNativeObservation;
use crate::node_agent_managed_fs::{
    ManagedSqliteAccess, ManagedSqliteFileKind, ManagedSqliteOpenMode,
};

// Match SQLite's normal Windows sharing contract: concurrent reads/writes are allowed, while an
// active pinned file prevents rename/delete from silently detaching its fixed namespace name.
const SQLITE_SHARE_ACCESS: u32 = FILE_SHARE_READ | FILE_SHARE_WRITE;

pub(in crate::node_agent_managed_fs) struct PlatformManagedSqliteOpen {
    pub(in crate::node_agent_managed_fs) file: File,
    pub(in crate::node_agent_managed_fs) call_status: i32,
    pub(in crate::node_agent_managed_fs) completion_status: i32,
    pub(in crate::node_agent_managed_fs) information: usize,
}

pub(in crate::node_agent_managed_fs) struct PlatformManagedSqliteCloseFailure {
    pub(in crate::node_agent_managed_fs) error: std::io::Error,
    pub(in crate::node_agent_managed_fs) custody: PlatformManagedSqliteCloseCustody,
}

pub(in crate::node_agent_managed_fs) enum PlatformManagedSqliteCloseCustody {
    Unattempted(File),
    OutcomeUncertainRawHandle(usize),
}

#[cfg(all(test, windows))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_managed_fs) enum PlatformManagedSqliteCloseTestNative {
    Retryable,
    OutcomeUncertain,
}

#[cfg(all(test, windows))]
pub(in crate::node_agent_managed_fs) struct PlatformManagedSqliteCloseTestNativeResult {
    pub(in crate::node_agent_managed_fs) result: Result<(), PlatformManagedSqliteCloseFailure>,
    pub(in crate::node_agent_managed_fs) observation:
        Option<ManagedSqliteShmTestUnmapNativeObservation>,
}

pub(in crate::node_agent_managed_fs) fn close_sqlite_file(
    file: File,
) -> Result<(), PlatformManagedSqliteCloseFailure> {
    let raw_handle = file.into_raw_handle();
    // SAFETY: `into_raw_handle` transfers the sole File owner into this exact close attempt.
    if unsafe { CloseHandle(raw_handle as HANDLE) } == 0 {
        return Err(PlatformManagedSqliteCloseFailure {
            error: std::io::Error::last_os_error(),
            custody: PlatformManagedSqliteCloseCustody::OutcomeUncertainRawHandle(
                raw_handle as usize,
            ),
        });
    }
    Ok(())
}

/// Narrow one-shot adapter at the same CloseHandle ownership boundary as production close.
#[cfg(all(test, windows))]
pub(in crate::node_agent_managed_fs) fn close_sqlite_file_for_test_native(
    file: File,
    native: PlatformManagedSqliteCloseTestNative,
) -> PlatformManagedSqliteCloseTestNativeResult {
    if native == PlatformManagedSqliteCloseTestNative::Retryable {
        let raw_handle = file.as_raw_handle() as HANDLE;
        // Protecting the exact live handle makes the immediately following real CloseHandle call
        // fail before consuming it. The protection is removed before live custody returns.
        if unsafe {
            SetHandleInformation(
                raw_handle,
                HANDLE_FLAG_PROTECT_FROM_CLOSE,
                HANDLE_FLAG_PROTECT_FROM_CLOSE,
            )
        } == 0
        {
            return PlatformManagedSqliteCloseTestNativeResult {
                result: Err(PlatformManagedSqliteCloseFailure {
                    error: std::io::Error::last_os_error(),
                    custody: PlatformManagedSqliteCloseCustody::Unattempted(file),
                }),
                observation: None,
            };
        }
        // SAFETY: this is the exact handle owned by `file`; PROTECT_FROM_CLOSE is expected to
        // reject this close while preserving the handle for retryable custody.
        let closed = unsafe { CloseHandle(raw_handle) };
        let close_error = (closed == 0).then(std::io::Error::last_os_error);
        if closed != 0 {
            std::mem::forget(file);
            return PlatformManagedSqliteCloseTestNativeResult {
                result: Ok(()),
                observation: None,
            };
        }
        // SAFETY: the failed CloseHandle left this same protected handle live. Clearing only the
        // protection bit restores ordinary sole-File custody.
        if unsafe { SetHandleInformation(raw_handle, HANDLE_FLAG_PROTECT_FROM_CLOSE, 0) } == 0 {
            let raw_handle = file.into_raw_handle();
            return PlatformManagedSqliteCloseTestNativeResult {
                result: Err(PlatformManagedSqliteCloseFailure {
                    error: std::io::Error::last_os_error(),
                    custody: PlatformManagedSqliteCloseCustody::OutcomeUncertainRawHandle(
                        raw_handle as usize,
                    ),
                }),
                observation: None,
            };
        }
        return PlatformManagedSqliteCloseTestNativeResult {
            result: Err(PlatformManagedSqliteCloseFailure {
                error: close_error.expect("failed CloseHandle has an OS error"),
                custody: PlatformManagedSqliteCloseCustody::Unattempted(file),
            }),
            observation: Some(ManagedSqliteShmTestUnmapNativeObservation::NativeFailureObserved),
        };
    }
    let raw_handle = file.into_raw_handle();
    // SAFETY: into_raw_handle transfers the only owner into this exact CloseHandle call. Its return
    // value is intentionally discarded, so the adapter cannot first learn success/failure and then
    // relabel that known result as uncertain.
    unsafe {
        CloseHandle(raw_handle as HANDLE);
    }
    PlatformManagedSqliteCloseTestNativeResult {
        result: Err(PlatformManagedSqliteCloseFailure {
            error: test_native_return_receipt_unavailable_error("CloseHandle(SQLite SHM file)"),
            custody: PlatformManagedSqliteCloseCustody::OutcomeUncertainRawHandle(
                raw_handle as usize,
            ),
        }),
        observation: Some(ManagedSqliteShmTestUnmapNativeObservation::ReturnReceiptUnavailable),
    }
}

pub(in crate::node_agent_managed_fs) fn open_sqlite_file_relative(
    parent: &File,
    kind: ManagedSqliteFileKind,
    access: ManagedSqliteAccess,
    mode: ManagedSqliteOpenMode,
) -> std::io::Result<PlatformManagedSqliteOpen> {
    let desired_access = match access {
        ManagedSqliteAccess::ReadOnly => FILE_GENERIC_READ | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
        ManagedSqliteAccess::ReadWrite => {
            FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_READ_ATTRIBUTES | SYNCHRONIZE
        }
    };
    let disposition = match mode {
        ManagedSqliteOpenMode::Existing => FILE_OPEN,
        ManagedSqliteOpenMode::OpenOrCreate => FILE_OPEN_IF,
    };
    open_relative(parent, kind.name(), desired_access, disposition)
}

pub(in crate::node_agent_managed_fs) fn open_sqlite_file_for_access_relative(
    parent: &File,
    kind: ManagedSqliteFileKind,
    access: ManagedSqliteAccess,
) -> std::io::Result<PlatformManagedSqliteOpen> {
    let desired_access = match access {
        ManagedSqliteAccess::ReadOnly => FILE_GENERIC_READ | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
        ManagedSqliteAccess::ReadWrite => {
            FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_READ_ATTRIBUTES | SYNCHRONIZE
        }
    };
    open_relative(parent, kind.name(), desired_access, FILE_OPEN)
}

pub(in crate::node_agent_managed_fs) fn open_sqlite_file_for_delete_relative(
    parent: &File,
    kind: ManagedSqliteFileKind,
) -> std::io::Result<PlatformManagedSqliteOpen> {
    open_relative(
        parent,
        kind.name(),
        FILE_READ_ATTRIBUTES | DELETE | SYNCHRONIZE,
        FILE_OPEN,
    )
}

pub(in crate::node_agent_managed_fs) fn read_sqlite_file_at(
    file: &File,
    buffer: &mut [u8],
    offset: u64,
) -> std::io::Result<usize> {
    let length = u32::try_from(buffer.len()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "NODE_MANAGED_SQLITE_READ_TOO_LARGE",
        )
    })?;
    let offset = i64::try_from(offset).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "NODE_MANAGED_SQLITE_READ_OFFSET_INVALID",
        )
    })?;
    let mut io_status = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
    // SAFETY: the synchronous handle remains live; all buffers and the explicit byte offset
    // outlive the call, and no APC or event is requested.
    let status = unsafe {
        NtReadFile(
            file.as_raw_handle() as HANDLE,
            std::ptr::null_mut(),
            None,
            std::ptr::null(),
            io_status.as_mut_ptr(),
            buffer.as_mut_ptr().cast(),
            length,
            &offset,
            std::ptr::null(),
        )
    };
    if status == STATUS_END_OF_FILE {
        return Ok(0);
    }
    checked_io_count(status, io_status, buffer.len())
}

pub(in crate::node_agent_managed_fs) fn write_sqlite_file_at(
    file: &File,
    buffer: &[u8],
    offset: u64,
) -> std::io::Result<usize> {
    let length = u32::try_from(buffer.len()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "NODE_MANAGED_SQLITE_WRITE_TOO_LARGE",
        )
    })?;
    let offset = i64::try_from(offset).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "NODE_MANAGED_SQLITE_WRITE_OFFSET_INVALID",
        )
    })?;
    let mut io_status = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
    // SAFETY: the synchronous handle remains live; all buffers and the explicit byte offset
    // outlive the call, and no APC or event is requested.
    let status = unsafe {
        NtWriteFile(
            file.as_raw_handle() as HANDLE,
            std::ptr::null_mut(),
            None,
            std::ptr::null(),
            io_status.as_mut_ptr(),
            buffer.as_ptr().cast(),
            length,
            &offset,
            std::ptr::null(),
        )
    };
    checked_io_count(status, io_status, buffer.len())
}

pub(in crate::node_agent_managed_fs) fn flush_sqlite_file(file: &File) -> std::io::Result<()> {
    // SAFETY: the borrowed File owns a live handle for the duration of the synchronous call.
    if unsafe { FlushFileBuffers(file.as_raw_handle() as HANDLE) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn open_relative(
    parent: &File,
    name: &OsStr,
    desired_access: u32,
    disposition: u32,
) -> std::io::Result<PlatformManagedSqliteOpen> {
    let mut name_utf16 = name.encode_wide().collect::<Vec<_>>();
    if name_utf16.is_empty()
        || name_utf16.contains(&0)
        || name_utf16.len() > usize::from(u16::MAX) / size_of::<u16>()
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "NODE_MANAGED_SQLITE_NAME_INVALID",
        ));
    }
    let name_bytes = (name_utf16.len() * size_of::<u16>()) as u16;
    let object_name = UNICODE_STRING {
        Length: name_bytes,
        MaximumLength: name_bytes,
        Buffer: name_utf16.as_mut_ptr(),
    };
    let attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: parent.as_raw_handle() as HANDLE,
        ObjectName: &object_name,
        // Exact fixed-name capability: do not permit a differently cased alias to resolve.
        Attributes: 0,
        SecurityDescriptor: std::ptr::null(),
        SecurityQualityOfService: std::ptr::null(),
    };
    let mut handle: HANDLE = std::ptr::null_mut();
    let mut io_status = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
    // SAFETY: the retained parent and relative UTF-16 component outlive this synchronous call. A
    // returned handle is transferred exactly once into File below, including non-success custody.
    let call_status = unsafe {
        NtCreateFile(
            &mut handle,
            desired_access,
            &attributes,
            io_status.as_mut_ptr(),
            std::ptr::null(),
            FILE_ATTRIBUTE_NORMAL,
            SQLITE_SHARE_ACCESS,
            disposition,
            FILE_NON_DIRECTORY_FILE | NT_FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
            std::ptr::null(),
            0,
        )
    };
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return if call_status == STATUS_SUCCESS {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "NODE_MANAGED_SQLITE_OPEN_RETURNED_INVALID_HANDLE",
            ))
        } else {
            Err(ntstatus_error(call_status))
        };
    }
    // SAFETY: NtCreateFile returned one live owned handle and no other File owner exists.
    let file = unsafe { File::from_raw_handle(handle as RawHandle) };
    if call_status != STATUS_SUCCESS {
        return Ok(PlatformManagedSqliteOpen {
            file,
            call_status,
            completion_status: call_status,
            information: usize::MAX,
        });
    }
    // SAFETY: STATUS_SUCCESS means the synchronous call initialized IO_STATUS_BLOCK.
    let io_status = unsafe { io_status.assume_init() };
    // SAFETY: Status is the documented completion arm after synchronous completion.
    let completion_status = unsafe { io_status.Anonymous.Status };
    Ok(PlatformManagedSqliteOpen {
        file,
        call_status,
        completion_status,
        information: io_status.Information,
    })
}

fn checked_io_count(
    status: i32,
    io_status: MaybeUninit<IO_STATUS_BLOCK>,
    requested: usize,
) -> std::io::Result<usize> {
    if status != STATUS_SUCCESS {
        return Err(ntstatus_error(status));
    }
    // SAFETY: STATUS_SUCCESS means the synchronous routine initialized IO_STATUS_BLOCK.
    let io_status = unsafe { io_status.assume_init() };
    // SAFETY: Status is the documented completion arm after synchronous completion.
    let completion = unsafe { io_status.Anonymous.Status };
    if completion != STATUS_SUCCESS {
        return Err(ntstatus_error(completion));
    }
    if io_status.Information > requested {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "NODE_MANAGED_SQLITE_IO_COUNT_INVALID",
        ));
    }
    Ok(io_status.Information)
}
