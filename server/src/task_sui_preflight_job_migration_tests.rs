use rusqlite::{params, Connection};

use super::migration_v159;

const USER_ID: &str = "user-v158";
const PROJECT_ID: &str = "project-v158";
const ADAPTER_ID: &str = "adapter-v158";
const CREATED_AT: &str = "2026-08-11T00:00:00Z";

#[test]
fn upgrades_v158_state_idempotently_without_losing_adapter_data() {
    let conn = legacy_v158_connection();
    migration_v159(&conn).unwrap();
    migration_v159(&conn).unwrap();

    assert_eq!(object_count(&conn, "table", "task_sui_preflight_jobs"), 1);
    assert_eq!(
        object_count(&conn, "index", "idx_task_sui_preflight_job_active_package"),
        1
    );
    assert_eq!(
        object_count(&conn, "index", "idx_task_sui_preflight_job_claim"),
        1
    );
    assert_eq!(
        object_count(&conn, "index", "idx_task_sui_preflight_job_adapter"),
        1
    );
    let adapter_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM task_sui_preflight_adapters WHERE id=?1",
            [ADAPTER_ID],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(adapter_count, 1);

    insert_job(
        &conn,
        "job-upgraded",
        PROJECT_ID,
        "standard",
        "projection-upgraded",
        "testnet",
        "pending",
        None,
        0,
        None,
        None,
        USER_ID,
    )
    .unwrap();
    let status: String = conn
        .query_row(
            "SELECT status FROM task_sui_preflight_jobs WHERE id='job-upgraded'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, "pending");
    assert_eq!(foreign_key_violation_count(&conn), 0);
}

#[test]
fn rejects_invalid_references_states_networks_and_attempts() {
    let conn = legacy_v158_connection();
    migration_v159(&conn).unwrap();

    assert!(insert_job(
        &conn,
        "job-missing-project",
        "missing-project",
        "standard",
        "projection-missing-project",
        "testnet",
        "pending",
        None,
        0,
        None,
        None,
        USER_ID,
    )
    .is_err());
    assert!(insert_job(
        &conn,
        "job-missing-user",
        PROJECT_ID,
        "standard",
        "projection-missing-user",
        "testnet",
        "pending",
        None,
        0,
        None,
        None,
        "missing-user",
    )
    .is_err());
    assert!(insert_job(
        &conn,
        "job-bad-kind",
        PROJECT_ID,
        "unknown",
        "projection-bad-kind",
        "testnet",
        "pending",
        None,
        0,
        None,
        None,
        USER_ID,
    )
    .is_err());
    assert!(insert_job(
        &conn,
        "job-bad-network",
        PROJECT_ID,
        "standard",
        "projection-bad-network",
        "localnet",
        "pending",
        None,
        0,
        None,
        None,
        USER_ID,
    )
    .is_err());
    assert!(insert_job(
        &conn,
        "job-bad-status",
        PROJECT_ID,
        "standard",
        "projection-bad-status",
        "testnet",
        "running",
        None,
        0,
        None,
        None,
        USER_ID,
    )
    .is_err());
    assert!(insert_job(
        &conn,
        "job-bad-attempt",
        PROJECT_ID,
        "standard",
        "projection-bad-attempt",
        "testnet",
        "pending",
        None,
        -1,
        None,
        None,
        USER_ID,
    )
    .is_err());
    assert!(insert_job(
        &conn,
        "job-missing-adapter",
        PROJECT_ID,
        "standard",
        "projection-missing-adapter",
        "testnet",
        "leased",
        Some("missing-adapter"),
        1,
        Some("lease-missing-adapter"),
        None,
        USER_ID,
    )
    .is_err());
    assert!(insert_job(
        &conn,
        "job-missing-report",
        PROJECT_ID,
        "standard",
        "projection-missing-report",
        "testnet",
        "completed",
        Some(ADAPTER_ID),
        1,
        None,
        Some("missing-report"),
        USER_ID,
    )
    .is_err());
    assert_eq!(foreign_key_violation_count(&conn), 0);
}

#[test]
fn enforces_active_package_lease_and_report_uniqueness() {
    let conn = legacy_v158_connection();
    migration_v159(&conn).unwrap();

    insert_job(
        &conn,
        "job-active-first",
        PROJECT_ID,
        "standard",
        "projection-active",
        "testnet",
        "pending",
        None,
        0,
        None,
        None,
        USER_ID,
    )
    .unwrap();
    assert!(insert_job(
        &conn,
        "job-active-second",
        PROJECT_ID,
        "standard",
        "projection-active",
        "testnet",
        "leased",
        Some(ADAPTER_ID),
        1,
        Some("lease-active-second"),
        None,
        USER_ID,
    )
    .is_err());
    conn.execute(
        "UPDATE task_sui_preflight_jobs SET status='completed' WHERE id='job-active-first'",
        [],
    )
    .unwrap();
    insert_job(
        &conn,
        "job-active-retry",
        PROJECT_ID,
        "standard",
        "projection-active",
        "testnet",
        "pending",
        None,
        0,
        None,
        None,
        USER_ID,
    )
    .unwrap();

    insert_job(
        &conn,
        "job-lease-first",
        PROJECT_ID,
        "standard",
        "projection-lease-first",
        "testnet",
        "leased",
        Some(ADAPTER_ID),
        1,
        Some("shared-lease-hash"),
        None,
        USER_ID,
    )
    .unwrap();
    assert!(insert_job(
        &conn,
        "job-lease-second",
        PROJECT_ID,
        "standard",
        "projection-lease-second",
        "testnet",
        "leased",
        Some(ADAPTER_ID),
        1,
        Some("shared-lease-hash"),
        None,
        USER_ID,
    )
    .is_err());

    insert_report(&conn, "report-shared", "report-shared-idempotency");
    insert_job(
        &conn,
        "job-report-first",
        PROJECT_ID,
        "standard",
        "projection-report-first",
        "testnet",
        "completed",
        Some(ADAPTER_ID),
        1,
        None,
        Some("report-shared"),
        USER_ID,
    )
    .unwrap();
    assert!(insert_job(
        &conn,
        "job-report-second",
        PROJECT_ID,
        "standard",
        "projection-report-second",
        "testnet",
        "completed",
        Some(ADAPTER_ID),
        1,
        None,
        Some("report-shared"),
        USER_ID,
    )
    .is_err());
    assert_eq!(foreign_key_violation_count(&conn), 0);
}

fn legacy_v158_connection() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE users(id TEXT PRIMARY KEY);
         CREATE TABLE projects(id TEXT PRIMARY KEY);",
    )
    .unwrap();
    crate::task_sui_preflight_migration::migration_v158(&conn).unwrap();
    conn.execute("INSERT INTO users(id) VALUES (?1)", [USER_ID])
        .unwrap();
    conn.execute("INSERT INTO projects(id) VALUES (?1)", [PROJECT_ID])
        .unwrap();
    conn.execute(
        "INSERT INTO task_sui_preflight_adapters (
           id, project_id, display_name, status, allowed_networks_json,
           allowed_package_kinds_json, token_hash, token_hint,
           credential_version, created_by_user_id, expires_at, created_at, updated_at
         ) VALUES (?1, ?2, 'V158 worker', 'active', '[\"testnet\"]',
                   '[\"standard\"]', 'v158-token-hash', '...v158', 1, ?3,
                   '2099-01-01T00:00:00Z', ?4, ?4)",
        params![ADAPTER_ID, PROJECT_ID, USER_ID, CREATED_AT],
    )
    .unwrap();
    conn
}

#[allow(clippy::too_many_arguments)]
fn insert_job(
    conn: &Connection,
    id: &str,
    project_id: &str,
    package_kind: &str,
    projection_id: &str,
    target_network: &str,
    status: &str,
    adapter_id: Option<&str>,
    attempt_no: i64,
    lease_token_hash: Option<&str>,
    report_id: Option<&str>,
    created_by_user_id: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO task_sui_preflight_jobs (
           id, project_id, package_kind, projection_package_id,
           target_network, handoff_digest, projection_digest, status,
           adapter_id, credential_version, attempt_no, lease_token_hash,
           report_id, created_by_user_id, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                   CASE WHEN ?9 IS NULL THEN NULL ELSE 1 END,
                   ?10, ?11, ?12, ?13, ?14, ?14)",
        params![
            id,
            project_id,
            package_kind,
            projection_id,
            target_network,
            format!("handoff-{id}"),
            format!("projection-{id}"),
            status,
            adapter_id,
            attempt_no,
            lease_token_hash,
            report_id,
            created_by_user_id,
            CREATED_AT,
        ],
    )
}

fn insert_report(conn: &Connection, id: &str, idempotency_key: &str) {
    conn.execute(
        "INSERT INTO task_sui_preflight_reports (
           id, project_id, adapter_id, credential_version, package_kind,
           projection_package_id, target_network, handoff_digest,
           projection_digest, outcome, summary, tool_version,
           idempotency_key, report_digest, created_at
         ) VALUES (?1, ?2, ?3, 1, 'standard', 'projection-report', 'testnet',
                   'handoff-report', 'projection-digest-report', 'passed',
                   'migration test report', 'migration-test-v1', ?4,
                   'report-digest', ?5)",
        params![id, PROJECT_ID, ADAPTER_ID, idempotency_key, CREATED_AT],
    )
    .unwrap();
}

fn object_count(conn: &Connection, object_type: &str, name: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type=?1 AND name=?2",
        params![object_type, name],
        |row| row.get(0),
    )
    .unwrap()
}

fn foreign_key_violation_count(conn: &Connection) -> usize {
    let mut statement = conn.prepare("PRAGMA foreign_key_check").unwrap();
    statement
        .query_map([], |_| Ok(()))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
        .len()
}
