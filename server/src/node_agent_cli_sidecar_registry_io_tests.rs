use super::*;
use std::fs;

#[test]
fn temporary_name_conflict_is_retried_without_deleting_the_foreign_file() {
    let root = test_root("temporary-conflict");
    fs::create_dir_all(&root).unwrap();
    let target = root.join("sessions.json");
    let conflict = root.join("occupied.tmp");
    fs::write(&conflict, b"foreign-owner").unwrap();

    write_atomic_with(
        &target,
        b"new-generation",
        |_, attempt| {
            if attempt == 1 {
                conflict.clone()
            } else {
                root.join(format!("owned-{attempt}.tmp"))
            }
        },
        atomic_replace,
    )
    .unwrap();

    assert_eq!(fs::read(&conflict).unwrap(), b"foreign-owner");
    assert_eq!(fs::read(&target).unwrap(), b"new-generation");
    assert_no_owned_temporaries(&root);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn transient_replace_contention_retries_and_installs_verified_bytes() {
    let root = test_root("replace-contention");
    fs::create_dir_all(&root).unwrap();
    let target = root.join("sessions.json");
    fs::write(&target, b"old-generation").unwrap();
    let mut replacements = 0;

    write_atomic_with(
        &target,
        b"new-generation",
        |_, attempt| root.join(format!("attempt-{attempt}.tmp")),
        |from, to| {
            replacements += 1;
            if replacements < 3 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    "simulated concurrent reader",
                ))
                .context("simulated atomic replace contention");
            }
            atomic_replace(from, to)
        },
    )
    .unwrap();

    assert_eq!(replacements, 3);
    assert_eq!(fs::read(&target).unwrap(), b"new-generation");
    assert_no_owned_temporaries(&root);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn persistent_disk_pressure_preserves_primary_and_reports_raw_io_chain() {
    let root = test_root("disk-pressure");
    fs::create_dir_all(&root).unwrap();
    let target = root.join("sessions.json");
    fs::write(&target, b"last-valid-generation").unwrap();
    #[cfg(windows)]
    let disk_full_code = 112;
    #[cfg(not(windows))]
    let disk_full_code = 28;

    let error = write_atomic_with(
        &target,
        b"new-generation",
        |_, attempt| root.join(format!("pressure-{attempt}.tmp")),
        |_, _| Err(std::io::Error::from_raw_os_error(disk_full_code).into()),
    )
    .expect_err("persistent disk pressure must fail closed");

    assert_eq!(raw_os_error(&error), Some(disk_full_code));
    let detail = format!("{error:#}");
    assert!(detail.contains("有界尝试后仍失败"));
    assert!(detail.contains(&format!("raw_os_error={disk_full_code}")));
    assert_eq!(fs::read(&target).unwrap(), b"last-valid-generation");
    assert_no_owned_temporaries(&root);
    let _ = fs::remove_dir_all(root);
}

#[cfg(windows)]
#[test]
fn windows_reader_without_delete_share_reproduces_movefile_error_and_recovers() {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 0x1;
    const FILE_SHARE_WRITE: u32 = 0x2;

    let root = test_root("win32-sharing");
    fs::create_dir_all(&root).unwrap();
    let target = root.join("sessions.json");
    fs::write(&target, b"old-generation").unwrap();
    let held = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .open(&target)
        .unwrap();
    let probe = root.join("probe.tmp");
    fs::write(&probe, b"probe-generation").unwrap();

    let error = atomic_replace(&probe, &target)
        .expect_err("MoveFileExW must expose a reader that denies delete sharing");
    assert!(matches!(raw_os_error(&error), Some(5 | 32)));
    assert!(format!("{error:#}").contains("MoveFileExW"));
    drop(held);
    let _ = fs::remove_file(&probe);

    let held = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .open(&target)
        .unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        drop(held);
    });
    write_atomic(&target, b"recovered-generation").unwrap();
    release.join().unwrap();

    assert_eq!(fs::read(&target).unwrap(), b"recovered-generation");
    assert_no_owned_temporaries(&root);
    let _ = fs::remove_dir_all(root);
}

fn assert_no_owned_temporaries(root: &Path) {
    let names = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .filter(|name| {
            name.starts_with("owned-")
                || name.starts_with("attempt-")
                || name.starts_with("pressure-")
                || name.starts_with(".sessions.json.")
        })
        .collect::<Vec<_>>();
    assert!(names.is_empty(), "owned temporary files leaked: {names:?}");
}

fn test_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "elon-sidecar-registry-io-{label}-{}",
        uuid::Uuid::new_v4().simple()
    ))
}
