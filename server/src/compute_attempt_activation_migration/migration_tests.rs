use rusqlite::Connection;
use uuid::Uuid;

use crate::store::Store;

#[test]
fn migrations_v211_to_v215_apply_idempotently_with_required_schema() {
    let connection = Connection::open_in_memory().expect("in-memory database should open");

    crate::store_schema::apply_migrations(&connection).expect("full schema should migrate to v215");
    crate::store_schema::apply_migrations(&connection)
        .expect("reapplying the current schema should be idempotent");

    assert_v211_to_v215_schema(&connection);
}

#[test]
fn migrations_v211_to_v215_survive_two_file_database_reopens() {
    let root = std::env::temp_dir().join(format!(
        "elon-compute-attempt-v215-disk-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("temporary migration directory should exist");
    let database = root.join("state.sqlite");

    drop(Store::open(&database).expect("file Store should apply the full schema"));
    let connection = Connection::open(&database).expect("migrated file database should reopen");
    assert_v211_to_v215_schema(&connection);
    crate::store_schema::apply_migrations(&connection)
        .expect("reapplying migrations after a file reopen should be idempotent");
    assert_single_migration_rows(&connection);
    drop(connection);

    drop(Store::open(&database).expect("file Store should survive a second reopen"));
    let connection = Connection::open(&database).expect("twice-reopened database should be valid");
    assert_v211_to_v215_schema(&connection);
    assert_single_migration_rows(&connection);
    drop(connection);

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

fn assert_v211_to_v215_schema(connection: &Connection) {
    let versions = connection
        .prepare(
            "SELECT version FROM schema_migrations
             WHERE version BETWEEN 211 AND 215 ORDER BY version",
        )
        .expect("migration version query should prepare")
        .query_map([], |row| row.get::<_, u32>(0))
        .expect("migration versions should be queryable")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("migration versions should decode");
    assert_eq!(versions, vec![211, 212, 213, 214, 215]);

    let required_objects = [
        ("table", "compute_attempt_dispatch_commands"),
        ("table", "compute_attempt_dispatch_acks"),
        ("table", "compute_attempt_dispatch_applications"),
        ("table", "compute_execution_capability_receipts"),
        ("table", "compute_artifact_access_receipts"),
        ("table", "compute_attempt_execution_plans"),
        ("table", "compute_attempt_execution_plan_accesses"),
        ("table", "compute_attempt_execution_plan_seals"),
        ("table", "compute_attempt_start_outbox"),
        ("table", "compute_attempt_start_send_attempts"),
        ("table", "compute_attempt_start_remote_observations"),
        ("table", "compute_attempt_no_start_proofs"),
        (
            "trigger",
            "trg_compute_attempt_dispatch_commands_sealed_plan_v212",
        ),
        ("trigger", "trg_compute_attempt_start_send_attempt_claim"),
        ("trigger", "trg_compute_attempt_remote_no_start_source_v214"),
        (
            "trigger",
            "trg_compute_attempt_accepted_ack_blocks_cleanup_v215",
        ),
        (
            "trigger",
            "trg_compute_attempt_application_live_authority_v215",
        ),
    ];

    for (object_type, object_name) in required_objects {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type=?1 AND name=?2",
                (object_type, object_name),
                |row| row.get(0),
            )
            .expect("schema object should be queryable");
        assert_eq!(count, 1, "missing {object_type} {object_name}");
    }
}

fn assert_single_migration_rows(connection: &Connection) {
    for version in 211..=215 {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version=?1",
                [version],
                |row| row.get(0),
            )
            .expect("migration row should be queryable");
        assert_eq!(count, 1, "migration v{version} should appear exactly once");
    }
}
