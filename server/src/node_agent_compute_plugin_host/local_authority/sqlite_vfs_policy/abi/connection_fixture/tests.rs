use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;

use super::*;

#[test]
fn real_connection_uses_named_vfs_and_keeps_security_policy_for_lifetime() -> anyhow::Result<()> {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "elon-sqlite-connection-fixture-{}-{unique}.sqlite3",
        std::process::id()
    ));
    let opens_before = test_transport_open_count();
    let mut fixture = ManagedSqliteConnectionFixture::open(&path, [0x7d; 16])?;
    assert!(test_transport_open_count() > opens_before);
    assert_eq!(
        fixture.security_snapshot()?,
        ConnectionSecuritySnapshot {
            defensive: 1,
            trusted_schema: 0,
            dqs_dml: 0,
            dqs_ddl: 0,
            load_extension: 0,
            attached_limit: 0,
            worker_thread_limit: 0,
        }
    );

    fixture.connection().execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=FULL;
         PRAGMA foreign_keys=ON;
         PRAGMA trusted_schema=OFF;
         PRAGMA temp_store=MEMORY;
         PRAGMA mmap_size=0;",
    )?;
    fixture.into_schema_migration()?;
    fixture.connection().execute_batch(
        "CREATE TABLE inventory (
             sku TEXT PRIMARY KEY,
             quantity INTEGER NOT NULL
         );
         PRAGMA user_version=6;
         PRAGMA application_id=1162625872;
         PRAGMA foreign_key_check;",
    )?;
    fixture.into_runtime()?;

    fixture.connection().execute(
        "INSERT INTO inventory(sku, quantity) VALUES (?1, ?2)",
        ("coffee", 4),
    )?;
    let quantity: i64 = fixture.connection().query_row(
        "SELECT sum(quantity) FROM inventory WHERE sku = ?1",
        ["coffee"],
        |row| row.get(0),
    )?;
    assert_eq!(quantity, 4);

    for forbidden in [
        "ATTACH DATABASE ':memory:' AS extra",
        "CREATE TEMP TABLE forbidden(value INTEGER)",
        "PRAGMA user_version",
        "SELECT load_extension('missing')",
        "SELECT lower('not-allowlisted')",
        "SELECT \"legacy dqs literal\"",
    ] {
        assert!(
            fixture.connection().execute_batch(forbidden).is_err(),
            "statement must be rejected: {forbidden}"
        );
    }

    fixture.close()?;
    fs::remove_file(&path)
        .with_context(|| format!("remove closed SQLite file at {}", path.display()))?;
    for suffix in ["-wal", "-shm", "-journal"] {
        let sidecar = std::path::PathBuf::from(format!("{}{suffix}", path.display()));
        if sidecar.exists() {
            fs::remove_file(&sidecar).with_context(|| {
                format!("remove closed SQLite sidecar at {}", sidecar.display())
            })?;
        }
    }
    Ok(())
}
