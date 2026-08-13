use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::{store::Store, store_migrations::MIGRATIONS};

use super::migration_v154;

const CREDENTIAL_TABLE: &str = "open_commerce_developer_production_credentials";
const APP_ID: &str = "legacy-app-record";
const ADMISSION_ID: &str = "legacy-admission";
const PROJECT_ID: &str = "legacy-project";
const USER_ID: &str = "legacy-owner";

#[test]
fn v153_disk_database_upgrades_to_repeatable_v154_schema() {
    let (root, database) = legacy_v153_database();

    let upgraded = Store::open(&database).expect("v153 database should upgrade through v154");
    {
        let connection = upgraded
            .conn()
            .expect("upgraded database should be readable");
        assert_eq!(latest_migration(&connection), current_migration());
        assert_eq!(migration_count(&connection, 154), 1);
        assert_eq!(credential_count(&connection), 0);
        assert_eq!(
            record_count(&connection, "open_commerce_developer_apps", APP_ID),
            1
        );
        assert_eq!(
            record_count(
                &connection,
                "open_commerce_developer_app_admissions",
                ADMISSION_ID,
            ),
            1
        );
        assert_v154_schema(&connection);

        migration_v154(&connection).expect("v154 should be idempotent over its populated schema");
        assert_v154_schema(&connection);
        assert_eq!(migration_count(&connection, 154), 1);

        insert_credential(&connection, "credential-active", "active")
            .expect("first active credential should be accepted");
        let conflict = insert_credential(&connection, "credential-conflict", "active")
            .expect_err("partial unique index should reject a second active App credential");
        assert!(conflict.to_string().contains("UNIQUE constraint failed"));
        insert_credential(&connection, "credential-revoked", "revoked")
            .expect("revoked credential history should remain appendable");

        let restricted = connection
            .execute(
                "DELETE FROM open_commerce_developer_app_admissions WHERE id=?1",
                [ADMISSION_ID],
            )
            .expect_err("admission with credential history should be delete-restricted");
        assert!(restricted
            .to_string()
            .contains("FOREIGN KEY constraint failed"));
    }
    drop(upgraded);

    let reopened = Store::open(&database).expect("fully migrated database should reopen cleanly");
    {
        let connection = reopened
            .conn()
            .expect("reopened database should be readable");
        assert_eq!(latest_migration(&connection), current_migration());
        assert_eq!(migration_count(&connection, 154), 1);
        assert_eq!(credential_count(&connection), 2);
        assert_v154_schema(&connection);
    }
    drop(reopened);
    cleanup_database(&root, &database);
}

fn legacy_v153_database() -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "elon-open-commerce-production-credential-v153-{}",
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
    for (version, _, apply) in MIGRATIONS.iter().filter(|(version, _, _)| *version <= 153) {
        apply(&connection).unwrap_or_else(|error| panic!("migration v{version} failed: {error:#}"));
        connection
            .execute(
                "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
                params![version, "2026-08-01T00:00:00Z"],
            )
            .unwrap_or_else(|error| panic!("migration v{version} ledger failed: {error:#}"));
    }
    assert_eq!(latest_migration(&connection), 153);
    assert_eq!(table_count(&connection, CREDENTIAL_TABLE), 0);
    insert_legacy_app_and_admission(&connection);
    drop(connection);
    (root, database)
}

fn insert_legacy_app_and_admission(connection: &Connection) {
    connection
        .execute_batch(
            "INSERT INTO users(
               id, email, password_hash, nickname, role, status, created_at, updated_at
             ) VALUES (
               'legacy-owner', 'legacy@example.test', 'test-hash', 'Legacy Owner',
               'user', 'active', '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z'
             );
             INSERT INTO projects(
               id, name, workspace_key, template, source_type, status,
               created_by, created_at, updated_at
             ) VALUES (
               'legacy-project', 'Legacy Project', 'legacy-project-key', 'android',
               'template', 'active', 'legacy-owner',
               '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z'
             );
             INSERT INTO open_commerce_developer_apps(
               id, project_id, owner_user_id, app_id, display_name, environment,
               status, test_token_hash, token_hint, created_at, updated_at
             ) VALUES (
               'legacy-app-record', 'legacy-project', 'legacy-owner', 'legacy.consumer',
               'Legacy Consumer', 'sandbox', 'active', 'legacy-test-token-hash',
               'legacy...', '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z'
             );
             INSERT INTO open_commerce_developer_app_admissions(
               id, app_record_id, project_id, manifest_revision, organization_name,
               jurisdiction, registration_id, attested_at, status, requested_at,
               reviewed_at, reviewed_by_user_id, review_note, risk_tier,
               created_at, updated_at
             ) VALUES (
               'legacy-admission', 'legacy-app-record', 'legacy-project', 0,
               'Legacy Merchant Ltd', 'Test Jurisdiction', 'LEGACY-001',
               '2026-08-01T00:00:00Z', 'approved', '2026-08-01T00:00:00Z',
               '2026-08-01T00:00:00Z', 'legacy-reviewer', 'approved fixture', 'standard',
               '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z'
             );",
        )
        .expect("v153 fixture data should insert");
}

fn insert_credential(
    connection: &Connection,
    credential_id: &str,
    status: &str,
) -> rusqlite::Result<usize> {
    connection.execute(
        "INSERT INTO open_commerce_developer_production_credentials(
           id, app_record_id, project_id, admission_id, manifest_revision,
           scopes_json, status, token_hash, token_hint, issued_by_user_id,
           issued_at, expires_at, created_at, updated_at
         ) VALUES (
           ?1, ?2, ?3, ?4, 0, '[\"menu.preview\"]', ?5, ?6, 'oc_live_...', ?7,
           '2026-08-01T00:00:00Z', '2026-09-01T00:00:00Z',
           '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z'
         )",
        params![
            credential_id,
            APP_ID,
            PROJECT_ID,
            ADMISSION_ID,
            status,
            format!("hash-{credential_id}"),
            USER_ID,
        ],
    )
}

fn assert_v154_schema(connection: &Connection) {
    assert_eq!(table_count(connection, CREDENTIAL_TABLE), 1);
    let columns: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info(?1)",
            [CREDENTIAL_TABLE],
            |row| row.get(0),
        )
        .expect("credential columns should be readable");
    assert_eq!(columns, 17);

    for (index, unique, partial) in [
        ("idx_open_commerce_production_credentials_active_app", 1, 1),
        ("idx_open_commerce_production_credentials_project", 0, 0),
        ("idx_open_commerce_production_credentials_expiry", 0, 0),
    ] {
        let shape: (i64, i64) = connection
            .query_row(
                "SELECT [unique], partial FROM pragma_index_list(?1) WHERE name=?2",
                params![CREDENTIAL_TABLE, index],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or_else(|error| panic!("index {index} should exist: {error}"));
        assert_eq!(shape, (unique, partial), "unexpected index shape: {index}");
    }

    let foreign_keys = connection
        .prepare(&format!("PRAGMA foreign_key_list('{CREDENTIAL_TABLE}')"))
        .expect("foreign key query should prepare")
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(6)?,
            ))
        })
        .expect("foreign keys should be queryable")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("foreign keys should decode");
    assert!(foreign_keys.contains(&(
        "open_commerce_developer_apps".to_string(),
        "app_record_id".to_string(),
        "id".to_string(),
        "CASCADE".to_string(),
    )));
    assert!(foreign_keys.contains(&(
        "open_commerce_developer_app_admissions".to_string(),
        "admission_id".to_string(),
        "id".to_string(),
        "RESTRICT".to_string(),
    )));
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

fn credential_count(connection: &Connection) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM open_commerce_developer_production_credentials",
            [],
            |row| row.get(0),
        )
        .expect("credential count should be readable")
}

fn record_count(connection: &Connection, table: &str, id: &str) -> i64 {
    connection
        .query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE id=?1"),
            [id],
            |row| row.get(0),
        )
        .expect("historical record count should be readable")
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
