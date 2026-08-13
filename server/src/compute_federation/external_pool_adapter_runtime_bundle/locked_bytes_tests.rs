use super::*;
use std::{
    fs,
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_FILE_ID: AtomicU64 = AtomicU64::new(1);

struct TestFile {
    path: PathBuf,
}

impl TestFile {
    fn open(bytes: &[u8]) -> (Self, File) {
        let path = std::env::temp_dir().join(format!(
            "elon-v256-locked-bytes-{}-{}.bin",
            std::process::id(),
            NEXT_FILE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let mut writer = fs::File::create(&path).expect("create test-only sensitive file");
        writer
            .write_all(bytes)
            .expect("write test-only sensitive file");
        writer.sync_all().expect("sync test-only sensitive file");
        drop(writer);
        let reader = fs::File::open(&path).expect("reopen test-only sensitive file");
        (Self { path }, reader)
    }
}

impl Drop for TestFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[test]
fn locked_sensitive_bytes_reads_exact_test_material() {
    let bytes = b"v256-test-only-credential";
    let (_file, mut reader) = TestFile::open(bytes);

    let custody = LockedSensitiveBytes::read_exact(&mut reader, bytes.len() as u64)
        .expect("lock exact test material");

    assert_eq!(custody.as_slice(), bytes);
}

#[test]
fn locked_sensitive_bytes_rejects_short_and_trailing_material() {
    let bytes = b"v256-test-only-config";
    let (_short_file, mut short_reader) = TestFile::open(bytes);
    assert!(matches!(
        LockedSensitiveBytes::read_exact(&mut short_reader, bytes.len() as u64 + 1),
        Err(ExternalPoolAdapterRuntimeBundleError::ContentDrift)
    ));

    let (_trailing_file, mut trailing_reader) = TestFile::open(bytes);
    assert!(matches!(
        LockedSensitiveBytes::read_exact(&mut trailing_reader, bytes.len() as u64 - 1),
        Err(ExternalPoolAdapterRuntimeBundleError::ContentDrift)
    ));
}
