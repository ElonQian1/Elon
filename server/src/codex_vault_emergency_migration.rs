use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v91(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS codex_vault_emergency_grants (
          id                 TEXT PRIMARY KEY,
          provider_user_id   TEXT NOT NULL,
          consumer_user_id   TEXT NOT NULL,
          status             TEXT NOT NULL DEFAULT 'active'
                             CHECK (status IN ('active', 'revoked')),
          label              TEXT,
          purpose            TEXT,
          max_lease_seconds  INTEGER NOT NULL DEFAULT 900,
          expires_at         TEXT,
          created_by_user_id TEXT NOT NULL,
          created_at         TEXT NOT NULL,
          updated_at         TEXT NOT NULL,
          revoked_at         TEXT,
          CHECK (provider_user_id != consumer_user_id),
          FOREIGN KEY (provider_user_id) REFERENCES users(id),
          FOREIGN KEY (consumer_user_id) REFERENCES users(id),
          FOREIGN KEY (created_by_user_id) REFERENCES users(id)
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_codex_vault_emergency_grants_active_pair
          ON codex_vault_emergency_grants(provider_user_id, consumer_user_id)
          WHERE status = 'active';

        CREATE INDEX IF NOT EXISTS idx_codex_vault_emergency_grants_provider
          ON codex_vault_emergency_grants(provider_user_id, status, updated_at DESC);

        CREATE INDEX IF NOT EXISTS idx_codex_vault_emergency_grants_consumer
          ON codex_vault_emergency_grants(consumer_user_id, status, updated_at DESC);

        CREATE TABLE IF NOT EXISTS codex_vault_emergency_leases (
          id                    TEXT PRIMARY KEY,
          grant_id              TEXT NOT NULL,
          provider_user_id      TEXT NOT NULL,
          consumer_user_id      TEXT NOT NULL,
          consumer_node_id      TEXT NOT NULL,
          provider_slot_id      TEXT NOT NULL,
          account_hint_hash     TEXT,
          purpose               TEXT,
          failure_reason        TEXT,
          billing_source        TEXT NOT NULL DEFAULT 'shared_codex',
          status                TEXT NOT NULL DEFAULT 'active'
                                CHECK (status IN ('active', 'cleared', 'expired')),
          leased_at             TEXT NOT NULL,
          expires_at            TEXT NOT NULL,
          cleared_at            TEXT,
          token_usage_event_id  TEXT,
          billing_event_id      TEXT,
          node_transaction_id   TEXT,
          input_tokens          INTEGER NOT NULL DEFAULT 0,
          output_tokens         INTEGER NOT NULL DEFAULT 0,
          total_tokens          INTEGER NOT NULL DEFAULT 0,
          billed_cost_rmb_fen   INTEGER NOT NULL DEFAULT 0,
          provider_earned_fen   INTEGER NOT NULL DEFAULT 0,
          accounting_status     TEXT,
          created_at            TEXT NOT NULL,
          updated_at            TEXT NOT NULL,
          FOREIGN KEY (grant_id) REFERENCES codex_vault_emergency_grants(id),
          FOREIGN KEY (provider_user_id) REFERENCES users(id),
          FOREIGN KEY (consumer_user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_codex_vault_emergency_leases_consumer_node
          ON codex_vault_emergency_leases(consumer_user_id, consumer_node_id, status, expires_at DESC);

        CREATE INDEX IF NOT EXISTS idx_codex_vault_emergency_leases_provider_time
          ON codex_vault_emergency_leases(provider_user_id, leased_at DESC);

        CREATE INDEX IF NOT EXISTS idx_codex_vault_emergency_leases_consumer_time
          ON codex_vault_emergency_leases(consumer_user_id, leased_at DESC);
        "#,
    )?;
    Ok(())
}
