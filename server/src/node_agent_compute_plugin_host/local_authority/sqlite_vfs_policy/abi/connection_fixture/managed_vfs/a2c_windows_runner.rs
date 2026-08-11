//! Windows-only SQL-driven foundation for the managed-VFS dynamic evidence runner.
//!
//! This test does not publish dynamic evidence. It proves one route-exact callback fault through
//! a real registered VFS and two real `rusqlite::Connection` values when the test is eventually
//! compiled and run on Windows.

use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context};
use rusqlite::{ffi, Error};

use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole;

#[test]
fn wal_sql_shm_map_fault_is_one_shot_and_route_exact() -> anyhow::Result<()> {
    let root = unique_root();
    let mut fixture = ManagedSqliteMultiConnectionFixture::open(&root, [0xa2; 16])?;
    assert_eq!(fixture.live_route_count()?, 2);

    let selected_route = fixture.route_ordinal(0)?;
    let sibling_route = fixture.route_ordinal(1)?;
    assert_ne!(selected_route, sibling_route);
    let selected_step = ManagedTestCallbackFaultStep::new(
        selected_route,
        ManagedSqliteLogicalFileRole::Main,
        ManagedTestCallbackFaultOperation::ShmMap,
        1,
        ManagedTestCallbackFaultTiming::BeforeCall,
    )
    .map_err(anyhow::Error::msg)?;
    fixture
        .install_callback_fault_script(&[selected_step])
        .map_err(anyhow::Error::msg)?;

    let sibling_mode: String =
        fixture
            .connection(1)?
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    assert_eq!(sibling_mode.to_ascii_lowercase(), "wal");
    fixture.route(1)?.into_schema_migration()?;
    fixture.connection(1)?.execute_batch(
        "CREATE TABLE a2c_probe (
             probe_id INTEGER PRIMARY KEY,
             value INTEGER NOT NULL
         );",
    )?;
    fixture.route(1)?.into_runtime()?;
    fixture
        .connection(1)?
        .execute("INSERT INTO a2c_probe(probe_id, value) VALUES (1, 41)", [])?;

    assert_eq!(
        fixture
            .pending_callback_fault_count()
            .map_err(anyhow::Error::msg)?,
        1
    );
    assert!(fixture
        .callback_fault_observations()
        .map_err(anyhow::Error::msg)?
        .is_empty());

    fixture.route(0)?.into_schema_migration()?;
    fixture.route(0)?.into_runtime()?;
    assert_shm_map_failure(fixture.connection(0)?.query_row(
        "SELECT value FROM a2c_probe WHERE probe_id = 1",
        [],
        |row| row.get::<_, i64>(0),
    ))?;

    assert_eq!(
        fixture
            .pending_callback_fault_count()
            .map_err(anyhow::Error::msg)?,
        0
    );
    let observations = fixture
        .callback_fault_observations()
        .map_err(anyhow::Error::msg)?
        .into_iter()
        .map(ManagedTestCallbackFaultObservation::step)
        .collect::<Vec<_>>();
    assert_eq!(observations, vec![selected_step]);

    fixture
        .connection(1)?
        .execute("UPDATE a2c_probe SET value = 42 WHERE probe_id = 1", [])?;
    let sibling_value: i64 = fixture.connection(1)?.query_row(
        "SELECT value FROM a2c_probe WHERE probe_id = 1",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(sibling_value, 42);

    let retried_value: i64 = fixture.connection(0)?.query_row(
        "SELECT value FROM a2c_probe WHERE probe_id = 1",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(retried_value, 42);
    assert_eq!(
        fixture
            .callback_fault_observations()
            .map_err(anyhow::Error::msg)?
            .len(),
        1
    );

    assert_eq!(fixture.live_route_count()?, 2);
    fixture.close_connection(0)?;
    assert_eq!(fixture.live_route_count()?, 1);
    fixture.close_connection(1)?;
    assert_eq!(fixture.live_route_count()?, 0);
    fixture.close()?;
    fs::remove_dir_all(&root)
        .with_context(|| format!("remove closed A2c runner root at {}", root.display()))?;
    Ok(())
}

fn assert_shm_map_failure<T>(result: rusqlite::Result<T>) -> anyhow::Result<()> {
    match result {
        Err(Error::SqliteFailure(failure, _))
            if failure.extended_code == ffi::SQLITE_IOERR_SHMMAP =>
        {
            Ok(())
        }
        Err(error) => Err(error).context("selected WAL SQL returned a non-SHMMAP error"),
        Ok(_) => Err(anyhow!("selected WAL SQL unexpectedly succeeded")),
    }
}

fn unique_root() -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "elon-managed-vfs-a2c-shm-map-{}-{unique}",
        std::process::id()
    ))
}
