use std::{
    ffi::OsStr,
    fs::File,
    mem::{size_of, MaybeUninit},
    os::windows::io::AsRawHandle,
};

use windows_sys::{
    Wdk::Storage::FileSystem::{
        FileAccessInformation, NtQueryInformationFile, FILE_ACCESS_INFORMATION,
        FILE_DIRECTORY_FILE, FILE_OPEN, FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
    },
    Win32::{
        Foundation::{HANDLE, STATUS_SUCCESS},
        Storage::FileSystem::{
            DELETE, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
            FILE_TRAVERSE, FILE_WRITE_DATA, SYNCHRONIZE,
        },
        System::IO::IO_STATUS_BLOCK,
    },
};

use super::{ntstatus_error, open_relative};
use crate::node_agent_managed_fs::extraction_loader_directory::{
    PlatformExtractionLoaderDirectoryProbe, PlatformExtractionLoaderDirectoryProbeFailure,
};

const PROBE_DESIRED_ACCESS: u32 = FILE_READ_ATTRIBUTES | FILE_TRAVERSE | SYNCHRONIZE;
const PROBE_SHARE_ACCESS: u32 = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;
const PROBE_CREATE_OPTIONS: u32 =
    FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT;
pub(super) fn probe_extraction_loader_directory_relative(
    retained_delete_owner: &File,
    parent: &File,
    name: &OsStr,
) -> Result<PlatformExtractionLoaderDirectoryProbe, PlatformExtractionLoaderDirectoryProbeFailure> {
    let retained_access = query_granted_access(retained_delete_owner)
        .map_err(PlatformExtractionLoaderDirectoryProbeFailure::before_probe)?;
    if retained_access & DELETE != DELETE {
        return Err(PlatformExtractionLoaderDirectoryProbeFailure::before_probe(
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "NODE_MANAGED_EXTRACTION_LOADER_RETAINED_DELETE_ACCESS_MISSING",
            ),
        ));
    }

    let probe = open_relative(
        parent,
        name,
        PROBE_DESIRED_ACCESS,
        FILE_OPEN,
        PROBE_CREATE_OPTIONS,
        PROBE_SHARE_ACCESS,
    )
    .map_err(PlatformExtractionLoaderDirectoryProbeFailure::before_probe)?;
    let probe_access = match query_granted_access(&probe) {
        Ok(access) => access,
        Err(error) => {
            return Err(PlatformExtractionLoaderDirectoryProbeFailure::after_probe(
                error, probe,
            ));
        }
    };
    if probe_access != PROBE_DESIRED_ACCESS || probe_access & (DELETE | FILE_WRITE_DATA) != 0 {
        return Err(PlatformExtractionLoaderDirectoryProbeFailure::after_probe(
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "NODE_MANAGED_EXTRACTION_LOADER_PROBE_ACCESS_CHANGED",
            ),
            probe,
        ));
    }

    Ok(PlatformExtractionLoaderDirectoryProbe {
        file: probe,
        retained_delete_owner_granted_access: retained_access,
        probe_granted_access: probe_access,
    })
}

fn query_granted_access(file: &File) -> std::io::Result<u32> {
    let mut access = MaybeUninit::<FILE_ACCESS_INFORMATION>::uninit();
    let mut io_status = MaybeUninit::<IO_STATUS_BLOCK>::uninit();
    // SAFETY: the File owns a live handle and both fixed-size output buffers remain valid for the
    // synchronous query. The result is read only after both NT and completion statuses succeed.
    let status = unsafe {
        NtQueryInformationFile(
            file.as_raw_handle() as HANDLE,
            io_status.as_mut_ptr(),
            access.as_mut_ptr().cast(),
            size_of::<FILE_ACCESS_INFORMATION>() as u32,
            FileAccessInformation,
        )
    };
    if status != STATUS_SUCCESS {
        return Err(ntstatus_error(status));
    }
    // SAFETY: STATUS_SUCCESS initializes the synchronous IO status block.
    let io_status = unsafe { io_status.assume_init() };
    // SAFETY: Status is the active completion arm for NtQueryInformationFile.
    let completion_status = unsafe { io_status.Anonymous.Status };
    if completion_status != STATUS_SUCCESS {
        return Err(ntstatus_error(completion_status));
    }
    // SAFETY: both statuses succeeded, so FILE_ACCESS_INFORMATION is initialized.
    Ok(unsafe { access.assume_init() }.AccessFlags)
}
