use std::{
    ffi::OsStr,
    fs::{self, OpenOptions},
    mem::{size_of, MaybeUninit},
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use windows_sys::Win32::{
    Foundation::HANDLE,
    Storage::FileSystem::{
        GetDriveTypeW, GetFileInformationByHandle, GetVolumeInformationW, GetVolumePathNameW,
        BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TRAVERSE, SYNCHRONIZE,
    },
    System::SystemInformation::OSVERSIONINFOW,
};

use super::child::{
    ChildIdentityFingerprint, RegistrationCommitment, RootCommitment, ValidatedChildProcessReceipt,
};

const REQUIRED_TARGET: &str = "elon-pc-node";
const DRIVE_FIXED: u32 = 3;
const MAX_WINDOWS_PATH_UTF16: usize = 32_768;

#[link(name = "ntdll")]
unsafe extern "system" {
    fn RtlGetVersion(version: *mut OSVERSIONINFOW) -> i32;
}

pub(super) struct CapturedRootBinding {
    pub(super) canonical_root: PathBuf,
    pub(super) commitment: RootCommitment,
}

/// Platform tuple and parent-recomputed binding for one exited child and its exact root.
pub(in super::super) struct WindowsDynamicEnvironment {
    pub(super) git_sha: String,
    pub(super) target: &'static str,
    pub(super) windows_build: String,
    pub(super) architecture: &'static str,
    pub(super) volume_kind: &'static str,
    pub(super) filesystem: String,
    pub(super) bundled_sqlite: String,
    pub(super) child_fingerprint: ChildIdentityFingerprint,
    pub(super) registration_commitment: RegistrationCommitment,
    pub(super) canonical_root: PathBuf,
    pub(super) root_commitment: RootCommitment,
}

impl WindowsDynamicEnvironment {
    /// Recomputes the root commitment after the exact child has exited and before parent cleanup.
    pub(in super::super) fn capture(
        root: &Path,
        child: &ValidatedChildProcessReceipt,
    ) -> Result<Self, &'static str> {
        let captured_root =
            capture_root_binding(root, child.identity.process_id, &child.identity.nonce)?;
        if captured_root.commitment != child.root_commitment {
            return Err("A2_DYNAMIC_ROOT_COMMITMENT_MISMATCH");
        }
        let git_sha = validate_git_sha(
            option_env!("ELON_NODE_AGENT_GIT_SHA").ok_or("A2_DYNAMIC_GIT_SHA_MISSING")?,
        )?
        .to_owned();
        let target = validate_target(
            option_env!("CARGO_BIN_NAME").ok_or("A2_DYNAMIC_CARGO_TARGET_MISSING")?,
        )?;
        let windows_build = capture_windows_build()?;
        let architecture = validate_architecture(std::env::consts::ARCH)?;
        let (volume_kind, filesystem) = capture_volume(&captured_root.canonical_root)?;
        let bundled_sqlite = capture_bundled_sqlite()?;
        Ok(Self {
            git_sha,
            target,
            windows_build,
            architecture,
            volume_kind,
            filesystem,
            bundled_sqlite,
            child_fingerprint: child.fingerprint(),
            registration_commitment: RegistrationCommitment(child.registration_commitment.0),
            canonical_root: captured_root.canonical_root,
            root_commitment: captured_root.commitment,
        })
    }

    pub(super) fn root_for_cleanup(&self) -> &Path {
        &self.canonical_root
    }
}

pub(super) fn capture_root_binding(
    root: &Path,
    process_id: u32,
    nonce: &str,
) -> Result<CapturedRootBinding, &'static str> {
    if !root.is_absolute() || process_id == 0 {
        return Err("A2_DYNAMIC_ROOT_IDENTITY_INVALID");
    }
    let original_metadata =
        fs::symlink_metadata(root).map_err(|_| "A2_DYNAMIC_ROOT_NOT_OBSERVABLE")?;
    validate_plain_directory(&original_metadata)?;
    let canonical_root =
        fs::canonicalize(root).map_err(|_| "A2_DYNAMIC_ROOT_CANONICALIZE_FAILED")?;
    if !canonical_root.is_absolute() {
        return Err("A2_DYNAMIC_ROOT_CANONICAL_INVALID");
    }
    let canonical_metadata = fs::symlink_metadata(&canonical_root)
        .map_err(|_| "A2_DYNAMIC_CANONICAL_ROOT_NOT_OBSERVABLE")?;
    validate_plain_directory(&canonical_metadata)?;
    if original_metadata.creation_time() != canonical_metadata.creation_time()
        || original_metadata.file_attributes() != canonical_metadata.file_attributes()
    {
        return Err("A2_DYNAMIC_ROOT_CHANGED_DURING_CAPTURE");
    }

    let information = open_root_information(&canonical_root)?;
    let creation_time = filetime_to_u64(information.ftCreationTime);
    if information.dwFileAttributes != canonical_metadata.file_attributes()
        || creation_time != canonical_metadata.creation_time()
        || information.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || information.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
    {
        return Err("A2_DYNAMIC_ROOT_HANDLE_METADATA_MISMATCH");
    }
    let path_units = canonical_root.as_os_str().encode_wide().collect::<Vec<_>>();
    if path_units.is_empty()
        || path_units.contains(&0)
        || path_units.len() >= MAX_WINDOWS_PATH_UTF16
    {
        return Err("A2_DYNAMIC_WINDOWS_PATH_INVALID");
    }
    let file_index =
        (u64::from(information.nFileIndexHigh) << 32) | u64::from(information.nFileIndexLow);
    let commitment = bind_root(
        &path_units,
        process_id,
        nonce,
        information.dwVolumeSerialNumber,
        file_index,
        creation_time,
        information.dwFileAttributes,
    );
    Ok(CapturedRootBinding {
        canonical_root,
        commitment,
    })
}

fn validate_plain_directory(metadata: &fs::Metadata) -> Result<(), &'static str> {
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    {
        return Err("A2_DYNAMIC_ROOT_NOT_PLAIN_DIRECTORY");
    }
    Ok(())
}

fn open_root_information(root: &Path) -> Result<BY_HANDLE_FILE_INFORMATION, &'static str> {
    let handle = OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES | FILE_TRAVERSE | SYNCHRONIZE)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(root)
        .map_err(|_| "A2_DYNAMIC_ROOT_HANDLE_OPEN_FAILED")?;
    let mut information = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    // SAFETY: `handle` stays live and `information` is writable for the synchronous call.
    if unsafe {
        GetFileInformationByHandle(handle.as_raw_handle() as HANDLE, information.as_mut_ptr())
    } == 0
    {
        return Err("A2_DYNAMIC_ROOT_HANDLE_INFORMATION_FAILED");
    }
    // SAFETY: successful GetFileInformationByHandle initialized the complete structure.
    Ok(unsafe { information.assume_init() })
}

fn filetime_to_u64(value: windows_sys::Win32::Foundation::FILETIME) -> u64 {
    (u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime)
}

fn bind_root(
    path_units: &[u16],
    process_id: u32,
    nonce: &str,
    volume_serial: u32,
    file_index: u64,
    creation_time: u64,
    attributes: u32,
) -> RootCommitment {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-a2-windows-dynamic-root-v2\0");
    hasher.update(process_id.to_le_bytes());
    hasher.update((nonce.len() as u64).to_le_bytes());
    hasher.update(nonce.as_bytes());
    hasher.update((path_units.len() as u64).to_le_bytes());
    for unit in path_units {
        hasher.update(unit.to_le_bytes());
    }
    hasher.update(volume_serial.to_le_bytes());
    hasher.update(file_index.to_le_bytes());
    hasher.update(creation_time.to_le_bytes());
    hasher.update(attributes.to_le_bytes());
    RootCommitment(hasher.finalize().into())
}

fn capture_windows_build() -> Result<String, &'static str> {
    let mut version = OSVERSIONINFOW {
        dwOSVersionInfoSize: size_of::<OSVERSIONINFOW>() as u32,
        dwMajorVersion: 0,
        dwMinorVersion: 0,
        dwBuildNumber: 0,
        dwPlatformId: 0,
        szCSDVersion: [0; 128],
    };
    // SAFETY: `version` has the documented size and remains writable for the synchronous call.
    let status = unsafe { RtlGetVersion(&mut version) };
    if status < 0 || version.dwMajorVersion < 10 || version.dwBuildNumber < 10_240 {
        return Err("A2_DYNAMIC_WINDOWS_BUILD_UNSUPPORTED");
    }
    Ok(format!(
        "{}.{}.{}",
        version.dwMajorVersion, version.dwMinorVersion, version.dwBuildNumber
    ))
}

fn capture_volume(root: &Path) -> Result<(&'static str, String), &'static str> {
    let root_utf16 = nul_terminated(root.as_os_str())?;
    let mut volume_path = vec![0u16; MAX_WINDOWS_PATH_UTF16];
    // SAFETY: both UTF-16 buffers are NUL-terminated/writable for their advertised lengths.
    if unsafe {
        GetVolumePathNameW(
            root_utf16.as_ptr(),
            volume_path.as_mut_ptr(),
            volume_path.len() as u32,
        )
    } == 0
    {
        return Err("A2_DYNAMIC_VOLUME_PATH_UNAVAILABLE");
    }
    let volume_length = volume_path
        .iter()
        .position(|unit| *unit == 0)
        .ok_or("A2_DYNAMIC_VOLUME_PATH_INVALID")?;
    volume_path.truncate(volume_length + 1);
    // SAFETY: `volume_path` is the NUL-terminated root returned by GetVolumePathNameW.
    if unsafe { GetDriveTypeW(volume_path.as_ptr()) } != DRIVE_FIXED {
        return Err("A2_DYNAMIC_VOLUME_TYPE_UNSUPPORTED");
    }

    let mut filesystem = [0u16; 64];
    // SAFETY: the returned volume path remains live and the filesystem buffer is writable.
    if unsafe {
        GetVolumeInformationW(
            volume_path.as_ptr(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            filesystem.as_mut_ptr(),
            filesystem.len() as u32,
        )
    } == 0
    {
        return Err("A2_DYNAMIC_VOLUME_INFORMATION_UNAVAILABLE");
    }
    let filesystem_length = filesystem
        .iter()
        .position(|unit| *unit == 0)
        .ok_or("A2_DYNAMIC_FILESYSTEM_INVALID")?;
    let filesystem = String::from_utf16(&filesystem[..filesystem_length])
        .map_err(|_| "A2_DYNAMIC_FILESYSTEM_INVALID")?
        .to_ascii_lowercase();
    if !matches!(filesystem.as_str(), "ntfs" | "refs") {
        return Err("A2_DYNAMIC_FILESYSTEM_UNSUPPORTED");
    }
    Ok(("fixed", filesystem))
}

fn capture_bundled_sqlite() -> Result<String, &'static str> {
    let version = rusqlite::version();
    validate_sqlite_version(version, rusqlite::version_number())?;
    Ok(version.to_owned())
}

fn nul_terminated(value: &OsStr) -> Result<Vec<u16>, &'static str> {
    let mut encoded = value.encode_wide().collect::<Vec<_>>();
    if encoded.is_empty() || encoded.contains(&0) || encoded.len() >= MAX_WINDOWS_PATH_UTF16 {
        return Err("A2_DYNAMIC_WINDOWS_PATH_INVALID");
    }
    encoded.push(0);
    Ok(encoded)
}

pub(super) fn validate_git_sha(value: &str) -> Result<&str, &'static str> {
    let value = value.trim();
    if !matches!(value.len(), 40 | 64)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("A2_DYNAMIC_GIT_SHA_INVALID");
    }
    Ok(value)
}

fn validate_target(value: &'static str) -> Result<&'static str, &'static str> {
    if value != REQUIRED_TARGET {
        return Err("A2_DYNAMIC_CARGO_TARGET_INVALID");
    }
    Ok(REQUIRED_TARGET)
}

fn validate_architecture(value: &'static str) -> Result<&'static str, &'static str> {
    if !matches!(value, "x86" | "x86_64" | "aarch64") {
        return Err("A2_DYNAMIC_ARCHITECTURE_UNSUPPORTED");
    }
    Ok(value)
}

pub(super) fn validate_sqlite_version(
    version: &str,
    version_number: i32,
) -> Result<(), &'static str> {
    let components = version
        .split('.')
        .map(str::parse::<i32>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "A2_DYNAMIC_SQLITE_VERSION_INVALID")?;
    let [major, minor, patch] = components.as_slice() else {
        return Err("A2_DYNAMIC_SQLITE_VERSION_INVALID");
    };
    let encoded = major
        .checked_mul(1_000_000)
        .and_then(|value| {
            minor
                .checked_mul(1_000)
                .and_then(|minor| value.checked_add(minor))
        })
        .and_then(|value| value.checked_add(*patch))
        .ok_or("A2_DYNAMIC_SQLITE_VERSION_INVALID")?;
    if *major < 3 || *minor < 0 || *patch < 0 || encoded != version_number {
        return Err("A2_DYNAMIC_SQLITE_VERSION_MISMATCH");
    }
    Ok(())
}
