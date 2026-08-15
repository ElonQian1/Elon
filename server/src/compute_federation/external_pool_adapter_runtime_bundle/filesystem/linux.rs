use std::{
    ffi::CString,
    fs::File,
    os::fd::{AsRawFd, FromRawFd},
    os::unix::ffi::OsStrExt,
    os::unix::fs::MetadataExt,
    path::{Component, Path},
};

use super::{
    ExternalPoolAdapterRuntimeBundleError, LockedSensitiveBytes, OpenedRuntimeBundle, CONFIG_FILE,
    CREDENTIAL_FILE, MANIFEST_FILE,
};

const DIRECTORY_MODE: u32 = 0o500;
const FILE_MODE: u32 = 0o400;

unsafe extern "C" {
    fn __errno_location() -> *mut libc::c_int;
}

pub(super) struct LinuxOpenedRuntimeBundle {
    directories: Vec<File>,
    custody_root_index: usize,
    manifest: File,
    config: File,
    credential: File,
    identities: Vec<FileIdentity>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct FileIdentity {
    device: u64,
    inode: u64,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
}

impl LinuxOpenedRuntimeBundle {
    pub(super) fn open(
        root: &File,
        digest: &str,
    ) -> Result<Self, ExternalPoolAdapterRuntimeBundleError> {
        let retained_root = duplicate_cloexec(root)?;
        validate_directory(&retained_root)?;
        require_local_filesystem(&retained_root)?;
        let mut directories = vec![retained_root];
        let custody_root_index = 0;
        let root_fd = directories
            .last()
            .ok_or(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)?;
        let v1 = open_directory_at(&root_fd, "v1")?;
        let sha256 = open_directory_at(&v1, "sha256")?;
        let shard = open_directory_at(&sha256, &digest[..2])?;
        let bundle = open_directory_at(&shard, digest)?;
        require_exact_entries(&bundle, &[MANIFEST_FILE, CONFIG_FILE, CREDENTIAL_FILE])?;

        let manifest = open_file_at(&bundle, MANIFEST_FILE)?;
        let config = open_file_at(&bundle, CONFIG_FILE)?;
        let credential = open_file_at(&bundle, CREDENTIAL_FILE)?;
        directories.extend([v1, sha256, shard, bundle]);
        let identities = directories
            .iter()
            .chain([&manifest, &config, &credential])
            .map(identity)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            directories,
            custody_root_index,
            manifest,
            config,
            credential,
            identities,
        })
    }
}

pub(super) fn open_custody_root(
    path: &Path,
) -> Result<File, ExternalPoolAdapterRuntimeBundleError> {
    let mut directories = open_absolute_root(path)?;
    directories
        .pop()
        .ok_or(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)
}

impl OpenedRuntimeBundle for LinuxOpenedRuntimeBundle {
    fn read_manifest(
        &mut self,
        max_bytes: u64,
    ) -> Result<LockedSensitiveBytes, ExternalPoolAdapterRuntimeBundleError> {
        let length = self
            .manifest
            .metadata()
            .map_err(|_| ExternalPoolAdapterRuntimeBundleError::Unavailable)?
            .len();
        if length == 0 || length > max_bytes {
            return Err(ExternalPoolAdapterRuntimeBundleError::InvalidAuthority);
        }
        read_from_start(&mut self.manifest, length)
    }

    fn read_sensitive(
        &mut self,
        name: &'static str,
        expected_size: u64,
    ) -> Result<LockedSensitiveBytes, ExternalPoolAdapterRuntimeBundleError> {
        let file = match name {
            CONFIG_FILE => &mut self.config,
            CREDENTIAL_FILE => &mut self.credential,
            _ => return Err(ExternalPoolAdapterRuntimeBundleError::InvalidAuthority),
        };
        if file
            .metadata()
            .map_err(|_| ExternalPoolAdapterRuntimeBundleError::Unavailable)?
            .len()
            != expected_size
        {
            return Err(ExternalPoolAdapterRuntimeBundleError::ContentDrift);
        }
        read_from_start(file, expected_size)
    }

    fn revalidate(&self) -> Result<(), ExternalPoolAdapterRuntimeBundleError> {
        for directory in self.directories.iter().skip(self.custody_root_index) {
            validate_directory(directory)?;
            require_local_filesystem(directory)?;
        }
        for file in [&self.manifest, &self.config, &self.credential] {
            validate_regular_file(file)?;
            require_local_filesystem(file)?;
        }
        let handles =
            self.directories
                .iter()
                .chain([&self.manifest, &self.config, &self.credential]);
        for (handle, expected) in handles.zip(&self.identities) {
            if identity(handle)? != *expected {
                return Err(ExternalPoolAdapterRuntimeBundleError::ContentDrift);
            }
        }
        require_exact_entries(
            self.directories
                .last()
                .ok_or(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)?,
            &[MANIFEST_FILE, CONFIG_FILE, CREDENTIAL_FILE],
        )
    }
}

fn open_absolute_root(path: &Path) -> Result<Vec<File>, ExternalPoolAdapterRuntimeBundleError> {
    if !path.is_absolute() || path.as_os_str().as_bytes().contains(&0) {
        return Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody);
    }
    let slash =
        CString::new("/").map_err(|_| ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)?;
    let root = file_from_fd(unsafe {
        libc::open(
            slash.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    })?;
    let mut directories = vec![root];
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                let parent = directories
                    .last()
                    .ok_or(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)?;
                let name = CString::new(name.as_bytes())
                    .map_err(|_| ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)?;
                let next = file_from_fd(unsafe {
                    libc::openat(
                        parent.as_raw_fd(),
                        name.as_ptr(),
                        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                    )
                })?;
                directories.push(next);
            }
            _ => return Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody),
        }
    }
    let custody_root = directories
        .last()
        .ok_or(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)?;
    validate_directory(custody_root)?;
    require_local_filesystem(custody_root)?;
    Ok(directories)
}

fn open_directory_at(
    parent: &File,
    name: &str,
) -> Result<File, ExternalPoolAdapterRuntimeBundleError> {
    let name = safe_component(name)?;
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    let file = file_from_fd(fd)?;
    validate_directory(&file)?;
    require_local_filesystem(&file)?;
    Ok(file)
}

fn open_file_at(parent: &File, name: &str) -> Result<File, ExternalPoolAdapterRuntimeBundleError> {
    let name = safe_component(name)?;
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    let file = file_from_fd(fd)?;
    validate_regular_file(&file)?;
    require_local_filesystem(&file)?;
    Ok(file)
}

fn safe_component(value: &str) -> Result<CString, ExternalPoolAdapterRuntimeBundleError> {
    if value.is_empty() || value == "." || value == ".." || value.contains(['/', '\\']) {
        return Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody);
    }
    CString::new(value).map_err(|_| ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)
}

fn file_from_fd(fd: i32) -> Result<File, ExternalPoolAdapterRuntimeBundleError> {
    if fd < 0 {
        Err(ExternalPoolAdapterRuntimeBundleError::Unavailable)
    } else {
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}

fn duplicate_cloexec(file: &File) -> Result<File, ExternalPoolAdapterRuntimeBundleError> {
    let duplicate = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    file_from_fd(duplicate)
}

fn validate_directory(file: &File) -> Result<(), ExternalPoolAdapterRuntimeBundleError> {
    let metadata = file
        .metadata()
        .map_err(|_| ExternalPoolAdapterRuntimeBundleError::Unavailable)?;
    let current_uid = unsafe { libc::geteuid() };
    if !metadata.is_dir()
        || metadata.uid() != current_uid
        || metadata.mode() & 0o7777 != DIRECTORY_MODE
    {
        return Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody);
    }
    reject_extended_attributes(file)?;
    Ok(())
}

fn validate_regular_file(file: &File) -> Result<(), ExternalPoolAdapterRuntimeBundleError> {
    let metadata = file
        .metadata()
        .map_err(|_| ExternalPoolAdapterRuntimeBundleError::Unavailable)?;
    let current_uid = unsafe { libc::geteuid() };
    if !metadata.is_file()
        || metadata.uid() != current_uid
        || metadata.mode() & 0o7777 != FILE_MODE
        || metadata.nlink() != 1
    {
        return Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody);
    }
    reject_extended_attributes(file)?;
    Ok(())
}

fn reject_extended_attributes(file: &File) -> Result<(), ExternalPoolAdapterRuntimeBundleError> {
    // A mode-only check cannot prove that a POSIX ACL does not grant another principal access.
    // This operator custody namespace has no xattr use, so reject every xattr rather than maintain
    // an allow-list that could accidentally admit a new security-relevant namespace.
    let size = unsafe { libc::flistxattr(file.as_raw_fd(), std::ptr::null_mut(), 0) };
    if size < 0 {
        return Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody);
    }
    if size != 0 {
        return Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody);
    }
    Ok(())
}

fn require_local_filesystem(file: &File) -> Result<(), ExternalPoolAdapterRuntimeBundleError> {
    const EXT_SUPER_MAGIC: i64 = 0xef53;
    const XFS_SUPER_MAGIC: i64 = 0x5846_5342;
    const BTRFS_SUPER_MAGIC: i64 = 0x9123_683e;
    const TMPFS_SUPER_MAGIC: i64 = 0x0102_1994;
    let mut status = std::mem::MaybeUninit::<libc::statfs>::uninit();
    if unsafe { libc::fstatfs(file.as_raw_fd(), status.as_mut_ptr()) } != 0 {
        return Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody);
    }
    let kind = unsafe { status.assume_init() }.f_type as i64;
    // An unknown filesystem is not evidence of locality. Support only audited local custody
    // families; network, userspace, overlay, and future filesystem kinds remain fail-closed.
    match kind {
        EXT_SUPER_MAGIC | XFS_SUPER_MAGIC | BTRFS_SUPER_MAGIC | TMPFS_SUPER_MAGIC => Ok(()),
        _ => Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody),
    }
}

fn identity(file: &File) -> Result<FileIdentity, ExternalPoolAdapterRuntimeBundleError> {
    let metadata = file
        .metadata()
        .map_err(|_| ExternalPoolAdapterRuntimeBundleError::Unavailable)?;
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        length: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
    })
}

fn read_from_start(
    file: &mut File,
    size: u64,
) -> Result<LockedSensitiveBytes, ExternalPoolAdapterRuntimeBundleError> {
    use std::io::{Seek, SeekFrom};
    file.seek(SeekFrom::Start(0))
        .map_err(|_| ExternalPoolAdapterRuntimeBundleError::ContentDrift)?;
    LockedSensitiveBytes::read_exact(file, size)
}

fn require_exact_entries(
    directory: &File,
    expected: &[&str],
) -> Result<(), ExternalPoolAdapterRuntimeBundleError> {
    let duplicate = unsafe { libc::fcntl(directory.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if duplicate < 0 {
        return Err(ExternalPoolAdapterRuntimeBundleError::Unavailable);
    }
    if unsafe { libc::lseek(duplicate, 0, libc::SEEK_SET) } < 0 {
        unsafe { libc::close(duplicate) };
        return Err(ExternalPoolAdapterRuntimeBundleError::Unavailable);
    }
    let stream = unsafe { libc::fdopendir(duplicate) };
    if stream.is_null() {
        unsafe { libc::close(duplicate) };
        return Err(ExternalPoolAdapterRuntimeBundleError::Unavailable);
    }
    let mut observed = Vec::new();
    loop {
        unsafe { *__errno_location() = 0 };
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            if unsafe { *__errno_location() } != 0 {
                unsafe { libc::closedir(stream) };
                return Err(ExternalPoolAdapterRuntimeBundleError::Unavailable);
            }
            break;
        }
        let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) };
        if name.to_bytes() != b"." && name.to_bytes() != b".." {
            observed.push(name.to_bytes().to_vec());
        }
    }
    unsafe { libc::closedir(stream) };
    observed.sort();
    let mut expected = expected
        .iter()
        .map(|name| name.as_bytes().to_vec())
        .collect::<Vec<_>>();
    expected.sort();
    if observed == expected {
        Ok(())
    } else {
        Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)
    }
}
