use std::{
    ffi::{OsStr, OsString},
    fs::{File, OpenOptions},
    mem::{size_of, MaybeUninit},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::OpenOptionsExt,
        io::{AsRawHandle, FromRawHandle, RawHandle},
    },
    path::{Path, PathBuf},
};

use windows_sys::{
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        Storage::FileSystem::{
            NtCreateFile, NtFlushBuffersFileEx, FILE_CREATE, FILE_DIRECTORY_FILE,
            FILE_NON_DIRECTORY_FILE, FILE_OPEN,
            FILE_OPEN_REPARSE_POINT as NT_FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
        },
    },
    Win32::{
        Foundation::{
            RtlNtStatusToDosError, HANDLE, INVALID_HANDLE_VALUE, NTSTATUS, STATUS_NO_SUCH_FILE,
            STATUS_OBJECT_NAME_NOT_FOUND, STATUS_SUCCESS, UNICODE_STRING,
        },
        Storage::FileSystem::{
            FileDispositionInfoEx, FileIdInfo, GetFileInformationByHandle,
            GetFileInformationByHandleEx, GetFinalPathNameByHandleW, GetVolumeInformationByHandleW,
            SetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, DELETE,
            FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_REPARSE_POINT,
            FILE_DISPOSITION_FLAG_DELETE, FILE_DISPOSITION_FLAG_IGNORE_READONLY_ATTRIBUTE,
            FILE_DISPOSITION_FLAG_POSIX_SEMANTICS, FILE_DISPOSITION_INFO_EX,
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ,
            FILE_GENERIC_WRITE, FILE_ID_INFO, FILE_NAME_NORMALIZED, FILE_READ_ATTRIBUTES,
            FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TRAVERSE, FILE_WRITE_DATA,
            SYNCHRONIZE, VOLUME_NAME_GUID,
        },
        System::{Kernel::OBJ_CASE_INSENSITIVE, IO::IO_STATUS_BLOCK},
    },
};

use super::{
    namespace::PlatformParentRelativeObservation, PlatformFileIdentity,
    PlatformNamespaceDurabilityReceipt, PlatformNamespaceFlushFailure,
};

#[path = "windows_sqlite.rs"]
mod sqlite;
#[path = "windows_sqlite_locking.rs"]
mod sqlite_locking;
pub(super) use sqlite::{
    flush_sqlite_file, open_sqlite_file_for_access_relative, open_sqlite_file_for_delete_relative,
    open_sqlite_file_relative, read_sqlite_file_at, write_sqlite_file_at,
    PlatformManagedSqliteOpen,
};
pub(super) use sqlite_locking::{try_lock_sqlite_byte_range, unlock_sqlite_byte_range};

const MAX_FINAL_PATH_UTF16: usize = 32_768;

/// The configured namespace is used only to acquire the first volume-root handle. Every child
/// below it is opened relative to its already pinned parent handle.
pub(super) fn open_initial_directory(path: &Path) -> std::io::Result<File> {
    OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES | FILE_TRAVERSE | SYNCHRONIZE)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
}

pub(super) fn open_directory_relative(parent: &File, name: &OsStr) -> std::io::Result<File> {
    open_relative(
        parent,
        name,
        FILE_READ_ATTRIBUTES | FILE_TRAVERSE | SYNCHRONIZE,
        FILE_OPEN,
        FILE_DIRECTORY_FILE | NT_FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
        FILE_SHARE_READ,
    )
}

/// Opens a directory inside the managed root with the minimum directory-write bit required by the
/// native namespace durability barrier. Delete sharing remains denied, so the same retained handle
/// also continues to fence rename or deletion of the directory itself.
pub(super) fn open_managed_directory_relative(
    parent: &File,
    name: &OsStr,
) -> std::io::Result<File> {
    open_relative(
        parent,
        name,
        FILE_READ_ATTRIBUTES | FILE_TRAVERSE | FILE_WRITE_DATA | SYNCHRONIZE,
        FILE_OPEN,
        FILE_DIRECTORY_FILE | NT_FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
    )
}

pub(super) fn create_new_directory_relative(parent: &File, name: &OsStr) -> std::io::Result<File> {
    open_relative(
        parent,
        name,
        FILE_READ_ATTRIBUTES | FILE_TRAVERSE | FILE_WRITE_DATA | SYNCHRONIZE | DELETE,
        FILE_CREATE,
        FILE_DIRECTORY_FILE | NT_FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
    )
}

pub(super) fn open_directory_relative_deletable(
    parent: &File,
    name: &OsStr,
) -> std::io::Result<File> {
    open_relative(
        parent,
        name,
        FILE_READ_ATTRIBUTES | FILE_TRAVERSE | FILE_WRITE_DATA | SYNCHRONIZE | DELETE,
        FILE_OPEN,
        FILE_DIRECTORY_FILE | NT_FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
    )
}

pub(super) fn open_existing_file_relative(
    parent: &File,
    name: &OsStr,
    writable: bool,
) -> std::io::Result<File> {
    let access = if writable {
        FILE_GENERIC_READ | FILE_GENERIC_WRITE
    } else {
        FILE_GENERIC_READ
    };
    open_relative(
        parent,
        name,
        access,
        FILE_OPEN,
        FILE_NON_DIRECTORY_FILE | NT_FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
        0,
    )
}

pub(super) fn create_new_file_relative(parent: &File, name: &OsStr) -> std::io::Result<File> {
    open_relative(
        parent,
        name,
        FILE_GENERIC_READ | FILE_GENERIC_WRITE | DELETE,
        FILE_CREATE,
        FILE_NON_DIRECTORY_FILE | NT_FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
        0,
    )
}

pub(super) fn open_existing_file_relative_deletable(
    parent: &File,
    name: &OsStr,
) -> std::io::Result<File> {
    open_relative(
        parent,
        name,
        FILE_GENERIC_READ | DELETE,
        FILE_OPEN,
        FILE_NON_DIRECTORY_FILE | NT_FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
        0,
    )
}

pub(super) fn observe_child_relative(
    parent: &File,
    name: &OsStr,
) -> std::io::Result<PlatformParentRelativeObservation> {
    match open_relative_raw(
        parent,
        name,
        FILE_READ_ATTRIBUTES | SYNCHRONIZE,
        FILE_OPEN,
        NT_FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
    ) {
        Ok(file) => Ok(PlatformParentRelativeObservation::Present(file)),
        Err(RelativeOpenFailure::NtStatus(status))
            if status == STATUS_OBJECT_NAME_NOT_FOUND || status == STATUS_NO_SUCH_FILE =>
        {
            Ok(PlatformParentRelativeObservation::Absent)
        }
        Err(error) => Err(error.into_io_error()),
    }
}

pub(super) fn delete_by_handle(file: &File) -> std::io::Result<()> {
    let disposition = FILE_DISPOSITION_INFO_EX {
        Flags: FILE_DISPOSITION_FLAG_DELETE
            | FILE_DISPOSITION_FLAG_POSIX_SEMANTICS
            | FILE_DISPOSITION_FLAG_IGNORE_READONLY_ATTRIBUTE,
    };
    // SAFETY: the borrowed File owns a live handle with DELETE access and the immutable input
    // structure remains valid for the synchronous call.
    if unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle() as HANDLE,
            FileDispositionInfoEx,
            (&disposition as *const FILE_DISPOSITION_INFO_EX).cast(),
            size_of::<FILE_DISPOSITION_INFO_EX>() as u32,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// Flushes directory data, metadata, and the underlying storage cache synchronously.
pub(super) fn flush_namespace_directory(
    directory: &File,
) -> Result<PlatformNamespaceDurabilityReceipt, PlatformNamespaceFlushFailure> {
    let filesystem_kind =
        ensure_supported_namespace_durability_filesystem(directory).map_err(|error| {
            if error.kind() == std::io::ErrorKind::Unsupported {
                PlatformNamespaceFlushFailure::unsupported(error)
            } else {
                PlatformNamespaceFlushFailure::retryable(error)
            }
        })?;
    let mut io_status = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
    // SAFETY: the borrowed File owns a live write-capable directory handle, flags=0 is the normal
    // data+metadata+storage-cache flush, and the synchronous output buffer remains live.
    let status = unsafe {
        NtFlushBuffersFileEx(
            directory.as_raw_handle() as HANDLE,
            0,
            std::ptr::null(),
            0,
            io_status.as_mut_ptr(),
        )
    };
    if status < 0 {
        return Err(PlatformNamespaceFlushFailure::retryable(ntstatus_error(
            status,
        )));
    }
    if status != STATUS_SUCCESS {
        return Err(PlatformNamespaceFlushFailure::outcome_uncertain(
            ntstatus_error(status),
        ));
    }
    // SAFETY: STATUS_SUCCESS means the synchronous routine initialized the output block.
    let io_status = unsafe { io_status.assume_init() };
    // SAFETY: the Status arm is the documented completion result for this call.
    let completion_status = unsafe { io_status.Anonymous.Status };
    if completion_status != STATUS_SUCCESS {
        return Err(PlatformNamespaceFlushFailure::outcome_uncertain(
            ntstatus_error(completion_status),
        ));
    }
    Ok(PlatformNamespaceDurabilityReceipt::new(filesystem_kind))
}

fn ensure_supported_namespace_durability_filesystem(
    directory: &File,
) -> std::io::Result<&'static str> {
    let mut filesystem_name = [0u16; 32];
    // SAFETY: the borrowed File owns a live handle and the UTF-16 output buffer is writable for
    // the advertised length. Optional outputs are null as permitted by the API.
    if unsafe {
        GetVolumeInformationByHandleW(
            directory.as_raw_handle() as HANDLE,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            filesystem_name.as_mut_ptr(),
            filesystem_name.len() as u32,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error());
    }
    let length = filesystem_name
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(filesystem_name.len());
    let filesystem = String::from_utf16(&filesystem_name[..length]).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "NODE_MANAGED_NAMESPACE_FILESYSTEM_NAME_INVALID",
        )
    })?;
    if filesystem.eq_ignore_ascii_case("NTFS") {
        Ok("ntfs")
    } else if filesystem.eq_ignore_ascii_case("ReFS") {
        Ok("refs")
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "NODE_MANAGED_NAMESPACE_FILESYSTEM_UNSUPPORTED",
        ))
    }
}

fn open_relative(
    parent: &File,
    name: &OsStr,
    desired_access: u32,
    create_disposition: u32,
    create_options: u32,
    share_access: u32,
) -> std::io::Result<File> {
    open_relative_raw(
        parent,
        name,
        desired_access,
        create_disposition,
        create_options,
        share_access,
    )
    .map_err(RelativeOpenFailure::into_io_error)
}

enum RelativeOpenFailure {
    Io(std::io::Error),
    NtStatus(NTSTATUS),
}

impl RelativeOpenFailure {
    fn into_io_error(self) -> std::io::Error {
        match self {
            Self::Io(error) => error,
            Self::NtStatus(status) => ntstatus_error(status),
        }
    }
}

fn open_relative_raw(
    parent: &File,
    name: &OsStr,
    desired_access: u32,
    create_disposition: u32,
    create_options: u32,
    share_access: u32,
) -> Result<File, RelativeOpenFailure> {
    let mut name_utf16 = name.encode_wide().collect::<Vec<_>>();
    if name_utf16.is_empty()
        || name_utf16.contains(&0)
        || name_utf16.len() > usize::from(u16::MAX) / size_of::<u16>()
    {
        return Err(RelativeOpenFailure::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "NODE_MANAGED_RELATIVE_NAME_INVALID",
        )));
    }
    let name_bytes = (name_utf16.len() * size_of::<u16>()) as u16;
    let object_name = UNICODE_STRING {
        Length: name_bytes,
        MaximumLength: name_bytes,
        Buffer: name_utf16.as_mut_ptr(),
    };
    let object_attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: parent.as_raw_handle() as HANDLE,
        ObjectName: &object_name,
        Attributes: OBJ_CASE_INSENSITIVE as u32,
        SecurityDescriptor: std::ptr::null(),
        SecurityQualityOfService: std::ptr::null(),
    };
    let mut handle: HANDLE = std::ptr::null_mut();
    let mut io_status = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
    // SAFETY: the parent File remains live, the relative UTF-16 name and object structures outlive
    // the synchronous call, and a successful handle is transferred exactly once into `File`.
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            desired_access,
            &object_attributes,
            io_status.as_mut_ptr(),
            std::ptr::null(),
            FILE_ATTRIBUTE_NORMAL,
            share_access,
            create_disposition,
            create_options,
            std::ptr::null(),
            0,
        )
    };
    if status < 0 {
        return Err(RelativeOpenFailure::NtStatus(status));
    }
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(RelativeOpenFailure::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "NODE_MANAGED_RELATIVE_OPEN_INVALID_HANDLE",
        )));
    }
    // SAFETY: NtCreateFile returned one owned live handle and no other owner was constructed.
    Ok(unsafe { File::from_raw_handle(handle as RawHandle) })
}

fn ntstatus_error(status: NTSTATUS) -> std::io::Error {
    // SAFETY: RtlNtStatusToDosError is a pure conversion for the returned NTSTATUS.
    let code = unsafe { RtlNtStatusToDosError(status) };
    std::io::Error::from_raw_os_error(code as i32)
}

pub(super) fn canonical_path(file: &File) -> std::io::Result<PathBuf> {
    let handle = file.as_raw_handle() as HANDLE;
    let flags = FILE_NAME_NORMALIZED | VOLUME_NAME_GUID;
    // SAFETY: a null/zero buffer is the documented size query for this live handle.
    let required = unsafe { GetFinalPathNameByHandleW(handle, std::ptr::null_mut(), 0, flags) };
    if required == 0 {
        return Err(std::io::Error::last_os_error());
    }
    let required = required as usize;
    if required > MAX_FINAL_PATH_UTF16 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "NODE_MANAGED_FINAL_PATH_TOO_LONG",
        ));
    }
    let mut buffer = vec![0u16; required + 1];
    // SAFETY: `buffer` is writable for its advertised length and the File remains live.
    let written = unsafe {
        GetFinalPathNameByHandleW(handle, buffer.as_mut_ptr(), buffer.len() as u32, flags)
    } as usize;
    if written == 0 {
        return Err(std::io::Error::last_os_error());
    }
    if written >= buffer.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "NODE_MANAGED_FINAL_PATH_CHANGED_DURING_READ",
        ));
    }
    buffer.truncate(written);
    let path = PathBuf::from(OsString::from_wide(&buffer));
    let stable_volume_prefix = r"\\?\Volume{";
    let path_text = path.to_string_lossy();
    let has_stable_volume_prefix = path_text
        .get(..stable_volume_prefix.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(stable_volume_prefix));
    if !path.is_absolute() || !has_stable_volume_prefix {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "NODE_MANAGED_FINAL_PATH_NOT_STABLE_VOLUME_GUID",
        ));
    }
    Ok(path)
}

pub(super) fn inspect(file: &File) -> std::io::Result<PlatformFileIdentity> {
    let handle = file.as_raw_handle() as HANDLE;
    let mut basic = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    // SAFETY: `handle` belongs to the borrowed live File and `basic` is a correctly sized output.
    if unsafe { GetFileInformationByHandle(handle, basic.as_mut_ptr()) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: the successful Windows call initialized the complete structure.
    let basic = unsafe { basic.assume_init() };

    let mut extended = MaybeUninit::<FILE_ID_INFO>::uninit();
    // SAFETY: `extended` matches FileIdInfo and the buffer remains live for the call.
    if unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            extended.as_mut_ptr().cast(),
            size_of::<FILE_ID_INFO>() as u32,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: a successful FileIdInfo query initialized the complete structure.
    let extended = unsafe { extended.assume_init() };
    let file_size = (u64::from(basic.nFileSizeHigh) << 32) | u64::from(basic.nFileSizeLow);
    Ok(PlatformFileIdentity {
        volume_serial: extended.VolumeSerialNumber,
        file_id: extended.FileId.Identifier,
        number_of_links: basic.nNumberOfLinks,
        file_size,
        is_directory: basic.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0,
        is_reparse_point: basic.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0,
    })
}
