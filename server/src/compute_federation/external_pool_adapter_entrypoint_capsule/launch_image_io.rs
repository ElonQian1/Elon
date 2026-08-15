use std::{
    ffi::CString,
    fs::File,
    io::ErrorKind,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{FileExt, MetadataExt},
    },
};

use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use super::types::ExternalPoolAdapterEntrypointCapsuleError;

pub(super) const MAX_IMAGE_BYTES: u64 = 268_435_456;
const REQUIRED_SEALS: libc::c_int =
    libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) struct LaunchIdentity {
    device: u64,
    inode: u64,
    length: u64,
    mode: u32,
    links: u64,
}

pub(super) fn create_launch_memfd() -> Result<File, ExternalPoolAdapterEntrypointCapsuleError> {
    let name = CString::new("elon-external-pool-launch").expect("static memfd name");
    let fd =
        unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING) };
    if fd < 0 {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::Unavailable);
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

pub(super) fn set_length(
    file: &File,
    length: u64,
) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    if length > MAX_IMAGE_BYTES
        || unsafe { libc::ftruncate(file.as_raw_fd(), length as libc::off_t) } != 0
    {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::MaterializationFailed);
    }
    Ok(())
}

pub(super) fn seal_launch(file: &File) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    if unsafe { libc::fchmod(file.as_raw_fd(), 0o500) } != 0
        || unsafe { libc::fcntl(file.as_raw_fd(), libc::F_ADD_SEALS, REQUIRED_SEALS) } != 0
    {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::MaterializationFailed);
    }
    Ok(())
}

pub(super) fn require_source_custody(
    file: &File,
    length: u64,
) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    let item = identity(file)?;
    if item.length != length
        || item.mode & 0o777 != 0o500
        || item.links != 0
        || unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GET_SEALS) } != REQUIRED_SEALS
    {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::ContentDrift);
    }
    Ok(())
}

pub(super) fn require_launch_custody(
    file: &File,
    length: u64,
) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    let item = identity(file)?;
    let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
    if item.length != length
        || item.mode & 0o777 != 0o500
        || item.links != 0
        || flags < 0
        || flags & libc::FD_CLOEXEC == 0
        || unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GET_SEALS) } != REQUIRED_SEALS
    {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::MaterializationFailed);
    }
    Ok(())
}

pub(super) fn identity(
    file: &File,
) -> Result<LaunchIdentity, ExternalPoolAdapterEntrypointCapsuleError> {
    let metadata = file.metadata().map_err(drift)?;
    if !metadata.is_file() || metadata.uid() != unsafe { libc::geteuid() } {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::ContentDrift);
    }
    Ok(LaunchIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        length: metadata.len(),
        mode: metadata.mode(),
        links: metadata.nlink(),
    })
}

pub(super) fn hash_exact(
    file: &File,
    length: u64,
) -> Result<[u8; 32], ExternalPoolAdapterEntrypointCapsuleError> {
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut offset = 0_u64;
    while offset < length {
        let count = usize::try_from((length - offset).min(buffer.len() as u64))
            .map_err(|_| ExternalPoolAdapterEntrypointCapsuleError::ContentDrift)?;
        read_exact_at(file, &mut buffer[..count], offset)?;
        digest.update(&buffer[..count]);
        offset += count as u64;
    }
    buffer.zeroize();
    Ok(digest.finalize().into())
}

pub(super) fn copy_range(
    source: &File,
    target: &File,
    mut source_offset: u64,
    mut target_offset: u64,
    mut length: u64,
) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    let mut buffer = [0_u8; 64 * 1024];
    while length > 0 {
        let count = usize::try_from(length.min(buffer.len() as u64))
            .map_err(|_| ExternalPoolAdapterEntrypointCapsuleError::MaterializationFailed)?;
        read_exact_at(source, &mut buffer[..count], source_offset)?;
        write_exact_at(target, &buffer[..count], target_offset)?;
        source_offset += count as u64;
        target_offset += count as u64;
        length -= count as u64;
    }
    buffer.zeroize();
    Ok(())
}

pub(super) fn read_exact_at(
    file: &File,
    mut output: &mut [u8],
    mut offset: u64,
) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    while !output.is_empty() {
        match file.read_at(output, offset) {
            Ok(0) => return Err(ExternalPoolAdapterEntrypointCapsuleError::ContentDrift),
            Ok(read) => {
                output = &mut output[read..];
                offset += read as u64;
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(_) => return Err(ExternalPoolAdapterEntrypointCapsuleError::ContentDrift),
        }
    }
    Ok(())
}

pub(super) fn write_exact_at(
    file: &File,
    mut input: &[u8],
    mut offset: u64,
) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    while !input.is_empty() {
        match file.write_at(input, offset) {
            Ok(0) => return Err(ExternalPoolAdapterEntrypointCapsuleError::MaterializationFailed),
            Ok(written) => {
                input = &input[written..];
                offset += written as u64;
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(_) => return Err(ExternalPoolAdapterEntrypointCapsuleError::MaterializationFailed),
        }
    }
    Ok(())
}

fn drift(_: std::io::Error) -> ExternalPoolAdapterEntrypointCapsuleError {
    ExternalPoolAdapterEntrypointCapsuleError::ContentDrift
}
