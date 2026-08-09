use std::time::{SystemTime, UNIX_EPOCH};

use super::*;

#[test]
fn managed_sqlite_vfs_routes_real_rollback_connection_through_file_custody() -> anyhow::Result<()> {
    let root = unique_root("rollback");
    let fixture = ManagedSqliteRoutedConnectionFixture::open(&root, [0x6e; 16])?;
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
    let journal_mode: String = fixture
        .connection()
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .context("read managed VFS rollback journal mode")?;
    assert_eq!(journal_mode.to_ascii_lowercase(), "delete");
    fixture
        .connection()
        .execute_batch(
            "PRAGMA synchronous=FULL;
         PRAGMA foreign_keys=ON;
         PRAGMA trusted_schema=OFF;
         PRAGMA temp_store=MEMORY;
         PRAGMA mmap_size=0;",
        )
        .context("configure managed VFS bootstrap pragmas")?;
    fixture.into_schema_migration()?;
    fixture
        .connection()
        .execute_batch(
            "CREATE TABLE inventory (
             sku TEXT PRIMARY KEY,
             quantity INTEGER NOT NULL
         );
         PRAGMA user_version=8;
         PRAGMA application_id=1162625872;
         PRAGMA foreign_key_check;",
        )
        .context("run managed VFS schema migration")?;
    fixture.into_runtime()?;
    fixture
        .connection()
        .execute(
            "INSERT INTO inventory(sku, quantity) VALUES (?1, ?2)",
            ("coffee", 9),
        )
        .context("insert through managed VFS runtime")?;
    let quantity: i64 = fixture
        .connection()
        .query_row(
            "SELECT sum(quantity) FROM inventory WHERE sku = ?1",
            ["coffee"],
            |row| row.get(0),
        )
        .context("query through managed VFS runtime")?;
    assert_eq!(quantity, 9);

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

    let live_counts = fixture.counts();
    assert_eq!(live_counts.main_opens, 1);
    assert!(live_counts.journal_opens >= 1);
    assert_eq!(live_counts.wal_open_attempts, 0);
    let closed_counts = fixture.close()?;
    assert_eq!(closed_counts, live_counts);
    fs::remove_dir_all(&root)
        .with_context(|| format!("remove closed managed VFS root at {}", root.display()))?;
    Ok(())
}

#[test]
fn managed_sqlite_vfs_promotes_main_and_routes_real_wal_connection() -> anyhow::Result<()> {
    let root = unique_root("wal");
    let fixture = ManagedSqliteRoutedConnectionFixture::open(&root, [0x7a; 16])?;
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
    let journal_mode: String = fixture
        .connection()
        .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
        .context("enable managed VFS WAL journal mode")?;
    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
    fixture
        .connection()
        .execute_batch(
            "PRAGMA synchronous=FULL;
             PRAGMA foreign_keys=ON;
             PRAGMA trusted_schema=OFF;
             PRAGMA temp_store=MEMORY;
             PRAGMA mmap_size=0;",
        )
        .context("configure managed WAL bootstrap pragmas")?;
    fixture.into_schema_migration()?;
    fixture
        .connection()
        .execute_batch(
            "CREATE TABLE inventory (
                 sku TEXT PRIMARY KEY,
                 quantity INTEGER NOT NULL
             );
             PRAGMA user_version=8;
             PRAGMA application_id=1162625872;
             PRAGMA foreign_key_check;",
        )
        .context("run managed WAL schema migration")?;
    fixture.into_runtime()?;
    fixture
        .connection()
        .execute(
            "INSERT INTO inventory(sku, quantity) VALUES (?1, ?2)",
            ("tea", 12),
        )
        .context("insert through managed WAL runtime")?;
    let quantity: i64 = fixture
        .connection()
        .query_row(
            "SELECT sum(quantity) FROM inventory WHERE sku = ?1",
            ["tea"],
            |row| row.get(0),
        )
        .context("query through managed WAL runtime")?;
    assert_eq!(quantity, 12);
    assert!(
        fixture
            .connection()
            .execute_batch("PRAGMA user_version")
            .is_err(),
        "runtime PRAGMA must remain denied after WAL promotion"
    );

    let live_counts = fixture.counts();
    assert_eq!(live_counts.main_opens, 1);
    assert!(live_counts.wal_open_attempts >= 1);
    let closed_counts = fixture.close()?;
    assert_eq!(closed_counts, live_counts);
    fs::remove_dir_all(&root)
        .with_context(|| format!("remove closed managed WAL VFS root at {}", root.display()))?;
    Ok(())
}

fn unique_root(mode: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "elon-managed-vfs-{mode}-{}-{unique}",
        std::process::id()
    ))
}
