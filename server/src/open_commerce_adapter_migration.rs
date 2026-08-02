use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v136(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_adapter_credentials (
           id                  TEXT PRIMARY KEY,
           project_id          TEXT NOT NULL,
           merchant_id         TEXT NOT NULL,
           integration_id      TEXT NOT NULL UNIQUE,
           status              TEXT NOT NULL CHECK(status IN ('active', 'revoked')),
           scopes_json         TEXT NOT NULL DEFAULT '[\"business_handoff.write\"]',
           token_hash          TEXT NOT NULL UNIQUE,
           token_hint          TEXT NOT NULL,
           credential_version  INTEGER NOT NULL DEFAULT 1 CHECK(credential_version > 0),
           created_by_user_id  TEXT NOT NULL,
           last_used_at        TEXT,
           created_at          TEXT NOT NULL,
           updated_at          TEXT NOT NULL,
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           FOREIGN KEY(integration_id) REFERENCES open_commerce_integrations(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_adapter_credentials_project
           ON open_commerce_adapter_credentials(project_id, status, updated_at DESC);

         CREATE TABLE open_commerce_business_handoff_receipts_v136 (
           id                       TEXT PRIMARY KEY,
           project_id               TEXT NOT NULL,
           merchant_id              TEXT NOT NULL,
           invocation_id            TEXT NOT NULL,
           integration_id           TEXT NOT NULL,
           receipt_key              TEXT NOT NULL,
           receipt_fingerprint      TEXT NOT NULL,
           status                   TEXT NOT NULL
                                    CHECK(status IN ('applied', 'ignored', 'rejected')),
           target_domain            TEXT NOT NULL CHECK(target_domain IN ('erp', 'crm')),
           evidence_result_sha256   TEXT NOT NULL,
           target_reference_sha256  TEXT,
           error_code               TEXT,
           confirmed_by_user        INTEGER NOT NULL CHECK(confirmed_by_user IN (0, 1)),
           assertion_authority      TEXT NOT NULL CHECK(assertion_authority IN (
                                      'project_editor_asserted',
                                      'adapter_token_authenticated'
                                    )),
           adapter_credential_id    TEXT,
           adapter_credential_version INTEGER,
           recorded_by_user_id      TEXT NOT NULL,
           recorded_by_app_id       TEXT NOT NULL,
           completed_at             TEXT NOT NULL,
           created_at               TEXT NOT NULL,
           UNIQUE(integration_id, receipt_key),
           CHECK(
             (assertion_authority = 'project_editor_asserted'
               AND confirmed_by_user = 1 AND adapter_credential_id IS NULL
               AND adapter_credential_version IS NULL)
             OR
             (assertion_authority = 'adapter_token_authenticated'
               AND confirmed_by_user = 0 AND adapter_credential_id IS NOT NULL
               AND adapter_credential_version > 0)
           ),
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           FOREIGN KEY(invocation_id) REFERENCES open_commerce_invocations(id) ON DELETE CASCADE,
           FOREIGN KEY(integration_id) REFERENCES open_commerce_integrations(id) ON DELETE CASCADE,
           FOREIGN KEY(adapter_credential_id) REFERENCES open_commerce_adapter_credentials(id)
         );
         INSERT INTO open_commerce_business_handoff_receipts_v136 (
           id, project_id, merchant_id, invocation_id, integration_id,
           receipt_key, receipt_fingerprint, status, target_domain,
           evidence_result_sha256, target_reference_sha256, error_code,
           confirmed_by_user, assertion_authority, adapter_credential_id,
           adapter_credential_version, recorded_by_user_id, recorded_by_app_id,
           completed_at, created_at
         )
         SELECT id, project_id, merchant_id, invocation_id, integration_id,
                receipt_key, receipt_fingerprint, status, target_domain,
                evidence_result_sha256, target_reference_sha256, error_code,
                confirmed_by_user, assertion_authority, NULL, NULL,
                recorded_by_user_id, recorded_by_app_id, completed_at, created_at
           FROM open_commerce_business_handoff_receipts;
         DROP TABLE open_commerce_business_handoff_receipts;
         ALTER TABLE open_commerce_business_handoff_receipts_v136
           RENAME TO open_commerce_business_handoff_receipts;
         CREATE INDEX idx_open_commerce_handoff_merchant_time
           ON open_commerce_business_handoff_receipts(
             project_id, merchant_id, created_at DESC
           );
         CREATE INDEX idx_open_commerce_handoff_invocation
           ON open_commerce_business_handoff_receipts(invocation_id, created_at DESC);",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_adds_adapter_credentials_and_receipt_provenance() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE projects(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        crate::open_commerce_migration::migration_v108(&conn).unwrap();
        crate::open_commerce_integration_migration::migration_v109(&conn).unwrap();
        crate::open_commerce_developer_event_migration::migration_v134(&conn).unwrap();
        crate::open_commerce_business_handoff_migration::migration_v135(&conn).unwrap();
        migration_v136(&conn).unwrap();
        let credentials: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type='table' AND name='open_commerce_adapter_credentials'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(credentials, 1);
        let has_adapter_column: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('open_commerce_business_handoff_receipts')
                 WHERE name='adapter_credential_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(has_adapter_column, 1);
    }
}
