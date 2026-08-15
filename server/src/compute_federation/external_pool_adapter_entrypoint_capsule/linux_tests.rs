use std::{
    cell::Cell,
    fs::{self, File, OpenOptions},
    io::Write,
    os::{
        fd::{AsRawFd, RawFd},
        unix::{fs::FileExt, fs::PermissionsExt},
    },
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use anyhow::Result;
use sha2::{Digest, Sha256};

use super::{with_external_pool_adapter_entrypoint_capsule, ExternalPoolAdapterEntrypointSource};

const SOURCE_MODE: u32 = 0o600;
const CAPSULE_MODE: u32 = 0o500;
const REQUIRED_SEALS: libc::c_int =
    libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;
const F_SEAL_FUTURE_WRITE: libc::c_int = 0x0010;
static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

struct EntrypointFixture {
    root: PathBuf,
    path: PathBuf,
    file: File,
    sha256: String,
    size_bytes: u64,
}

impl EntrypointFixture {
    fn new(bytes: &[u8]) -> Self {
        let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "elon-v257-entrypoint-capsule-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("create isolated V257 fixture directory");
        let path = root.join("entrypoint.bin");
        let mut file = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&path)
            .expect("create V257 source fixture");
        file.write_all(bytes).expect("write V257 source fixture");
        file.sync_all().expect("flush V257 source fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(SOURCE_MODE))
            .expect("set exact V257 source mode");
        Self {
            root,
            path,
            file,
            sha256: format!("{:x}", Sha256::digest(bytes)),
            size_bytes: bytes.len() as u64,
        }
    }
}

impl Drop for EntrypointFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl ExternalPoolAdapterEntrypointSource for EntrypointFixture {
    fn retained_entrypoint(&self) -> Result<(&File, &str, u64)> {
        Ok((&self.file, &self.sha256, self.size_bytes))
    }
}

#[test]
fn linux_kernel_materializes_exact_immutable_memfd_and_drops_descriptor() {
    let bytes = minimal_static_elf64();
    let fixture = EntrypointFixture::new(&bytes);
    let observed_fd = Cell::new(-1);

    with_external_pool_adapter_entrypoint_capsule(&fixture, |capsule| {
        let source_image = &capsule.sealed_image;
        let image = &capsule.launch_image;
        let fd = image.as_raw_fd();
        observed_fd.set(fd);
        let metadata = image.metadata()?;

        assert!(metadata.is_file());
        assert_eq!(metadata.permissions().mode() & 0o777, CAPSULE_MODE);
        assert!(metadata.len() > bytes.len() as u64);
        assert_eq!(metadata_nlink(image), 0);
        assert_eq!(capsule.entrypoint_size_bytes(), bytes.len() as u64);
        assert_eq!(capsule.entrypoint_sha256(), fixture.sha256);
        assert_eq!(capsule.launch_size_bytes(), metadata.len());
        assert_eq!(hash_file(source_image, bytes.len()), fixture.sha256);
        assert_eq!(
            hash_file(image, metadata.len() as usize),
            capsule.launch_sha256()
        );
        assert_ne!(capsule.launch_sha256(), fixture.sha256);
        assert!(!capsule.policy_digest().is_empty());

        let descriptor_flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        assert_ne!(descriptor_flags, -1);
        assert_ne!(descriptor_flags & libc::FD_CLOEXEC, 0);
        assert_eq!(
            unsafe { libc::fcntl(fd, libc::F_GET_SEALS) },
            REQUIRED_SEALS
        );

        let write_error = image
            .write_at(&[0x90], 0)
            .expect_err("sealed capsule must reject writes");
        assert_eq!(write_error.raw_os_error(), Some(libc::EPERM));
        assert_ftruncate_rejected(fd, bytes.len() as i64 + 1);
        assert_ftruncate_rejected(fd, bytes.len() as i64 - 1);
        assert_eq!(
            unsafe { libc::fcntl(fd, libc::F_ADD_SEALS, F_SEAL_FUTURE_WRITE) },
            -1
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EPERM)
        );
        Ok(())
    })
    .expect("materialize V257 capsule on Linux");

    assert_descriptor_closed(observed_fd.get());
}

#[test]
fn linux_kernel_rejects_insecure_source_mode_and_hard_links() {
    let bytes = minimal_static_elf64();
    let fixture = EntrypointFixture::new(&bytes);
    fs::set_permissions(&fixture.path, fs::Permissions::from_mode(0o640))
        .expect("set insecure source mode");
    assert_consumer_not_called(&fixture);

    let fixture = EntrypointFixture::new(&bytes);
    fs::hard_link(&fixture.path, fixture.root.join("entrypoint-link.bin"))
        .expect("create forbidden extra source link");
    assert_consumer_not_called(&fixture);
}

#[test]
fn linux_kernel_rejects_digest_and_size_drift() {
    let bytes = minimal_static_elf64();
    let mut fixture = EntrypointFixture::new(&bytes);
    fixture.sha256 = "00".repeat(32);
    assert_consumer_not_called(&fixture);

    let mut fixture = EntrypointFixture::new(&bytes);
    fixture.size_bytes -= 1;
    assert_consumer_not_called(&fixture);

    let mut fixture = EntrypointFixture::new(&bytes);
    fixture.size_bytes += 1;
    assert_consumer_not_called(&fixture);
}

#[test]
fn linux_kernel_rejects_unsafe_elf_without_creating_consumer_effect() {
    let mut bytes = minimal_static_elf64();
    bytes[16..18].copy_from_slice(&3_u16.to_le_bytes());
    let fixture = EntrypointFixture::new(&bytes);
    assert_consumer_not_called(&fixture);
}

fn assert_consumer_not_called(source: &EntrypointFixture) {
    let called = Cell::new(false);
    let result = with_external_pool_adapter_entrypoint_capsule(source, |_| {
        called.set(true);
        Ok(())
    });
    assert!(result.is_err());
    assert!(!called.get());
}

fn assert_ftruncate_rejected(fd: RawFd, length: i64) {
    assert_eq!(unsafe { libc::ftruncate(fd, length) }, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::EPERM)
    );
}

fn assert_descriptor_closed(fd: RawFd) {
    assert!(fd >= 0);
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::EBADF)
    );
}

fn metadata_nlink(file: &File) -> u64 {
    use std::os::unix::fs::MetadataExt;
    file.metadata().expect("read capsule metadata").nlink()
}

fn hash_file(file: &File, length: usize) -> String {
    let mut bytes = vec![0_u8; length];
    let mut offset = 0;
    while offset < bytes.len() {
        let read = file
            .read_at(&mut bytes[offset..], offset as u64)
            .expect("read sealed capsule");
        assert_ne!(read, 0);
        offset += read;
    }
    format!("{:x}", Sha256::digest(bytes))
}

fn minimal_static_elf64() -> Vec<u8> {
    const ELF_HEADER_BYTES: usize = 64;
    const PROGRAM_HEADER_BYTES: usize = 56;
    const IMAGE_BYTES: usize = ELF_HEADER_BYTES + PROGRAM_HEADER_BYTES;
    const LOAD_ADDRESS: u64 = 0x0040_0000;

    let mut image = vec![0_u8; IMAGE_BYTES];
    image[..4].copy_from_slice(b"\x7fELF");
    image[4] = 2;
    image[5] = 1;
    image[6] = 1;
    put_u16(&mut image, 16, 2);
    put_u16(&mut image, 18, 62);
    put_u32(&mut image, 20, 1);
    put_u64(&mut image, 24, LOAD_ADDRESS + ELF_HEADER_BYTES as u64);
    put_u64(&mut image, 32, ELF_HEADER_BYTES as u64);
    put_u16(&mut image, 52, ELF_HEADER_BYTES as u16);
    put_u16(&mut image, 54, PROGRAM_HEADER_BYTES as u16);
    put_u16(&mut image, 56, 1);

    let program = ELF_HEADER_BYTES;
    put_u32(&mut image, program, 1);
    put_u32(&mut image, program + 4, 5);
    put_u64(&mut image, program + 8, 0);
    put_u64(&mut image, program + 16, LOAD_ADDRESS);
    put_u64(&mut image, program + 24, LOAD_ADDRESS);
    put_u64(&mut image, program + 32, IMAGE_BYTES as u64);
    put_u64(&mut image, program + 40, IMAGE_BYTES as u64);
    put_u64(&mut image, program + 48, 4096);
    image
}

fn put_u16(output: &mut [u8], offset: usize, value: u16) {
    output[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(output: &mut [u8], offset: usize, value: u32) {
    output[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(output: &mut [u8], offset: usize, value: u64) {
    output[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
