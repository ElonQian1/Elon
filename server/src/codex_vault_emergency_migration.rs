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

pub(crate) fn migration_v92(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS codex_vault_emergency_lease_usage_events (
          id                    TEXT PRIMARY KEY,
          lease_id              TEXT NOT NULL,
          token_usage_event_id  TEXT NOT NULL,
          billing_event_id      TEXT,
          node_transaction_id   TEXT,
          input_tokens          INTEGER NOT NULL DEFAULT 0,
          output_tokens         INTEGER NOT NULL DEFAULT 0,
          total_tokens          INTEGER NOT NULL DEFAULT 0,
          billed_cost_rmb_fen   INTEGER NOT NULL DEFAULT 0,
          provider_earned_fen   INTEGER NOT NULL DEFAULT 0,
          accounting_status     TEXT,
          created_at            TEXT NOT NULL,
          FOREIGN KEY (lease_id) REFERENCES codex_vault_emergency_leases(id),
          UNIQUE (lease_id, token_usage_event_id)
        );

        CREATE INDEX IF NOT EXISTS idx_codex_vault_emergency_lease_usage_lease
          ON codex_vault_emergency_lease_usage_events(lease_id, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_codex_vault_emergency_lease_usage_token
          ON codex_vault_emergency_lease_usage_events(token_usage_event_id);

        INSERT OR IGNORE INTO codex_vault_emergency_lease_usage_events
          (id, lease_id, token_usage_event_id, billing_event_id, node_transaction_id,
           input_tokens, output_tokens, total_tokens, billed_cost_rmb_fen,
           provider_earned_fen, accounting_status, created_at)
        SELECT
          'cvlu_' || lower(hex(randomblob(16))),
          id,
          token_usage_event_id,
          billing_event_id,
          node_transaction_id,
          input_tokens,
          output_tokens,
          total_tokens,
          billed_cost_rmb_fen,
          provider_earned_fen,
          accounting_status,
          updated_at
        FROM codex_vault_emergency_leases
        WHERE token_usage_event_id IS NOT NULL
          AND trim(token_usage_event_id) != '';
        "#,
    )?;
    Ok(())
}

pub(crate) fn migration_v93(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS codex_vault_usage_snapshots (
          id                       TEXT PRIMARY KEY,
          provider_user_id          TEXT NOT NULL,
          observed_by_user_id       TEXT NOT NULL,
          lease_id                  TEXT,
          account_hint_hash         TEXT,
          source                    TEXT NOT NULL DEFAULT 'codex_app_server',
          limit_id                  TEXT NOT NULL,
          limit_name                TEXT,
          plan_type                 TEXT,
          used_percent              REAL,
          remaining_percent         REAL,
          window_duration_mins      INTEGER,
          resets_at                 TEXT,
          rate_limit_reached_type   TEXT,
          credits_balance           TEXT,
          lifetime_tokens           INTEGER,
          daily_bucket_date         TEXT,
          daily_tokens              INTEGER,
          observed_at               TEXT NOT NULL,
          created_at                TEXT NOT NULL,
          FOREIGN KEY (provider_user_id) REFERENCES users(id),
          FOREIGN KEY (observed_by_user_id) REFERENCES users(id),
          FOREIGN KEY (lease_id) REFERENCES codex_vault_emergency_leases(id)
        );

        CREATE INDEX IF NOT EXISTS idx_codex_vault_usage_snapshots_provider_window
          ON codex_vault_usage_snapshots(provider_user_id, limit_id, resets_at, observed_at);

        CREATE INDEX IF NOT EXISTS idx_codex_vault_usage_snapshots_observer
          ON codex_vault_usage_snapshots(observed_by_user_id, observed_at DESC);
        "#,
    )?;
    Ok(())
}
