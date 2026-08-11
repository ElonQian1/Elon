use std::{
    fs::{File, Metadata, OpenOptions},
    path::Path,
};

use crate::compute_federation::external_pool_adapter_artifact_source::ExternalPoolAdapterArtifactSourceFsError;

use super::paths::unsafe_file_type;

pub(super) fn open_verified_final(
    path: &Path,
    quarantine_root: &Path,
) -> Result<File, ExternalPoolAdapterArtifactSourceFsError> {
    let path_metadata = path_metadata(path)?;
    if unsafe_regular_file(&path_metadata) {
        return Err(ExternalPoolAdapterArtifactSourceFsError::UnsafeTarget);
    }

    let file = open_no_follow(path).map_err(|error| classify_open_error(path, error))?;
    let handle_metadata = file
        .metadata()
        .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
    if unsafe_regular_file(&handle_metadata) {
        return Err(ExternalPoolAdapterArtifactSourceFsError::UnsafeTarget);
    }
    require_current_path_identity(path, quarantine_root, &file)?;
    Ok(file)
}

pub(super) fn require_current_path_identity(
    path: &Path,
    quarantine_root: &Path,
    file: &File,
) -> Result<(), ExternalPoolAdapterArtifactSourceFsError> {
    let path_metadata = path_metadata(path)?;
    if unsafe_regular_file(&path_metadata) {
        return Err(ExternalPoolAdapterArtifactSourceFsError::UnsafeTarget);
    }

    let canonical_root = canonicalize(quarantine_root)?;
    let canonical_path = canonicalize(path)?;
    if canonical_path == canonical_root || !canonical_path.starts_with(&canonical_root) {
        return Err(ExternalPoolAdapterArtifactSourceFsError::UnsafeTarget);
    }

    let verifier = open_no_follow(path).map_err(|error| classify_open_error(path, error))?;
    let verifier_metadata = verifier
        .metadata()
        .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
    if unsafe_regular_file(&verifier_metadata)
        || !same_file_identity(file, &verifier)?
        || !path_metadata_matches_file(&path_metadata, file)?
    {
        return Err(ExternalPoolAdapterArtifactSourceFsError::UnsafeTarget);
    }
    Ok(())
}

fn canonicalize(
    path: &Path,
) -> Result<std::path::PathBuf, ExternalPoolAdapterArtifactSourceFsError> {
    match std::fs::canonicalize(path) {
        Ok(value) => Ok(value),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(ExternalPoolAdapterArtifactSourceFsError::BlobMissing)
        }
        Err(error) => Err(ExternalPoolAdapterArtifactSourceFsError::Storage(error)),
    }
}

fn path_metadata(path: &Path) -> Result<Metadata, ExternalPoolAdapterArtifactSourceFsError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(ExternalPoolAdapterArtifactSourceFsError::BlobMissing)
        }
        Err(error) => Err(ExternalPoolAdapterArtifactSourceFsError::Storage(error)),
    }
}

fn classify_open_error(
    path: &Path,
    error: std::io::Error,
) -> ExternalPoolAdapterArtifactSourceFsError {
    match std::fs::symlink_metadata(path) {
        Err(metadata_error) if metadata_error.kind() == std::io::ErrorKind::NotFound => {
            ExternalPoolAdapterArtifactSourceFsError::BlobMissing
        }
        Ok(metadata) if unsafe_regular_file(&metadata) => {
            ExternalPoolAdapterArtifactSourceFsError::UnsafeTarget
        }
        _ => ExternalPoolAdapterArtifactSourceFsError::Storage(error),
    }
}

pub(super) fn unsafe_regular_file(metadata: &Metadata) -> bool {
    if unsafe_file_type(metadata) || !metadata.is_file() {
        return true;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        return metadata.mode() & 0o077 != 0;
    }
    #[cfg(not(unix))]
    false
}

#[cfg(windows)]
fn open_no_follow(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;

    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}

#[cfg(unix)]
fn open_no_follow(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    OpenOptions::new()
        .read(true)
        .custom_flags(unix_no_follow_flag())
        .open(path)
}

#[cfg(all(
    any(target_os = "linux", target_os = "android"),
    any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "powerpc",
        target_arch = "powerpc64"
    )
))]
const fn unix_no_follow_flag() -> i32 {
    0x8000
}

#[cfg(all(target_os = "android", target_arch = "riscv64"))]
const fn unix_no_follow_flag() -> i32 {
    0x400000
}

#[cfg(all(
    any(target_os = "linux", target_os = "android"),
    not(any(
        target_arch = "arm",
        target_arch = "aarch64",
        target_arch = "powerpc",
        target_arch = "powerpc64"
    )),
    not(all(target_os = "android", target_arch = "riscv64"))
))]
const fn unix_no_follow_flag() -> i32 {
    0x20000
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
const fn unix_no_follow_flag() -> i32 {
    0x100
}

#[cfg(any(target_os = "solaris", target_os = "illumos"))]
const fn unix_no_follow_flag() -> i32 {
    0x20000
}

#[cfg(target_os = "aix")]
const fn unix_no_follow_flag() -> i32 {
    0x1000000
}

#[cfg(target_os = "haiku")]
const fn unix_no_follow_flag() -> i32 {
    0x00080000
}

#[cfg(target_os = "hurd")]
const fn unix_no_follow_flag() -> i32 {
    0x100000
}

#[cfg(target_os = "redox")]
const fn unix_no_follow_flag() -> i32 {
    i32::MIN
}

#[cfg(unix)]
fn path_metadata_matches_file(
    path_metadata: &Metadata,
    file: &File,
) -> Result<bool, ExternalPoolAdapterArtifactSourceFsError> {
    use std::os::unix::fs::MetadataExt;

    let file_metadata = file
        .metadata()
        .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
    Ok(path_metadata.dev() == file_metadata.dev() && path_metadata.ino() == file_metadata.ino())
}

#[cfg(windows)]
fn path_metadata_matches_file(
    _path_metadata: &Metadata,
    _file: &File,
) -> Result<bool, ExternalPoolAdapterArtifactSourceFsError> {
    // The retained leaf handle excludes DELETE sharing, so the verified path cannot be replaced.
    Ok(true)
}

#[cfg(unix)]
fn same_file_identity(
    left: &File,
    right: &File,
) -> Result<bool, ExternalPoolAdapterArtifactSourceFsError> {
    use std::os::unix::fs::MetadataExt;

    let left = left
        .metadata()
        .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
    let right = right
        .metadata()
        .map_err(ExternalPoolAdapterArtifactSourceFsError::Storage)?;
    Ok(left.dev() == right.dev() && left.ino() == right.ino())
}

#[cfg(windows)]
fn same_file_identity(
    left: &File,
    right: &File,
) -> Result<bool, ExternalPoolAdapterArtifactSourceFsError> {
    Ok(windows_file_identity(left)? == windows_file_identity(right)?)
}

#[cfg(windows)]
fn windows_file_identity(
    file: &File,
) -> Result<(u32, u64), ExternalPoolAdapterArtifactSourceFsError> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) };
    if ok == 0 {
        return Err(ExternalPoolAdapterArtifactSourceFsError::Storage(
            std::io::Error::last_os_error(),
        ));
    }
    Ok((
        information.dwVolumeSerialNumber,
        ((information.nFileIndexHigh as u64) << 32) | information.nFileIndexLow as u64,
    ))
}
