use rusqlite::Connection;
use uuid::Uuid;

use crate::store::Store;

#[test]
fn platform_reference_price_curve_v224_replaces_legacy_ttl_trigger_on_reopen() {
    let root = std::env::temp_dir().join(format!(
        "elon-reference-curve-v224-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("temporary migration directory should exist");
    let database = root.join("state.sqlite");

    drop(Store::open(&database).expect("current Store should migrate through v224"));
    let connection = Connection::open(&database).expect("migrated database should reopen");
    connection
        .execute_batch(
            "DROP TRIGGER trg_platform_reference_curve_binding_source;
             CREATE TRIGGER trg_platform_reference_curve_binding_source
             BEFORE INSERT ON compute_platform_reference_price_curve_snapshot_bindings
             BEGIN
                 SELECT RAISE(ABORT, 'legacy floating TTL guard');
             END;
             DELETE FROM schema_migrations WHERE version=224;",
        )
        .expect("legacy v223 trigger should be simulated");
    drop(connection);

    drop(Store::open(&database).expect("v224 should replace the legacy trigger"));
    assert_repaired(&database);
    drop(Store::open(&database).expect("reopening an applied v224 database should be idempotent"));
    assert_repaired(&database);

    for path in [
        database.clone(),
        root.join("state.sqlite-wal"),
        root.join("state.sqlite-shm"),
    ] {
        if path.exists() {
            std::fs::remove_file(path).expect("temporary database artifact should be removable");
        }
    }
    std::fs::remove_dir(root).expect("temporary migration directory should be empty");
}

fn assert_repaired(database: &std::path::Path) {
    let connection = Connection::open(database).expect("repaired database should open");
    let trigger_sql: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master
             WHERE type='trigger' AND name='trg_platform_reference_curve_binding_source'",
            [],
            |row| row.get(0),
        )
        .expect("repaired trigger should exist");
    assert!(trigger_sql.contains("strftime('%s'"), "{trigger_sql}");
    assert!(!trigger_sql.contains("julianday"), "{trigger_sql}");
    let migration_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=224",
            [],
            |row| row.get(0),
        )
        .expect("v224 migration row should read");
    assert_eq!(migration_count, 1);
}
