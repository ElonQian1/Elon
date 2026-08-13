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

use super::{
    elf::validate_static_elf64_x86_64,
    policy::entrypoint_capsule_policy,
    types::{
        ExternalPoolAdapterEntrypointCapsuleError, PreparedExternalPoolAdapterEntrypointCapsule,
    },
    ExternalPoolAdapterEntrypointSource,
};

const COPY_BUFFER_BYTES: usize = 64 * 1024;
const SOURCE_MODE: u32 = 0o600;
const CAPSULE_MODE: u32 = 0o500;
const REQUIRED_SEALS: libc::c_int =
    libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;

#[derive(Clone, Copy, Eq, PartialEq)]
struct FileIdentity {
    device: u64,
    inode: u64,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    mode: u32,
    owner: u32,
    group: u32,
    links: u64,
}

pub(super) fn materialize(
    source: &impl ExternalPoolAdapterEntrypointSource,
) -> Result<PreparedExternalPoolAdapterEntrypointCapsule, ExternalPoolAdapterEntrypointCapsuleError>
{
    let (source_file, expected_sha256, expected_size) = source
        .retained_entrypoint()
        .map_err(|_| ExternalPoolAdapterEntrypointCapsuleError::InvalidAuthority)?;
    let mut expected_digest = [0_u8; 32];
    if expected_size == 0
        || expected_sha256.len() != 64
        || !expected_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        || hex::decode_to_slice(expected_sha256, &mut expected_digest).is_err()
    {
        expected_digest.zeroize();
        return Err(ExternalPoolAdapterEntrypointCapsuleError::InvalidAuthority);
    }

    let source_before = source_identity(source_file, expected_size)?;
    validate_static_elf64_x86_64(source_file, expected_size)?;
    let capsule = create_memfd()?;
    let mut copied_digest = copy_exact(source_file, &capsule, expected_size)?;
    if copied_digest != expected_digest {
        copied_digest.zeroize();
        expected_digest.zeroize();
        return Err(ExternalPoolAdapterEntrypointCapsuleError::ContentDrift);
    }
    set_capsule_mode_and_seals(&capsule)?;
    let capsule_before = capsule_identity(&capsule, expected_size)?;
    validate_static_elf64_x86_64(&capsule, expected_size)?;
    let mut capsule_digest = hash_exact(&capsule, expected_size)?;
    let mut source_after_digest = hash_exact(source_file, expected_size)?;
    let source_after = source_identity(source_file, expected_size)?;
    let capsule_after = capsule_identity(&capsule, expected_size)?;
    let exact = copied_digest == expected_digest
        && capsule_digest == expected_digest
        && source_after_digest == expected_digest
        && source_before == source_after
        && capsule_before == capsule_after;
    copied_digest.zeroize();
    capsule_digest.zeroize();
    source_after_digest.zeroize();
    expected_digest.zeroize();
    if !exact {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::ContentDrift);
    }
    require_exact_seals(&capsule)?;
    let policy = entrypoint_capsule_policy()?;
    Ok(PreparedExternalPoolAdapterEntrypointCapsule {
        sealed_image: capsule,
        entrypoint_sha256: expected_sha256.to_string(),
        entrypoint_size_bytes: expected_size,
        policy_digest: policy.policy_digest,
    })
}

fn create_memfd() -> Result<File, ExternalPoolAdapterEntrypointCapsuleError> {
    let name = CString::new("elon-external-pool-entrypoint")
        .expect("static memfd name does not contain NUL");
    let fd =
        unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING) };
    if fd < 0 {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::Unavailable);
    }
    let file = unsafe { File::from_raw_fd(fd) };
    let descriptor_flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
    if descriptor_flags < 0 || descriptor_flags & libc::FD_CLOEXEC == 0 {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::MaterializationFailed);
    }
    Ok(file)
}

fn copy_exact(
    source: &File,
    target: &File,
    expected_size: u64,
) -> Result<[u8; 32], ExternalPoolAdapterEntrypointCapsuleError> {
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; COPY_BUFFER_BYTES];
    let mut offset = 0_u64;
    while offset < expected_size {
        let requested = usize::try_from((expected_size - offset).min(buffer.len() as u64))
            .map_err(|_| ExternalPoolAdapterEntrypointCapsuleError::ContentDrift)?;
        read_exact_at(source, &mut buffer[..requested], offset)?;
        write_all_at(target, &buffer[..requested], offset)?;
        digest.update(&buffer[..requested]);
        offset = offset
            .checked_add(requested as u64)
            .ok_or(ExternalPoolAdapterEntrypointCapsuleError::ContentDrift)?;
    }
    buffer.zeroize();
    reject_extra_byte(source, expected_size)?;
    Ok(digest.finalize().into())
}

fn hash_exact(
    file: &File,
    expected_size: u64,
) -> Result<[u8; 32], ExternalPoolAdapterEntrypointCapsuleError> {
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; COPY_BUFFER_BYTES];
    let mut offset = 0_u64;
    while offset < expected_size {
        let requested = usize::try_from((expected_size - offset).min(buffer.len() as u64))
            .map_err(|_| ExternalPoolAdapterEntrypointCapsuleError::ContentDrift)?;
        read_exact_at(file, &mut buffer[..requested], offset)?;
        digest.update(&buffer[..requested]);
        offset = offset
            .checked_add(requested as u64)
            .ok_or(ExternalPoolAdapterEntrypointCapsuleError::ContentDrift)?;
    }
    buffer.zeroize();
    reject_extra_byte(file, expected_size)?;
    Ok(digest.finalize().into())
}

fn read_exact_at(
    file: &File,
    mut output: &mut [u8],
    mut offset: u64,
) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    while !output.is_empty() {
        match file.read_at(output, offset) {
            Ok(0) => return Err(ExternalPoolAdapterEntrypointCapsuleError::ContentDrift),
            Ok(read) => {
                offset = offset
                    .checked_add(read as u64)
                    .ok_or(ExternalPoolAdapterEntrypointCapsuleError::ContentDrift)?;
                output = &mut output[read..];
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(_) => return Err(ExternalPoolAdapterEntrypointCapsuleError::ContentDrift),
        }
    }
    Ok(())
}

fn write_all_at(
    file: &File,
    mut input: &[u8],
    mut offset: u64,
) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    while !input.is_empty() {
        match file.write_at(input, offset) {
            Ok(0) => return Err(ExternalPoolAdapterEntrypointCapsuleError::MaterializationFailed),
            Ok(written) => {
                offset = offset
                    .checked_add(written as u64)
                    .ok_or(ExternalPoolAdapterEntrypointCapsuleError::MaterializationFailed)?;
                input = &input[written..];
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(_) => return Err(ExternalPoolAdapterEntrypointCapsuleError::MaterializationFailed),
        }
    }
    Ok(())
}

fn reject_extra_byte(
    file: &File,
    offset: u64,
) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    let mut extra = [0_u8; 1];
    let result = loop {
        match file.read_at(&mut extra, offset) {
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            other => break other,
        }
    };
    extra.zeroize();
    match result {
        Ok(0) => Ok(()),
        Ok(_) | Err(_) => Err(ExternalPoolAdapterEntrypointCapsuleError::ContentDrift),
    }
}

fn set_capsule_mode_and_seals(
    capsule: &File,
) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    if unsafe { libc::fchmod(capsule.as_raw_fd(), CAPSULE_MODE) } != 0
        || unsafe { libc::fcntl(capsule.as_raw_fd(), libc::F_ADD_SEALS, REQUIRED_SEALS) } != 0
    {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::MaterializationFailed);
    }
    require_exact_seals(capsule)
}

fn require_exact_seals(capsule: &File) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    let observed = unsafe { libc::fcntl(capsule.as_raw_fd(), libc::F_GET_SEALS) };
    if observed != REQUIRED_SEALS {
        Err(ExternalPoolAdapterEntrypointCapsuleError::MaterializationFailed)
    } else {
        Ok(())
    }
}

fn source_identity(
    file: &File,
    expected_size: u64,
) -> Result<FileIdentity, ExternalPoolAdapterEntrypointCapsuleError> {
    let identity = identity(file)?;
    let metadata = file
        .metadata()
        .map_err(|_| ExternalPoolAdapterEntrypointCapsuleError::ContentDrift)?;
    if !metadata.is_file()
        || identity.length != expected_size
        || identity.owner != unsafe { libc::geteuid() }
        || identity.mode & 0o777 != SOURCE_MODE
        || identity.links != 1
    {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::InvalidAuthority);
    }
    Ok(identity)
}

fn capsule_identity(
    file: &File,
    expected_size: u64,
) -> Result<FileIdentity, ExternalPoolAdapterEntrypointCapsuleError> {
    let identity = identity(file)?;
    let metadata = file
        .metadata()
        .map_err(|_| ExternalPoolAdapterEntrypointCapsuleError::MaterializationFailed)?;
    if !metadata.is_file()
        || identity.length != expected_size
        || identity.owner != unsafe { libc::geteuid() }
        || identity.mode & 0o777 != CAPSULE_MODE
        || identity.links != 0
    {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::MaterializationFailed);
    }
    Ok(identity)
}

fn identity(file: &File) -> Result<FileIdentity, ExternalPoolAdapterEntrypointCapsuleError> {
    let metadata = file
        .metadata()
        .map_err(|_| ExternalPoolAdapterEntrypointCapsuleError::ContentDrift)?;
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        length: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        mode: metadata.mode(),
        owner: metadata.uid(),
        group: metadata.gid(),
        links: metadata.nlink(),
    })
}
