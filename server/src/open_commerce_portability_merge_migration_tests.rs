use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::{store::Store, store_migrations::MIGRATIONS};

use super::migration_v161;

const USER_ID: &str = "legacy-portability-owner";
const PROJECT_ID: &str = "legacy-portability-project";
const IMPORT_ID: &str = "legacy-portability-import";

#[test]
fn v141_disk_database_upgrades_without_losing_single_source_records() {
    let (root, database) = legacy_v141_database();

    let upgraded = Store::open(&database).expect("v141 database should upgrade to current schema");
    {
        let connection = upgraded
            .conn()
            .expect("upgraded database should be readable");
        assert_eq!(latest_migration(&connection), current_migration());
        assert_eq!(migration_count(&connection, 161), 1);
        assert_eq!(
            table_count(
                &connection,
                "open_commerce_consumer_portability_merge_adoptions"
            ),
            1
        );
        assert_eq!(
            index_count(&connection, "idx_open_commerce_portability_merge_owner"),
            1
        );
        assert_eq!(
            record_count(&connection, "open_commerce_consumer_preference_profiles"),
            1
        );
        assert_eq!(
            record_count(&connection, "open_commerce_consumer_portability_imports"),
            1
        );
        assert_eq!(
            record_count(&connection, "open_commerce_consumer_portability_adoptions"),
            1
        );
        let revision: i64 = connection
            .query_row(
                "SELECT revision FROM open_commerce_consumer_preference_profiles
                  WHERE consumer_project_id=?1 AND consumer_user_id=?2",
                params![PROJECT_ID, USER_ID],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(revision, 2);

        migration_v161(&connection).expect("v161 must be repeatable over current schema");
        assert_eq!(
            table_count(
                &connection,
                "open_commerce_consumer_portability_merge_adoptions"
            ),
            1
        );
        assert_eq!(
            index_count(&connection, "idx_open_commerce_portability_merge_owner"),
            1
        );
    }
    drop(upgraded);

    let reopened = Store::open(&database).expect("fully migrated database should reopen cleanly");
    {
        let connection = reopened
            .conn()
            .expect("reopened database should be readable");
        assert_eq!(latest_migration(&connection), current_migration());
        assert_eq!(migration_count(&connection, 161), 1);
        assert_eq!(
            record_count(&connection, "open_commerce_consumer_portability_adoptions"),
            1
        );
    }
    drop(reopened);
    cleanup_database(&root, &database);
}

fn legacy_v141_database() -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "elon-portability-merge-v141-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("temporary migration directory should exist");
    let database = root.join("state.sqlite");
    let connection = Connection::open(&database).expect("legacy database should open");
    connection
        .execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE schema_migrations (
               version INTEGER PRIMARY KEY,
               applied_at TEXT NOT NULL
             );",
        )
        .expect("legacy migration ledger should initialize");
    for (version, _, apply) in MIGRATIONS.iter().filter(|(version, _, _)| *version <= 141) {
        apply(&connection).unwrap_or_else(|error| panic!("migration v{version} failed: {error:#}"));
        connection
            .execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
                params![version, "2026-08-01T00:00:00Z"],
            )
            .unwrap_or_else(|error| panic!("migration v{version} ledger failed: {error:#}"));
    }
    assert_eq!(latest_migration(&connection), 141);
    assert_eq!(
        table_count(
            &connection,
            "open_commerce_consumer_portability_merge_adoptions"
        ),
        0
    );
    insert_legacy_records(&connection);
    drop(connection);
    (root, database)
}

fn insert_legacy_records(connection: &Connection) {
    let before = r#"{"categories":["tea"],"tags":["nearby"],"city":"Beijing","max_unit_price_micros":30000000,"prefer_public":false}"#;
    let applied = r#"{"categories":["coffee"],"tags":["quiet"],"city":"Shanghai","max_unit_price_micros":80000000,"prefer_public":true}"#;
    connection
        .execute_batch(&format!(
            "INSERT INTO users(
               id, email, password_hash, nickname, role, status, created_at, updated_at
             ) VALUES (
               '{USER_ID}', 'legacy-portability@example.test', 'test-hash', 'Legacy Owner',
               'user', 'active', '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z'
             );
             INSERT INTO projects(
               id, name, workspace_key, template, source_type, status,
               created_by, created_at, updated_at
             ) VALUES (
               '{PROJECT_ID}', 'Legacy Portability', 'legacy-portability-key', 'android',
               'template', 'active', '{USER_ID}',
               '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z'
             );"
        ))
        .expect("legacy owner and project should insert");
    connection
        .execute(
            "INSERT INTO open_commerce_consumer_preference_profiles(
               consumer_project_id, consumer_user_id, preferences_json, revision,
               created_at, updated_at
             ) VALUES (?1, ?2, ?3, 2, ?4, ?4)",
            params![PROJECT_ID, USER_ID, applied, "2026-08-01T00:00:00Z"],
        )
        .expect("legacy preference profile should insert");
    connection
        .execute(
            "INSERT INTO open_commerce_consumer_portability_imports(
               id, destination_project_id, consumer_user_id, source_operator,
               source_project_id, source_package_id, source_package_schema,
               envelope_sha256, payload_sha256, package_json, imported_at
             ) VALUES (?1, ?2, ?3, 'legacy-operator', ?2, 'legacy-package',
               'open_commerce.consumer_portability_export.v1', 'legacy-envelope',
               'legacy-payload', '{}', ?4)",
            params![IMPORT_ID, PROJECT_ID, USER_ID, "2026-08-01T00:00:00Z"],
        )
        .expect("legacy import should insert");
    connection
        .execute(
            "INSERT INTO open_commerce_consumer_portability_adoptions(
               id, import_id, destination_project_id, consumer_user_id, adoption_kind,
               before_preferences_json, before_revision, applied_preferences_json,
               resulting_revision, status, applied_at
             ) VALUES ('legacy-single-adoption', ?1, ?2, ?3, 'preferences',
               ?4, 1, ?5, 2, 'applied', ?6)",
            params![
                IMPORT_ID,
                PROJECT_ID,
                USER_ID,
                before,
                applied,
                "2026-08-01T00:00:00Z"
            ],
        )
        .expect("legacy single-source adoption should insert");
}

fn current_migration() -> i64 {
    i64::from(
        MIGRATIONS
            .last()
            .expect("migration list should not be empty")
            .0,
    )
}

fn latest_migration(connection: &Connection) -> i64 {
    connection
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("latest migration should be readable")
}

fn migration_count(connection: &Connection, version: i64) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=?1",
            [version],
            |row| row.get(0),
        )
        .expect("migration count should be readable")
}

fn table_count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |row| row.get(0),
        )
        .expect("table count should be readable")
}

fn index_count(connection: &Connection, index: &str) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
            [index],
            |row| row.get(0),
        )
        .expect("index count should be readable")
}

fn record_count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("record count should be readable")
}

fn cleanup_database(root: &Path, database: &Path) {
    for path in [
        database.to_path_buf(),
        root.join("state.sqlite-wal"),
        root.join("state.sqlite-shm"),
    ] {
        if path.exists() {
            std::fs::remove_file(path).expect("temporary database artifact should be removable");
        }
    }
    std::fs::remove_dir(root).expect("temporary migration directory should be empty");
}
