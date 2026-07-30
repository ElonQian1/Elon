use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v110(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS task_economy_project_settings (
           project_id TEXT PRIMARY KEY REFERENCES projects(id),
           enabled INTEGER NOT NULL DEFAULT 0 CHECK(enabled IN (0, 1)),
           shadow_only INTEGER NOT NULL DEFAULT 1 CHECK(shadow_only = 1),
           updated_by_user_id TEXT REFERENCES users(id),
           updated_at TEXT
         );

         CREATE TABLE IF NOT EXISTS task_usage_receipts (
           id TEXT PRIMARY KEY,
           project_id TEXT NOT NULL REFERENCES projects(id),
           subject_type TEXT NOT NULL,
           subject_id TEXT NOT NULL,
           source_type TEXT NOT NULL,
           source_id TEXT NOT NULL,
           source_digest TEXT NOT NULL,
           consumer_user_id TEXT NOT NULL REFERENCES users(id),
           provider_user_id TEXT REFERENCES users(id),
           units INTEGER NOT NULL DEFAULT 0 CHECK(units >= 0),
           amount_micros INTEGER NOT NULL DEFAULT 0 CHECK(amount_micros >= 0),
           provider_amount_micros INTEGER NOT NULL DEFAULT 0
             CHECK(provider_amount_micros >= 0 AND provider_amount_micros <= amount_micros),
           currency TEXT NOT NULL,
           billing_source TEXT NOT NULL,
           source_status TEXT NOT NULL,
           occurred_at TEXT NOT NULL,
           created_at TEXT NOT NULL,
           UNIQUE(project_id, source_type, source_id)
         );
         CREATE INDEX IF NOT EXISTS idx_task_usage_receipts_project_created
           ON task_usage_receipts(project_id, created_at DESC);
         CREATE INDEX IF NOT EXISTS idx_task_usage_receipts_subject
           ON task_usage_receipts(project_id, subject_type, subject_id);

         CREATE TABLE IF NOT EXISTS task_settlement_intents (
           id TEXT PRIMARY KEY,
           project_id TEXT NOT NULL REFERENCES projects(id),
           matter_id TEXT,
           assignment_id TEXT,
           payer_user_id TEXT NOT NULL REFERENCES users(id),
           payee_user_id TEXT REFERENCES users(id),
           idempotency_key TEXT NOT NULL,
           policy_version TEXT NOT NULL,
           policy_digest TEXT NOT NULL,
           status TEXT NOT NULL DEFAULT 'pending'
             CHECK(status IN ('pending', 'posted', 'voided')),
           shadow_only INTEGER NOT NULL DEFAULT 1 CHECK(shadow_only = 1),
           created_at TEXT NOT NULL,
           updated_at TEXT NOT NULL,
           UNIQUE(project_id, idempotency_key)
         );
         CREATE INDEX IF NOT EXISTS idx_task_settlement_intents_matter
           ON task_settlement_intents(project_id, matter_id, status);

         CREATE TABLE IF NOT EXISTS task_settlement_intent_sources (
           intent_id TEXT NOT NULL REFERENCES task_settlement_intents(id),
           usage_receipt_id TEXT NOT NULL REFERENCES task_usage_receipts(id),
           created_at TEXT NOT NULL,
           PRIMARY KEY(intent_id, usage_receipt_id)
         );

         CREATE TABLE IF NOT EXISTS task_settlement_receipts (
           id TEXT PRIMARY KEY,
           project_id TEXT NOT NULL REFERENCES projects(id),
           intent_id TEXT NOT NULL REFERENCES task_settlement_intents(id),
           posting_key TEXT NOT NULL,
           status TEXT NOT NULL CHECK(status IN ('reconciled', 'voided')),
           compute_amount_micros INTEGER NOT NULL DEFAULT 0 CHECK(compute_amount_micros >= 0),
           provider_amount_micros INTEGER NOT NULL DEFAULT 0 CHECK(provider_amount_micros >= 0),
           platform_amount_micros INTEGER NOT NULL DEFAULT 0 CHECK(platform_amount_micros >= 0),
           outcome_reward_micros INTEGER NOT NULL DEFAULT 0 CHECK(outcome_reward_micros >= 0),
           review_reward_micros INTEGER NOT NULL DEFAULT 0 CHECK(review_reward_micros >= 0),
           currency TEXT NOT NULL,
           shadow_only INTEGER NOT NULL DEFAULT 1 CHECK(shadow_only = 1),
           accepted_matter_id TEXT,
           reason TEXT NOT NULL,
           created_at TEXT NOT NULL,
           UNIQUE(project_id, posting_key),
           UNIQUE(intent_id)
         );
         CREATE INDEX IF NOT EXISTS idx_task_settlement_receipts_project_created
           ON task_settlement_receipts(project_id, created_at DESC);

         CREATE TABLE IF NOT EXISTS task_ledger_transactions (
           id TEXT PRIMARY KEY,
           project_id TEXT NOT NULL REFERENCES projects(id),
           settlement_receipt_id TEXT NOT NULL UNIQUE REFERENCES task_settlement_receipts(id),
           posting_key TEXT NOT NULL UNIQUE,
           description TEXT NOT NULL,
           created_at TEXT NOT NULL
         );

         CREATE TABLE IF NOT EXISTS task_ledger_entries (
           id TEXT PRIMARY KEY,
           transaction_id TEXT NOT NULL REFERENCES task_ledger_transactions(id),
           account_key TEXT NOT NULL,
           user_id TEXT REFERENCES users(id),
           side TEXT NOT NULL CHECK(side IN ('debit', 'credit')),
           amount_micros INTEGER NOT NULL CHECK(amount_micros > 0),
           currency TEXT NOT NULL,
           created_at TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_task_ledger_entries_transaction
           ON task_ledger_entries(transaction_id);",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_all_shadow_settlement_tables() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE users(id TEXT PRIMARY KEY);
             CREATE TABLE projects(id TEXT PRIMARY KEY);",
        )
        .unwrap();
        migration_v110(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                  WHERE type = 'table'
                    AND name IN (
                      'task_economy_project_settings',
                      'task_usage_receipts',
                      'task_settlement_intents',
                      'task_settlement_intent_sources',
                      'task_settlement_receipts',
                      'task_ledger_transactions',
                      'task_ledger_entries'
                    )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 7);
    }
}
