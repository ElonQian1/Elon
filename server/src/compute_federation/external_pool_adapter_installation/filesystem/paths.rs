use std::{
    fs::{File, Metadata, OpenOptions},
    path::{Path, PathBuf},
};

use super::ExternalPoolAdapterInstallationFsError;

const INSTALL_COMPONENTS: [&str; 5] = [
    "compute-federation",
    "external-pool-adapter-artifacts",
    "v1",
    "installed-inert",
    "sha256",
];

pub(super) struct InstallationPaths {
    pub(super) shard: PathBuf,
    pub(super) final_root: PathBuf,
    pub(super) staging_root: PathBuf,
    pub(super) pinned_directories: Vec<File>,
}

pub(super) fn prepare_paths(
    data_dir: &Path,
    digest: &str,
) -> Result<InstallationPaths, ExternalPoolAdapterInstallationFsError> {
    validate_digest(digest)?;
    ensure_directory(data_dir, false, true)?;
    let mut pinned = Vec::new();
    pin_directory(data_dir, &mut pinned)?;
    let mut current = data_dir.to_path_buf();
    for component in INSTALL_COMPONENTS {
        current.push(component);
        ensure_directory(&current, true, true)?;
        pin_directory(&current, &mut pinned)?;
    }
    current.push(&digest[..2]);
    ensure_directory(&current, true, true)?;
    pin_directory(&current, &mut pinned)?;
    let final_root = current.join(digest);
    let staging_root = current.join(format!(".{digest}.{}.part", uuid::Uuid::new_v4().simple()));
    Ok(InstallationPaths {
        shard: current,
        final_root,
        staging_root,
        pinned_directories: pinned,
    })
}

pub(super) fn locate_paths(
    data_dir: &Path,
    digest: &str,
) -> Result<InstallationPaths, ExternalPoolAdapterInstallationFsError> {
    validate_digest(digest)?;
    ensure_directory(data_dir, false, false)?;
    let mut pinned = Vec::new();
    pin_directory(data_dir, &mut pinned)?;
    let mut current = data_dir.to_path_buf();
    for component in INSTALL_COMPONENTS
        .into_iter()
        .chain(std::iter::once(&digest[..2]))
    {
        current.push(component);
        ensure_directory(&current, true, false)?;
        pin_directory(&current, &mut pinned)?;
    }
    Ok(InstallationPaths {
        final_root: current.join(digest),
        staging_root: current.join(format!(".{digest}.audit-only.part")),
        shard: current,
        pinned_directories: pinned,
    })
}

pub(super) fn create_private_directory(
    path: &Path,
) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    std::fs::create_dir(path).map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
    set_private_directory_permissions(path)?;
    require_safe_directory(
        &std::fs::symlink_metadata(path)
            .map_err(ExternalPoolAdapterInstallationFsError::Storage)?,
        true,
    )
}

pub(super) fn ensure_child_directories(
    root: &Path,
    relative_parent: &Path,
) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    let mut current = root.to_path_buf();
    for component in relative_parent.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(ExternalPoolAdapterInstallationFsError::UnsafeTarget);
        };
        current.push(component);
        match std::fs::create_dir(&current) {
            Ok(()) => set_private_directory_permissions(&current)?,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(ExternalPoolAdapterInstallationFsError::Storage(error)),
        }
        require_safe_directory(
            &std::fs::symlink_metadata(&current)
                .map_err(ExternalPoolAdapterInstallationFsError::Storage)?,
            true,
        )?;
    }
    Ok(())
}

fn ensure_directory(
    path: &Path,
    private: bool,
    create_missing: bool,
) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => require_safe_directory(&metadata, private),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && create_missing => {
            match std::fs::create_dir(path) {
                Ok(()) => set_private_directory_permissions(path)?,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(ExternalPoolAdapterInstallationFsError::Storage(error)),
            }
            require_safe_directory(
                &std::fs::symlink_metadata(path)
                    .map_err(ExternalPoolAdapterInstallationFsError::Storage)?,
                private,
            )
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(ExternalPoolAdapterInstallationFsError::Missing)
        }
        Err(error) => Err(ExternalPoolAdapterInstallationFsError::Storage(error)),
    }
}

pub(super) fn require_safe_directory(
    metadata: &Metadata,
    private: bool,
) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    if metadata.file_type().is_symlink() || !metadata.is_dir() || windows_reparse(metadata) {
        return Err(ExternalPoolAdapterInstallationFsError::UnsafeTarget);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let mode = metadata.mode() & 0o777;
        if (private && mode != 0o700) || (!private && mode & 0o022 != 0) {
            return Err(ExternalPoolAdapterInstallationFsError::UnsafeTarget);
        }
    }
    Ok(())
}

pub(super) fn require_safe_regular_file(
    metadata: &Metadata,
) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    if metadata.file_type().is_symlink() || !metadata.is_file() || windows_reparse(metadata) {
        return Err(ExternalPoolAdapterInstallationFsError::UnsafeTarget);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.mode() & 0o077 != 0 || metadata.nlink() != 1 {
            return Err(ExternalPoolAdapterInstallationFsError::UnsafeTarget);
        }
    }
    Ok(())
}

#[cfg(windows)]
fn windows_reparse(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x0000_0400 != 0
}

#[cfg(not(windows))]
fn windows_reparse(_metadata: &Metadata) -> bool {
    false
}

#[cfg(unix)]
fn set_private_directory_permissions(
    path: &Path,
) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .map_err(ExternalPoolAdapterInstallationFsError::Storage)
}

#[cfg(not(unix))]
fn set_private_directory_permissions(
    _path: &Path,
) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    Ok(())
}

#[cfg(windows)]
pub(super) fn pin_directory(
    path: &Path,
    pins: &mut Vec<File>,
) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 1;
    const FILE_SHARE_WRITE: u32 = 2;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    let directory = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
    require_safe_directory(
        &directory
            .metadata()
            .map_err(ExternalPoolAdapterInstallationFsError::Storage)?,
        false,
    )?;
    pins.push(directory);
    Ok(())
}

#[cfg(not(windows))]
pub(super) fn pin_directory(
    _path: &Path,
    _pins: &mut Vec<File>,
) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    Ok(())
}

pub(super) fn open_no_follow(path: &Path) -> Result<File, ExternalPoolAdapterInstallationFsError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ExternalPoolAdapterInstallationFsError::Missing
        } else {
            ExternalPoolAdapterInstallationFsError::Storage(error)
        }
    })?;
    require_safe_regular_file(&metadata)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1).custom_flags(0x0020_0000);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(unix_no_follow_flag());
    }
    let file = options
        .open(path)
        .map_err(ExternalPoolAdapterInstallationFsError::Storage)?;
    require_safe_regular_file(
        &file
            .metadata()
            .map_err(ExternalPoolAdapterInstallationFsError::Storage)?,
    )?;
    require_single_link_handle(&file)?;
    Ok(file)
}

#[cfg(windows)]
pub(super) fn require_single_link_handle(
    file: &File,
) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) } == 0 {
        return Err(ExternalPoolAdapterInstallationFsError::Storage(
            std::io::Error::last_os_error(),
        ));
    }
    if information.nNumberOfLinks != 1 {
        return Err(ExternalPoolAdapterInstallationFsError::UnsafeTarget);
    }
    Ok(())
}

#[cfg(not(windows))]
pub(super) fn require_single_link_handle(
    _file: &File,
) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    Ok(())
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

fn validate_digest(value: &str) -> Result<(), ExternalPoolAdapterInstallationFsError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ExternalPoolAdapterInstallationFsError::InvalidContentAddress);
    }
    Ok(())
}
