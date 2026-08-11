use rusqlite::{params, Connection};

use crate::compute_federation::external_pool_adapter_release_lifecycle::EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED;

use super::{lifecycle_support::*, *};

const TERMINAL_TABLE: &str = "compute_external_pool_adapter_release_admission_terminal_receipts";
const CURRENT_VIEW: &str = "compute_external_pool_adapter_release_admission_current";

#[test]
fn lifecycle_v229_schema_shape_is_present_and_repeatable() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let connection = store.conn().expect("Store connection should open");

    assert_columns(
        &connection,
        TERMINAL_TABLE,
        &[
            "terminal_receipt_id",
            "terminal_receipt_schema",
            "terminal_receipt_digest",
            "terminal_receipt_json",
            "canonicalization",
            "digest_algorithm",
            "request_digest",
            "admission_id",
            "admission_digest",
            "adapter_id",
            "release_version",
            "prior_status",
            "terminal_status",
            "successor_admission_id",
            "successor_admission_digest",
            "successor_release_version",
            "actor_kind",
            "actor_id",
            "reason",
            "confirmation",
            "idempotency_scope",
            "idempotency_key",
            "occurred_at",
            "recorded_at",
            "currentness_effect",
            "artifact_intake_effect",
            "existing_artifact_source_effect",
            "adapter_effect",
            "route_effect",
        ],
    );
    assert_columns(
        &connection,
        CURRENT_VIEW,
        &[
            "admission_id",
            "admission_digest",
            "adapter_id",
            "release_version",
            "applied_at",
            "admission_status",
            "current_status",
            "terminal_receipt_id",
            "terminal_receipt_digest",
            "terminal_occurred_at",
            "successor_admission_id",
            "successor_admission_digest",
            "successor_release_version",
        ],
    );

    for (kind, name) in required_schema_objects() {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type=?1 AND name=?2",
                params![kind, name],
                |row| row.get(0),
            )
            .expect("v229 schema object should be queryable");
        assert_eq!(count, 1, "missing {kind} {name}");
    }
    assert_single_v229_migration(&connection);

    connection
        .execute("DELETE FROM schema_migrations WHERE version=229", [])
        .expect("test should expose the v229 repeat path");
    crate::store_schema::apply_migrations(&connection)
        .expect("v229 migration should be repeatable over its exact schema");
    assert_single_v229_migration(&connection);
    assert_eq!(columns(&connection, TERMINAL_TABLE).len(), 29);
    assert_eq!(columns(&connection, CURRENT_VIEW).len(), 13);

    drop(connection);
    drop(store);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

#[tokio::test]
async fn lifecycle_v228_upgrade_preserves_v222_v227_and_two_reopens() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let release = stage_release(&store, "upgrade-pool", "1.0.0", "upgrade-base");
    let artifact = record_artifact(&store, &data_dir, &release, "upgrade-artifact").await;
    drop(store);

    let connection = Connection::open(&database_path).expect("pre-upgrade database should reopen");
    strip_v229(&connection);
    drop(connection);

    let upgraded = Store::open(&database_path).expect("simulated v228 database should upgrade");
    let current = upgraded
        .external_pool_adapter_release_admission_currentness(&release.admission_id)
        .expect("upgraded currentness should read")
        .expect("upgraded admission should exist");
    assert_eq!(current.admission_status, "staged");
    assert_eq!(current.current_status, "staged");
    let historical = upgraded
        .external_pool_adapter_artifact_source_for_admission(&release.admission_id)
        .expect("upgraded artifact history should read")
        .expect("v227 receipt should survive v229");
    assert_eq!(historical.source_receipt_id, artifact.source_receipt_id);

    let terminal = upgraded
        .create_external_pool_adapter_release_admission_terminal(terminal_input(
            &release,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            "upgrade-terminal",
        ))
        .expect("upgraded admission should accept its terminal");
    drop(upgraded);

    let connection = Connection::open(&database_path).expect("repeat database should reopen");
    connection
        .execute("DELETE FROM schema_migrations WHERE version=229", [])
        .expect("test should force an applied-data migration repeat");
    drop(connection);

    let first_reopen = Store::open(&database_path).expect("v229 repeat with data should succeed");
    let replay = first_reopen
        .create_external_pool_adapter_release_admission_terminal(terminal_input(
            &release,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            "upgrade-terminal",
        ))
        .expect("terminal should replay after migration repeat");
    assert!(replay.replayed);
    assert_eq!(
        replay.terminal_receipt.terminal_receipt_id,
        terminal.terminal_receipt.terminal_receipt_id
    );
    drop(first_reopen);

    let second_reopen = Store::open(&database_path).expect("second reopen should succeed");
    let current = second_reopen
        .external_pool_adapter_release_admission_currentness(&release.admission_id)
        .unwrap()
        .unwrap();
    assert_eq!(current.admission_status, "staged");
    assert_eq!(current.current_status, "revoked");
    assert_eq!(
        second_reopen
            .external_pool_adapter_artifact_source_for_admission(&release.admission_id)
            .unwrap()
            .unwrap()
            .source_receipt_id,
        artifact.source_receipt_id
    );
    drop(second_reopen);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

#[tokio::test]
async fn lifecycle_v229_sql_guards_are_append_only_and_block_terminal_intake() {
    let (store, database_path, data_dir) = temporary_lifecycle_store();
    let release = stage_release(&store, "guard-pool", "1.0.0", "guard-base");
    record_artifact(&store, &data_dir, &release, "guard-artifact").await;
    store
        .create_external_pool_adapter_release_admission_terminal(terminal_input(
            &release,
            EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
            "guard-terminal",
        ))
        .expect("guard fixture should become terminal");
    drop(store);

    let connection = Connection::open(&database_path).expect("guard database should reopen");
    for statement in [
        format!("UPDATE {TERMINAL_TABLE} SET reason=reason WHERE admission_id=?1"),
        format!("DELETE FROM {TERMINAL_TABLE} WHERE admission_id=?1"),
        format!(
            "INSERT OR REPLACE INTO {TERMINAL_TABLE} SELECT * FROM {TERMINAL_TABLE} WHERE admission_id=?1"
        ),
    ] {
        assert!(
            connection
                .execute(&statement, params![release.admission_id])
                .is_err(),
            "terminal history mutation must fail: {statement}"
        );
    }
    let base_status: String = connection
        .query_row(
            "SELECT status FROM compute_external_pool_adapter_release_admissions WHERE admission_id=?1",
            params![release.admission_id],
            |row| row.get(0),
        )
        .expect("immutable v222 status should read");
    assert_eq!(base_status, "staged");

    connection
        .execute_batch(
            "CREATE TEMP TABLE copied_artifact_source AS
                 SELECT * FROM compute_external_pool_adapter_artifact_source_receipts;
             DROP TRIGGER trg_external_pool_adapter_artifact_source_no_delete;
             DELETE FROM compute_external_pool_adapter_artifact_source_receipts;",
        )
        .expect("test should isolate the v229 artifact currentness trigger");
    let rejection = connection
        .execute(
            "INSERT INTO compute_external_pool_adapter_artifact_source_receipts
                 SELECT * FROM copied_artifact_source",
            [],
        )
        .expect_err("terminal admission must reject an otherwise exact v227 receipt");
    assert!(rejection.to_string().contains("admission is terminal"));

    drop(connection);
    cleanup_lifecycle_files(&database_path, &data_dir);
}

fn columns(connection: &Connection, object: &str) -> Vec<String> {
    connection
        .prepare(&format!("PRAGMA table_info({object})"))
        .expect("column query should prepare")
        .query_map([], |row| row.get::<_, String>(1))
        .expect("columns should be queryable")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("columns should decode")
}

fn assert_columns(connection: &Connection, object: &str, expected: &[&str]) {
    let actual = columns(connection, object);
    let actual = actual.iter().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(actual.as_slice(), expected);
}

fn required_schema_objects() -> [(&'static str, &'static str); 11] {
    [
        ("table", TERMINAL_TABLE),
        ("view", CURRENT_VIEW),
        (
            "trigger",
            "trg_external_pool_adapter_release_terminal_projection",
        ),
        (
            "trigger",
            "trg_external_pool_adapter_release_terminal_exact_source",
        ),
        (
            "trigger",
            "trg_external_pool_adapter_release_terminal_successor",
        ),
        (
            "trigger",
            "trg_external_pool_adapter_artifact_source_current_admission",
        ),
        (
            "trigger",
            "trg_external_pool_adapter_release_terminal_no_update",
        ),
        (
            "trigger",
            "trg_external_pool_adapter_release_terminal_no_delete",
        ),
        (
            "trigger",
            "trg_external_pool_adapter_release_terminal_no_replace",
        ),
        ("index", "idx_external_pool_adapter_release_terminal_status"),
        (
            "index",
            "idx_external_pool_adapter_release_terminal_successor",
        ),
    ]
}

fn assert_single_v229_migration(connection: &Connection) {
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=229",
            [],
            |row| row.get(0),
        )
        .expect("v229 migration row should be queryable");
    assert_eq!(count, 1);
}

fn strip_v229(connection: &Connection) {
    connection
        .execute_batch(
            "DROP VIEW compute_external_pool_adapter_release_admission_current;
             DROP TRIGGER trg_external_pool_adapter_release_terminal_projection;
             DROP TRIGGER trg_external_pool_adapter_release_terminal_exact_source;
             DROP TRIGGER trg_external_pool_adapter_release_terminal_successor;
             DROP TRIGGER trg_external_pool_adapter_artifact_source_current_admission;
             DROP TRIGGER trg_external_pool_adapter_release_terminal_no_update;
             DROP TRIGGER trg_external_pool_adapter_release_terminal_no_delete;
             DROP TRIGGER trg_external_pool_adapter_release_terminal_no_replace;
             DROP TABLE compute_external_pool_adapter_release_admission_terminal_receipts;
             DELETE FROM schema_migrations WHERE version=229;",
        )
        .expect("test fixture should become an exact pre-v229 schema");
}
