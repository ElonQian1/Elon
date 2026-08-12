//! Windows-only process-isolated direct xShmUnmap ShmFileClose physical-subset runner.
//!
//! The child retains the deliberately poisoned fixture until process exit. This source does not
//! publish WindowsDynamic evidence until a later acceptance batch compiles and runs it.

use std::{fs, mem, path::Path, process::Command};

use anyhow::Context;

use super::a2b2_cases::{
    validate_shm_file_close_after_success_physical_subset, ShmFileClosePhysicalSubsetActual,
};
use super::*;
use crate::node_agent_managed_fs::{ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase};

const CHILD_ROOT_ENV: &str = "ELON_SQLITE_A2C_SHM_FILE_CLOSE_CHILD_ROOT";
const EXACT_TEST: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2c_shm_file_close_runner::final_keep_shm_file_close_after_success_physical_subset";

#[test]
fn final_keep_shm_file_close_after_success_physical_subset() -> anyhow::Result<()> {
    if let Some(root) = std::env::var_os(CHILD_ROOT_ENV).map(std::path::PathBuf::from) {
        exercise_child(&root)?;
        return Ok(());
    }

    let root = std::env::temp_dir().join(format!(
        "elon-managed-vfs-a2c-shm-file-close-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let status = Command::new(std::env::current_exe().context("resolve current test executable")?)
        .args(["--exact", EXACT_TEST, "--nocapture"])
        .env(CHILD_ROOT_ENV, &root)
        .status()
        .context("run isolated direct xShmUnmap ShmFileClose fault child")?;
    let cleanup = fs::remove_dir_all(&root).with_context(|| {
        format!(
            "remove ShmFileClose child namespace after process exit at {}",
            root.display()
        )
    });
    assert!(status.success(), "ShmFileClose fault child must pass");
    cleanup?;
    Ok(())
}

fn exercise_child(root: &Path) -> anyhow::Result<()> {
    let fixture = ManagedSqliteRoutedConnectionFixture::open(root, [0xe5; 16])?;
    // Runtime `FileClose` is the physical seam represented by frozen `Phase::ShmFileClose`.
    let phase = ManagedSqliteShmFailurePhase::FileClose;
    fixture
        .install_shm_fault_script(&[(phase, 1, ManagedSqliteShmFailureClass::MutatedButKnown)])
        .map_err(anyhow::Error::msg)?;

    let mode: String = fixture
        .connection()
        .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    assert_eq!(mode.to_ascii_lowercase(), "wal");
    fixture.into_schema_migration()?;
    fixture.connection().execute_batch(
        "CREATE TABLE a2c_shm_file_close_probe (
             probe_id INTEGER PRIMARY KEY,
             value INTEGER NOT NULL
         );",
    )?;
    fixture.into_runtime()?;
    fixture.connection().execute(
        "INSERT INTO a2c_shm_file_close_probe(probe_id, value) VALUES (1, 85)",
        [],
    )?;

    let witness = fixture
        .installed_shm_fault_witness()
        .map_err(anyhow::Error::msg)?;
    let observer = witness.observer().map_err(anyhow::Error::msg)?;
    let pre = observer.snapshot()?;
    let pending_before = checked_u8(
        witness.pending_count().map_err(anyhow::Error::msg)?,
        "pre fault pending count",
    )?;
    let triggered_before = witness
        .was_triggered(phase, 1)
        .map_err(anyhow::Error::msg)?;

    let callback_result_code = fixture.call_main_shm_unmap_keep();
    // The one-shot seam activates only after close returned a real receipt. Retain immediately;
    // no fallible post-observation may Drop this terminal route into another xShmUnmap.
    mem::forget(fixture);

    let post = observer.snapshot()?;
    let pending_after = checked_u8(
        witness.pending_count().map_err(anyhow::Error::msg)?,
        "post fault pending count",
    )?;
    let triggered_after = witness
        .was_triggered(phase, 1)
        .map_err(anyhow::Error::msg)?;
    validate_shm_file_close_after_success_physical_subset(ShmFileClosePhysicalSubsetActual {
        callback_result_code,
        pre,
        post,
        pending_before,
        pending_after,
        triggered_before,
        triggered_after,
    })
    .map_err(anyhow::Error::msg)?;

    assert!(root.is_dir(), "poisoned ShmFileClose child keeps its root");
    Ok(())
}

fn checked_u8(value: usize, label: &'static str) -> anyhow::Result<u8> {
    u8::try_from(value).with_context(|| format!("{label} exceeds u8"))
}
