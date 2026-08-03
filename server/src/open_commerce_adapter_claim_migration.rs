use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v138(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_business_handoff_claims (
           id                          TEXT PRIMARY KEY,
           project_id                  TEXT NOT NULL,
           merchant_id                 TEXT NOT NULL,
           invocation_id               TEXT NOT NULL,
           integration_id              TEXT NOT NULL,
           adapter_credential_id        TEXT NOT NULL,
           adapter_credential_version   INTEGER NOT NULL CHECK(adapter_credential_version > 0),
           attempt_no                   INTEGER NOT NULL CHECK(attempt_no > 0),
           status                       TEXT NOT NULL
                                        CHECK(status IN ('active', 'completed', 'expired')),
           lease_token_hash             TEXT NOT NULL UNIQUE,
           lease_token_hint             TEXT NOT NULL,
           lease_expires_at             TEXT NOT NULL,
           lease_deadline_at            TEXT NOT NULL,
           release_reason_code           TEXT,
           released_at                   TEXT,
           completion_status             TEXT
                                        CHECK(completion_status IS NULL OR completion_status IN (
                                          'applied', 'ignored', 'rejected'
                                        )),
           retry_not_before               TEXT,
           retry_suspended_at              TEXT,
           retry_suspension_reason         TEXT,
           retry_resumed_at                TEXT,
           retry_resumed_by_user_id        TEXT,
           completed_receipt_id          TEXT,
           created_at                   TEXT NOT NULL,
           updated_at                   TEXT NOT NULL,
           UNIQUE(invocation_id, attempt_no),
           FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
           FOREIGN KEY(merchant_id) REFERENCES open_commerce_merchants(id) ON DELETE CASCADE,
           FOREIGN KEY(invocation_id) REFERENCES open_commerce_invocations(id) ON DELETE CASCADE,
           FOREIGN KEY(integration_id) REFERENCES open_commerce_integrations(id) ON DELETE CASCADE,
           FOREIGN KEY(adapter_credential_id)
             REFERENCES open_commerce_adapter_credentials(id)
         );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_open_commerce_handoff_claim_active
           ON open_commerce_business_handoff_claims(invocation_id)
          WHERE status='active';
         CREATE INDEX IF NOT EXISTS idx_open_commerce_handoff_claim_adapter
           ON open_commerce_business_handoff_claims(
             integration_id, status, lease_expires_at
           );",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_business_handoff_receipts",
        "adapter_claim_id",
        "adapter_claim_id TEXT REFERENCES open_commerce_business_handoff_claims(id)",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_business_handoff_claims",
        "lease_deadline_at",
        "lease_deadline_at TEXT",
    )?;
    conn.execute(
        "UPDATE open_commerce_business_handoff_claims
            SET lease_deadline_at=datetime(created_at, '+1 hour')
          WHERE lease_deadline_at IS NULL OR TRIM(lease_deadline_at)=''",
        [],
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_business_handoff_claims",
        "release_reason_code",
        "release_reason_code TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_business_handoff_claims",
        "released_at",
        "released_at TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_business_handoff_claims",
        "completion_status",
        "completion_status TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_business_handoff_claims",
        "retry_not_before",
        "retry_not_before TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_business_handoff_claims",
        "retry_suspended_at",
        "retry_suspended_at TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_business_handoff_claims",
        "retry_suspension_reason",
        "retry_suspension_reason TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_business_handoff_claims",
        "retry_resumed_at",
        "retry_resumed_at TEXT",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_business_handoff_claims",
        "retry_resumed_by_user_id",
        "retry_resumed_by_user_id TEXT",
    )?;
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_open_commerce_handoff_receipt_claim
           ON open_commerce_business_handoff_receipts(adapter_claim_id)
          WHERE adapter_claim_id IS NOT NULL;",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_adds_claim_lease_and_receipt_provenance() {
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
        crate::open_commerce_adapter_migration::migration_v136(&conn).unwrap();
        crate::open_commerce_adapter_expiration_migration::migration_v137(&conn).unwrap();
        migration_v138(&conn).unwrap();
        migration_v138(&conn).unwrap();

        let claims: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                  WHERE type='table' AND name='open_commerce_business_handoff_claims'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let receipt_column: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info(
                   'open_commerce_business_handoff_receipts'
                 ) WHERE name='adapter_claim_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let retry_columns: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info(
                   'open_commerce_business_handoff_claims'
                 ) WHERE name IN (
                   'completion_status', 'retry_not_before', 'retry_suspended_at',
                   'retry_suspension_reason', 'retry_resumed_at',
                   'retry_resumed_by_user_id'
                 )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(claims, 1);
        assert_eq!(receipt_column, 1);
        assert_eq!(retry_columns, 6);
        let lease_deadline_column: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info(
                   'open_commerce_business_handoff_claims'
                 ) WHERE name='lease_deadline_at'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(lease_deadline_column, 1);
    }
}
