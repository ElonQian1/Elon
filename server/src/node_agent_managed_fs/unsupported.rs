use std::{
    ffi::OsStr,
    fs::File,
    path::{Path, PathBuf},
};

use super::PlatformFileIdentity;

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

pub(super) fn create_new_directory_relative(
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

pub(super) fn inspect(_file: &File) -> std::io::Result<PlatformFileIdentity> {
    Err(unsupported())
}
