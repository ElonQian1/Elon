use std::{
    fs::{File, OpenOptions},
    io::{Seek, SeekFrom},
    mem::{size_of, MaybeUninit},
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle},
    path::{Component, Path, PathBuf, Prefix},
};

use windows_sys::Win32::{
    Foundation::{GetHandleInformation, HANDLE, HANDLE_FLAG_INHERIT},
    Storage::FileSystem::{
        FileIdInfo, GetDriveTypeW, GetFileInformationByHandle, GetFileInformationByHandleEx,
        BY_HANDLE_FILE_INFORMATION, DRIVE_FIXED, FILE_ATTRIBUTE_DIRECTORY,
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_GENERIC_READ, FILE_ID_INFO, FILE_SHARE_READ,
    },
};

use super::{
    ExternalPoolAdapterRuntimeBundleError, LockedSensitiveBytes, OpenedRuntimeBundle, CONFIG_FILE,
    CREDENTIAL_FILE, MANIFEST_FILE,
};

pub(super) struct WindowsOpenedRuntimeBundle {
    directories: Vec<File>,
    manifest: File,
    config: File,
    credential: File,
    bundle_path: PathBuf,
    identities: Vec<FileIdentity>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct FileIdentity {
    volume_serial: u64,
    file_id: [u8; 16],
    links: u32,
    length: u64,
    last_write_time: u64,
    directory: bool,
}

impl WindowsOpenedRuntimeBundle {
    pub(super) fn open(
        root: &Path,
        digest: &str,
    ) -> Result<Self, ExternalPoolAdapterRuntimeBundleError> {
        validate_local_absolute_root(root)?;
        let root_handle = open_directory(root)?;
        let v1_path = root.join("v1");
        let v1 = open_directory(&v1_path)?;
        let sha256_path = v1_path.join("sha256");
        let sha256 = open_directory(&sha256_path)?;
        let shard_path = sha256_path.join(&digest[..2]);
        let shard = open_directory(&shard_path)?;
        let bundle_path = shard_path.join(digest);
        let bundle = open_directory(&bundle_path)?;
        require_exact_entries(&bundle_path)?;

        let manifest = open_regular_file(&bundle_path.join(MANIFEST_FILE))?;
        let config = open_regular_file(&bundle_path.join(CONFIG_FILE))?;
        let credential = open_regular_file(&bundle_path.join(CREDENTIAL_FILE))?;
        let directories = vec![root_handle, v1, sha256, shard, bundle];
        let identities = directories
            .iter()
            .chain([&manifest, &config, &credential])
            .map(identity)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            directories,
            manifest,
            config,
            credential,
            bundle_path,
            identities,
        })
    }
}

impl OpenedRuntimeBundle for WindowsOpenedRuntimeBundle {
    fn read_manifest(
        &mut self,
        max_bytes: u64,
    ) -> Result<LockedSensitiveBytes, ExternalPoolAdapterRuntimeBundleError> {
        let length = identity(&self.manifest)?.length;
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
        if identity(file)?.length != expected_size {
            return Err(ExternalPoolAdapterRuntimeBundleError::ContentDrift);
        }
        read_from_start(file, expected_size)
    }

    fn revalidate(&self) -> Result<(), ExternalPoolAdapterRuntimeBundleError> {
        for (handle, expected) in self
            .directories
            .iter()
            .chain([&self.manifest, &self.config, &self.credential])
            .zip(&self.identities)
        {
            if identity(handle)? != *expected {
                return Err(ExternalPoolAdapterRuntimeBundleError::ContentDrift);
            }
        }
        require_exact_entries(&self.bundle_path)
    }
}

fn validate_local_absolute_root(path: &Path) -> Result<(), ExternalPoolAdapterRuntimeBundleError> {
    let mut components = path.components();
    let drive = match components.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(letter) => letter,
            _ => return Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody),
        },
        _ => return Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody),
    };
    if !matches!(components.next(), Some(Component::RootDir)) {
        return Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody);
    }
    let drive_root = format!("{}:\\", char::from(drive));
    let mut wide = std::ffi::OsStr::new(&drive_root)
        .encode_wide()
        .collect::<Vec<_>>();
    wide.push(0);
    if unsafe { GetDriveTypeW(wide.as_ptr()) } != DRIVE_FIXED {
        return Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody);
    }
    Ok(())
}

fn open_directory(path: &Path) -> Result<File, ExternalPoolAdapterRuntimeBundleError> {
    let file = open_readonly(
        path,
        FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
    )?;
    validate_handle(&file, true)?;
    Ok(file)
}

fn open_regular_file(path: &Path) -> Result<File, ExternalPoolAdapterRuntimeBundleError> {
    let file = open_readonly(path, FILE_FLAG_OPEN_REPARSE_POINT)?;
    validate_handle(&file, false)?;
    Ok(file)
}

fn open_readonly(path: &Path, flags: u32) -> Result<File, ExternalPoolAdapterRuntimeBundleError> {
    OpenOptions::new()
        .access_mode(FILE_GENERIC_READ)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(flags)
        .open(path)
        .map_err(|_| ExternalPoolAdapterRuntimeBundleError::Unavailable)
}

fn validate_handle(
    file: &File,
    expected_directory: bool,
) -> Result<(), ExternalPoolAdapterRuntimeBundleError> {
    let observed = identity(file)?;
    if observed.directory != expected_directory
        || (!expected_directory && observed.links != 1)
        || handle_is_inheritable(file)?
    {
        return Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody);
    }
    // Until the protected-DACL parser accepts only the service identity and LocalSystem, Windows
    // custody remains deliberately unavailable instead of silently weakening this boundary.
    validate_protected_dacl(file)
}

fn handle_is_inheritable(file: &File) -> Result<bool, ExternalPoolAdapterRuntimeBundleError> {
    let mut flags = 0_u32;
    if unsafe { GetHandleInformation(file.as_raw_handle() as HANDLE, &mut flags) } == 0 {
        return Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody);
    }
    Ok(flags & HANDLE_FLAG_INHERIT != 0)
}

fn validate_protected_dacl(_file: &File) -> Result<(), ExternalPoolAdapterRuntimeBundleError> {
    Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)
}

fn identity(file: &File) -> Result<FileIdentity, ExternalPoolAdapterRuntimeBundleError> {
    let handle = file.as_raw_handle() as HANDLE;
    let mut basic = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    if unsafe { GetFileInformationByHandle(handle, basic.as_mut_ptr()) } == 0 {
        return Err(ExternalPoolAdapterRuntimeBundleError::Unavailable);
    }
    let basic = unsafe { basic.assume_init() };
    if basic.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody);
    }
    let mut extended = MaybeUninit::<FILE_ID_INFO>::uninit();
    if unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            extended.as_mut_ptr().cast(),
            size_of::<FILE_ID_INFO>() as u32,
        )
    } == 0
    {
        return Err(ExternalPoolAdapterRuntimeBundleError::Unavailable);
    }
    let extended = unsafe { extended.assume_init() };
    Ok(FileIdentity {
        volume_serial: extended.VolumeSerialNumber,
        file_id: extended.FileId.Identifier,
        links: basic.nNumberOfLinks,
        length: (u64::from(basic.nFileSizeHigh) << 32) | u64::from(basic.nFileSizeLow),
        last_write_time: (u64::from(basic.ftLastWriteTime.dwHighDateTime) << 32)
            | u64::from(basic.ftLastWriteTime.dwLowDateTime),
        directory: basic.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0,
    })
}

fn read_from_start(
    file: &mut File,
    size: u64,
) -> Result<LockedSensitiveBytes, ExternalPoolAdapterRuntimeBundleError> {
    file.seek(SeekFrom::Start(0))
        .map_err(|_| ExternalPoolAdapterRuntimeBundleError::ContentDrift)?;
    LockedSensitiveBytes::read_exact(file, size)
}

fn require_exact_entries(path: &Path) -> Result<(), ExternalPoolAdapterRuntimeBundleError> {
    let mut observed = std::fs::read_dir(path)
        .map_err(|_| ExternalPoolAdapterRuntimeBundleError::Unavailable)?
        .map(|entry| {
            entry
                .map_err(|_| ExternalPoolAdapterRuntimeBundleError::Unavailable)
                .and_then(|entry| {
                    entry
                        .file_name()
                        .into_string()
                        .map_err(|_| ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    observed.sort();
    let mut expected = [MANIFEST_FILE, CONFIG_FILE, CREDENTIAL_FILE];
    expected.sort();
    if observed == expected {
        Ok(())
    } else {
        Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)
    }
}
