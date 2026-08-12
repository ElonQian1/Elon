use rusqlite::{params, Connection};

pub(super) const REVOKED_AT: &str = "2026-08-13T00:00:01.000000000Z";

const TERMINAL_SCHEMA: &str =
    "compute_federation.external_pool_adapter_installation_terminal_receipt.v1";
const INSTALLED_AT: &str = "2026-08-13T00:00:00.000000000Z";

pub(super) fn create_v246_fixture_schema(connection: &Connection) {
    connection
        .execute_batch(
            r#"
            PRAGMA foreign_keys=ON;
            CREATE TABLE compute_external_pool_adapter_installation_receipts (
                installation_receipt_id TEXT PRIMARY KEY,
                installation_receipt_digest TEXT NOT NULL UNIQUE,
                receipt_json TEXT NOT NULL,
                entry_count INTEGER NOT NULL,
                adoption_receipt_id TEXT NOT NULL,
                adoption_receipt_digest TEXT NOT NULL,
                package_receipt_id TEXT NOT NULL,
                package_receipt_digest TEXT NOT NULL,
                source_receipt_id TEXT NOT NULL,
                source_receipt_digest TEXT NOT NULL,
                installed_at TEXT NOT NULL
            );
            CREATE TABLE compute_external_pool_adapter_installation_files (
                installation_receipt_id TEXT NOT NULL,
                size_bytes INTEGER NOT NULL
            );
            CREATE TABLE compute_external_pool_adapter_adoption_current (
                adoption_receipt_id TEXT NOT NULL,
                adoption_receipt_digest TEXT NOT NULL,
                current_status TEXT NOT NULL
            );
            CREATE TABLE compute_external_pool_adapter_artifact_package_current (
                package_receipt_id TEXT NOT NULL,
                package_receipt_digest TEXT NOT NULL,
                current_status TEXT NOT NULL
            );
            CREATE TABLE compute_external_pool_adapter_artifact_source_receipts (
                source_receipt_id TEXT NOT NULL,
                source_receipt_digest TEXT NOT NULL
            );
            "#,
        )
        .unwrap();
}

pub(super) fn seed_current_installation(connection: &Connection) {
    connection
        .execute(
            "INSERT INTO compute_external_pool_adapter_installation_receipts(
                 installation_receipt_id,installation_receipt_digest,receipt_json,entry_count,
                 adoption_receipt_id,adoption_receipt_digest,package_receipt_id,
                 package_receipt_digest,source_receipt_id,source_receipt_digest,installed_at)
             VALUES('installation-1',?1,
                 '{\"installation\":{\"binding\":{\"installed_files\":[{\"size_bytes\":4}]}}}',
                 1,'adoption-1',?2,'package-1',?3,'source-1',?4,?5)",
            params![
                digest('a'),
                digest('1'),
                digest('2'),
                digest('3'),
                INSTALLED_AT
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_external_pool_adapter_installation_files VALUES(
                 'installation-1',4)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_external_pool_adapter_adoption_current VALUES(
                 'adoption-1',?1,'adopted_current')",
            [digest('1')],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_external_pool_adapter_artifact_package_current VALUES(
                 'package-1',?1,'verified_current')",
            [digest('2')],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_external_pool_adapter_artifact_source_receipts VALUES(
                 'source-1',?1)",
            [digest('3')],
        )
        .unwrap();
}

#[allow(clippy::too_many_arguments)]
pub(super) fn insert_terminal(
    connection: &Connection,
    terminal_receipt_id: &str,
    terminal_receipt_digest: &str,
    installation_receipt_digest: &str,
    json_installation_receipt_digest: &str,
    idempotency_key: &str,
    revoked_at: &str,
    json_terminal_kind: &str,
) -> rusqlite::Result<usize> {
    let material_digest = digest('8');
    let receipt_json = serde_json::json!({
        "schema": TERMINAL_SCHEMA,
        "terminal_receipt_id": terminal_receipt_id,
        "terminal_receipt_digest": terminal_receipt_digest,
        "terminal_material_digest": material_digest,
        "canonicalization": "rfc8785_jcs",
        "digest_algorithm": "sha256",
        "terminal": {
            "installation_receipt_id": "installation-1",
            "installation_receipt_digest": json_installation_receipt_digest,
            "terminal_kind": json_terminal_kind,
            "revoked_by_admin_user_id": "admin-1",
            "reason": "compromised adapter bytes",
            "confirmation": "confirm_external_pool_adapter_installation_revocation",
            "idempotency_scope": "installation-revocation",
            "idempotency_key": idempotency_key,
            "revoked_at": revoked_at,
            "recorded_at": revoked_at,
            "installation_effect": "installed_instance_revoked",
            "credential_effect": "none",
            "provider_effect": "none",
            "route_effect": "none",
            "execution_effect": "none",
            "settlement_effect": "none"
        }
    })
    .to_string();
    connection.execute(
        "INSERT INTO compute_external_pool_adapter_installation_terminal_receipts(
             terminal_receipt_id,terminal_receipt_digest,terminal_receipt_schema,receipt_json,
             terminal_material_digest,canonicalization,digest_algorithm,installation_receipt_id,
             installation_receipt_digest,terminal_kind,revoked_by_admin_user_id,reason,
             confirmation,idempotency_scope,idempotency_key,revoked_at,recorded_at,
             installation_effect,credential_effect,provider_effect,route_effect,execution_effect,
             settlement_effect)
         VALUES(?1,?2,?3,?4,?5,'rfc8785_jcs','sha256','installation-1',?6,'revoked',
                'admin-1','compromised adapter bytes',
                'confirm_external_pool_adapter_installation_revocation',
                'installation-revocation',?7,?8,?8,'installed_instance_revoked','none','none',
                'none','none','none')",
        params![
            terminal_receipt_id,
            terminal_receipt_digest,
            TERMINAL_SCHEMA,
            receipt_json,
            material_digest,
            installation_receipt_digest,
            idempotency_key,
            revoked_at,
        ],
    )
}

pub(super) fn digest(character: char) -> String {
    character.to_string().repeat(64)
}

pub(super) fn object_count(connection: &Connection, name: &str) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name=?1",
            [name],
            |row| row.get(0),
        )
        .unwrap()
}

pub(super) fn object_sql(connection: &Connection, kind: &str, name: &str) -> String {
    connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type=?1 AND name=?2",
            [kind, name],
            |row| row.get(0),
        )
        .unwrap()
}

pub(super) fn object_columns(connection: &Connection, object: &str) -> Vec<String> {
    connection
        .prepare(&format!("PRAGMA table_info({object})"))
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
}
