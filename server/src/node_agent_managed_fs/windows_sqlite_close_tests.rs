use std::{
    fs::{File, OpenOptions},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use super::*;

static NEXT_TEST_FILE: AtomicU64 = AtomicU64::new(1);

fn test_file(label: &str) -> (PathBuf, File) {
    let serial = NEXT_TEST_FILE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "elon-managed-sqlite-{label}-{}-{serial}.db",
        std::process::id()
    ));
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("create isolated CloseHandle test file");
    (path, file)
}

#[test]
fn retryable_close_observes_one_real_native_failure_and_returns_live_file() {
    let (path, file) = test_file("close-retryable");
    let native =
        close_sqlite_file_for_test_native(file, PlatformManagedSqliteCloseTestNative::Retryable);
    assert_eq!(
        native.exact_call_occurrence.map(|value| value.get()),
        Some(1)
    );
    assert_eq!(
        native.observation,
        Some(ManagedSqliteShmTestUnmapNativeObservation::NativeFailureObserved)
    );
    let failure = native.result.expect_err("protected CloseHandle must fail");
    let file = match failure.custody {
        PlatformManagedSqliteCloseCustody::Unattempted(file) => file,
        PlatformManagedSqliteCloseCustody::OutcomeUncertainRawHandle(_) => {
            panic!("retryable close must retain live File custody")
        }
    };
    drop(file);
    std::fs::remove_file(path).expect("remove retryable CloseHandle test file");
}

#[test]
fn uncertain_close_discards_one_real_call_receipt_and_returns_terminal_custody() {
    let (path, file) = test_file("close-uncertain");
    let native = close_sqlite_file_for_test_native(
        file,
        PlatformManagedSqliteCloseTestNative::OutcomeUncertain,
    );
    assert_eq!(
        native.exact_call_occurrence.map(|value| value.get()),
        Some(1)
    );
    assert_eq!(
        native.observation,
        Some(ManagedSqliteShmTestUnmapNativeObservation::ReturnReceiptUnavailable)
    );
    let failure = native
        .result
        .expect_err("discarded CloseHandle receipt is never projected as success");
    assert!(failure
        .error
        .get_ref()
        .and_then(|source| {
            source
                .downcast_ref::<super::super::PlatformManagedSqliteNativeReturnReceiptUnavailable>()
        })
        .is_some());
    match failure.custody {
        PlatformManagedSqliteCloseCustody::OutcomeUncertainRawHandle(raw_handle) => {
            assert_ne!(raw_handle, 0)
        }
        PlatformManagedSqliteCloseCustody::Unattempted(_) => {
            panic!("unavailable receipt must not return live File custody")
        }
    }
    std::fs::remove_file(path).expect("remove uncertain CloseHandle test file");
}
