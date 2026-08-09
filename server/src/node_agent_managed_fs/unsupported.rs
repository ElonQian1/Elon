use std::{
    ffi::OsStr,
    fs::File,
    path::{Path, PathBuf},
};

use super::{
    namespace::PlatformParentRelativeObservation, ManagedSqliteAccess, ManagedSqliteFileKind,
    ManagedSqliteOpenMode, PlatformFileIdentity, PlatformNamespaceDurabilityReceipt,
    PlatformNamespaceFlushFailure,
};

pub(super) struct PlatformManagedSqliteOpen {
    pub(super) file: File,
    pub(super) call_status: i32,
    pub(super) completion_status: i32,
    pub(super) information: usize,
}

fn unsupported() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "NODE_MANAGED_FS_PLATFORM_UNSUPPORTED",
    )
}

pub(super) fn open_initial_directory(_path: &Path) -> std::io::Result<File> {
    Err(unsupported())
}

pub(super) fn open_directory_relative(_parent: &File, _name: &OsStr) -> std::io::Result<File> {
    Err(unsupported())
}

pub(super) fn open_managed_directory_relative(
    _parent: &File,
    _name: &OsStr,
) -> std::io::Result<File> {
    Err(unsupported())
}

pub(super) fn create_new_directory_relative(
    _parent: &File,
    _name: &OsStr,
) -> std::io::Result<File> {
    Err(unsupported())
}

pub(super) fn open_directory_relative_deletable(
    _parent: &File,
    _name: &OsStr,
) -> std::io::Result<File> {
    Err(unsupported())
}

pub(super) fn open_existing_file_relative(
    _parent: &File,
    _name: &OsStr,
    _writable: bool,
) -> std::io::Result<File> {
    Err(unsupported())
}

pub(super) fn canonical_path(_file: &File) -> std::io::Result<PathBuf> {
    Err(unsupported())
}

pub(super) fn create_new_file_relative(_parent: &File, _name: &OsStr) -> std::io::Result<File> {
    Err(unsupported())
}

pub(super) fn open_existing_file_relative_deletable(
    _parent: &File,
    _name: &OsStr,
) -> std::io::Result<File> {
    Err(unsupported())
}

pub(super) fn open_sqlite_file_relative(
    _parent: &File,
    _kind: ManagedSqliteFileKind,
    _access: ManagedSqliteAccess,
    _mode: ManagedSqliteOpenMode,
) -> std::io::Result<PlatformManagedSqliteOpen> {
    Err(unsupported())
}

pub(super) fn open_sqlite_file_for_access_relative(
    _parent: &File,
    _kind: ManagedSqliteFileKind,
    _access: ManagedSqliteAccess,
) -> std::io::Result<PlatformManagedSqliteOpen> {
    Err(unsupported())
}

pub(super) fn open_sqlite_file_for_delete_relative(
    _parent: &File,
    _kind: ManagedSqliteFileKind,
) -> std::io::Result<PlatformManagedSqliteOpen> {
    Err(unsupported())
}

pub(super) fn read_sqlite_file_at(
    _file: &File,
    _buffer: &mut [u8],
    _offset: u64,
) -> std::io::Result<usize> {
    Err(unsupported())
}

pub(super) fn write_sqlite_file_at(
    _file: &File,
    _buffer: &[u8],
    _offset: u64,
) -> std::io::Result<usize> {
    Err(unsupported())
}

pub(super) fn flush_sqlite_file(_file: &File) -> std::io::Result<()> {
    Err(unsupported())
}

pub(super) fn delete_by_handle(_file: &File) -> std::io::Result<()> {
    Err(unsupported())
}

pub(super) fn flush_namespace_directory(
    _directory: &File,
) -> Result<PlatformNamespaceDurabilityReceipt, PlatformNamespaceFlushFailure> {
    Err(PlatformNamespaceFlushFailure::unsupported(unsupported()))
}

pub(super) fn observe_child_relative(
    _parent: &File,
    _name: &OsStr,
) -> std::io::Result<PlatformParentRelativeObservation> {
    Err(unsupported())
}

pub(super) fn inspect(_file: &File) -> std::io::Result<PlatformFileIdentity> {
    Err(unsupported())
}
