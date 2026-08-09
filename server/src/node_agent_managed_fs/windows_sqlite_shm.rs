use std::{
    fs::File,
    io,
    mem::MaybeUninit,
    os::windows::io::AsRawHandle,
    ptr::{self, NonNull},
    sync::OnceLock,
};

use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::{
        Memory::{
            CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_READ, FILE_MAP_WRITE,
            MEMORY_MAPPED_VIEW_ADDRESS, PAGE_READWRITE,
        },
        SystemInformation::{GetSystemInfo, SYSTEM_INFO},
    },
};

static ALLOCATION_GRANULARITY: OnceLock<u64> = OnceLock::new();

pub(super) struct OwnedSqliteShmMapping {
    handle: HANDLE,
    outcome_uncertain: bool,
}

pub(super) struct OwnedSqliteShmView {
    address: MEMORY_MAPPED_VIEW_ADDRESS,
    mapped_length: usize,
    outcome_uncertain: bool,
}

// SAFETY: Windows mapping handles are process objects. Access and closure are serialized by the
// SHM coordinator; transferring their numeric values does not transfer an untracked owner.
unsafe impl Send for OwnedSqliteShmMapping {}
unsafe impl Sync for OwnedSqliteShmMapping {}

// SAFETY: the address names one process-wide mapped view retained until coordinator teardown.
// SQLite, rather than Rust, defines synchronized access to its bytes through xShmBarrier/locks.
unsafe impl Send for OwnedSqliteShmView {}
unsafe impl Sync for OwnedSqliteShmView {}

pub(super) fn allocation_granularity() -> io::Result<u64> {
    let value = *ALLOCATION_GRANULARITY.get_or_init(|| {
        let mut information = MaybeUninit::<SYSTEM_INFO>::zeroed();
        // SAFETY: GetSystemInfo initializes the caller-provided SYSTEM_INFO structure.
        unsafe { GetSystemInfo(information.as_mut_ptr()) };
        // SAFETY: the API has initialized every public field of SYSTEM_INFO.
        u64::from(unsafe { information.assume_init() }.dwAllocationGranularity)
    });
    if value == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "NODE_MANAGED_SQLITE_SHM_ALLOCATION_GRANULARITY_ZERO",
        ));
    }
    Ok(value)
}

pub(super) fn create_mapping(file: &File, maximum_size: u64) -> io::Result<OwnedSqliteShmMapping> {
    if maximum_size == 0 || maximum_size > i64::MAX as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "NODE_MANAGED_SQLITE_SHM_MAPPING_SIZE_INVALID",
        ));
    }
    // SAFETY: `file` owns a live file handle, the file was grown before this call, and both the
    // security descriptor and global mapping name are deliberately null.
    let handle = unsafe {
        CreateFileMappingW(
            file.as_raw_handle() as HANDLE,
            ptr::null(),
            PAGE_READWRITE,
            (maximum_size >> 32) as u32,
            maximum_size as u32,
            ptr::null(),
        )
    };
    if handle.is_null() {
        return Err(io::Error::last_os_error());
    }
    Ok(OwnedSqliteShmMapping {
        handle,
        outcome_uncertain: false,
    })
}

pub(super) fn map_view(
    mapping: &OwnedSqliteShmMapping,
    aligned_offset: u64,
    mapped_length: usize,
) -> io::Result<OwnedSqliteShmView> {
    if mapped_length == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "NODE_MANAGED_SQLITE_SHM_VIEW_LENGTH_ZERO",
        ));
    }
    let granularity = allocation_granularity()?;
    if aligned_offset % granularity != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "NODE_MANAGED_SQLITE_SHM_VIEW_OFFSET_UNALIGNED",
        ));
    }
    // SAFETY: the retained mapping handle covers this checked file range. The returned address is
    // owned exactly once by OwnedSqliteShmView and later passed unchanged to UnmapViewOfFile.
    let address = unsafe {
        MapViewOfFile(
            mapping.handle,
            FILE_MAP_READ | FILE_MAP_WRITE,
            (aligned_offset >> 32) as u32,
            aligned_offset as u32,
            mapped_length,
        )
    };
    if address.Value.is_null() {
        return Err(io::Error::last_os_error());
    }
    Ok(OwnedSqliteShmView {
        address,
        mapped_length,
        outcome_uncertain: false,
    })
}

impl OwnedSqliteShmView {
    pub(super) fn base(&self) -> Option<NonNull<u8>> {
        NonNull::new(self.address.Value.cast())
    }

    pub(super) fn mapped_length(&self) -> usize {
        self.mapped_length
    }

    /// Leaves the original address retained when Windows reports failure, so the caller can move
    /// this owner into quarantined teardown custody instead of claiming the view was released.
    pub(super) fn unmap_explicit(&mut self) -> io::Result<()> {
        if self.outcome_uncertain {
            return Err(io::Error::other(
                "NODE_MANAGED_SQLITE_SHM_VIEW_UNMAP_ALREADY_UNCERTAIN",
            ));
        }
        if self.address.Value.is_null() {
            return Ok(());
        }
        // SAFETY: this is the exact aligned base returned by MapViewOfFile, not SQLite's shifted
        // logical region pointer.
        if unsafe { UnmapViewOfFile(self.address) } == 0 {
            self.outcome_uncertain = true;
            return Err(io::Error::last_os_error());
        }
        self.address.Value = ptr::null_mut();
        Ok(())
    }
}

impl OwnedSqliteShmMapping {
    /// Leaves the handle value retained on failure. The coordinator must poison itself and keep
    /// this owner rather than retrying normal SHM initialization through a second mapping domain.
    pub(super) fn close_explicit(&mut self) -> io::Result<()> {
        if self.outcome_uncertain {
            return Err(io::Error::other(
                "NODE_MANAGED_SQLITE_SHM_MAPPING_CLOSE_ALREADY_UNCERTAIN",
            ));
        }
        if self.handle.is_null() {
            return Ok(());
        }
        // SAFETY: this value is the sole owner of the CreateFileMappingW handle.
        if unsafe { CloseHandle(self.handle) } == 0 {
            self.outcome_uncertain = true;
            return Err(io::Error::last_os_error());
        }
        self.handle = ptr::null_mut();
        Ok(())
    }
}

impl Drop for OwnedSqliteShmView {
    fn drop(&mut self) {
        if !self.outcome_uncertain {
            let _ = self.unmap_explicit();
        }
    }
}

impl Drop for OwnedSqliteShmMapping {
    fn drop(&mut self) {
        if !self.outcome_uncertain {
            let _ = self.close_explicit();
        }
    }
}
