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

pub(crate) fn migration_v100(conn: &Connection) -> Result<()> {
    // Older builds could leave more than one active row for a consumer/node.
    // Collapse those rows deterministically before installing the invariant:
    // latest leased_at wins, and id is the stable tie breaker.
    conn.execute_batch(
        r#"
        UPDATE node_compute_runs
           SET replay_deadline = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
               updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE lease_id IN (
               SELECT id
                 FROM codex_vault_emergency_leases
                WHERE status = 'active'
                  AND cleared_at IS NULL
                  AND (
                      julianday(expires_at) IS NULL
                      OR julianday(expires_at) <= julianday('now')
                  )
           )
           AND (
               replay_deadline IS NULL
               OR julianday(replay_deadline) IS NULL
               OR julianday(replay_deadline) > julianday('now')
           );

        UPDATE codex_vault_emergency_leases
           SET status = 'expired',
               updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE status = 'active'
           AND cleared_at IS NULL
           AND (
               julianday(expires_at) IS NULL
               OR julianday(expires_at) <= julianday('now')
           );

        UPDATE node_compute_runs
           SET replay_deadline = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
               updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE lease_id IN (
               SELECT lease.id
                 FROM codex_vault_emergency_leases AS lease
                WHERE lease.status = 'active'
                  AND lease.cleared_at IS NULL
                  AND NOT EXISTS (
                      SELECT 1
                        FROM codex_vault_emergency_grants AS grant_row
                       WHERE grant_row.id = lease.grant_id
                         AND grant_row.status = 'active'
                         AND (
                             grant_row.expires_at IS NULL
                             OR (
                                 julianday(grant_row.expires_at) IS NOT NULL
                                 AND julianday(grant_row.expires_at) > julianday('now')
                             )
                         )
                  )
           )
           AND (
               replay_deadline IS NULL
               OR julianday(replay_deadline) IS NULL
               OR julianday(replay_deadline) > julianday('now')
           );

        UPDATE codex_vault_emergency_leases
           SET status = 'cleared',
               cleared_at = COALESCE(
                   cleared_at,
                   strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
               ),
               updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE status = 'active'
           AND cleared_at IS NULL
           AND NOT EXISTS (
               SELECT 1
                 FROM codex_vault_emergency_grants AS grant_row
                WHERE grant_row.id = codex_vault_emergency_leases.grant_id
                  AND grant_row.status = 'active'
                  AND (
                      grant_row.expires_at IS NULL
                      OR (
                          julianday(grant_row.expires_at) IS NOT NULL
                          AND julianday(grant_row.expires_at) > julianday('now')
                      )
                  )
           );

        UPDATE node_compute_runs
           SET replay_deadline = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
               updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE lease_id IN (
               SELECT lease.id
                 FROM codex_vault_emergency_leases AS lease
                WHERE lease.status = 'active'
                  AND lease.cleared_at IS NULL
                  AND EXISTS (
                      SELECT 1
                        FROM codex_vault_emergency_leases AS newer
                       WHERE newer.consumer_user_id = lease.consumer_user_id
                         AND newer.consumer_node_id = lease.consumer_node_id
                         AND newer.status = 'active'
                         AND newer.cleared_at IS NULL
                         AND (
                             (
                                 julianday(newer.leased_at) IS NOT NULL
                                 AND julianday(lease.leased_at) IS NULL
                             )
                             OR (
                                 julianday(newer.leased_at) > julianday(lease.leased_at)
                             )
                             OR (
                                 (
                                     julianday(newer.leased_at) = julianday(lease.leased_at)
                                     OR (
                                         julianday(newer.leased_at) IS NULL
                                         AND julianday(lease.leased_at) IS NULL
                                     )
                                 )
                                 AND newer.id > lease.id
                             )
                         )
                  )
           )
           AND (
               replay_deadline IS NULL
               OR julianday(replay_deadline) IS NULL
               OR julianday(replay_deadline) > julianday('now')
           );

        UPDATE codex_vault_emergency_leases
           SET status = 'cleared',
               cleared_at = COALESCE(
                   cleared_at,
                   strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
               ),
               updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE status = 'active'
           AND cleared_at IS NULL
           AND EXISTS (
               SELECT 1
                 FROM codex_vault_emergency_leases AS newer
                WHERE newer.consumer_user_id = codex_vault_emergency_leases.consumer_user_id
                  AND newer.consumer_node_id = codex_vault_emergency_leases.consumer_node_id
                  AND newer.status = 'active'
                  AND newer.cleared_at IS NULL
                  AND (
                      (
                          julianday(newer.leased_at) IS NOT NULL
                          AND julianday(codex_vault_emergency_leases.leased_at) IS NULL
                      )
                      OR (
                          julianday(newer.leased_at)
                              > julianday(codex_vault_emergency_leases.leased_at)
                      )
                      OR (
                          (
                              julianday(newer.leased_at)
                                  = julianday(codex_vault_emergency_leases.leased_at)
                              OR (
                                  julianday(newer.leased_at) IS NULL
                                  AND julianday(codex_vault_emergency_leases.leased_at) IS NULL
                              )
                          )
                          AND newer.id > codex_vault_emergency_leases.id
                      )
                  )
           );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_codex_vault_emergency_leases_one_active_node
          ON codex_vault_emergency_leases(consumer_user_id, consumer_node_id)
          WHERE status = 'active' AND cleared_at IS NULL;
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{migration_v100, migration_v91};
    use rusqlite::{params, Connection};

    #[test]
    fn v100_collapses_legacy_active_leases_before_unique_index() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE users (id TEXT PRIMARY KEY);
            CREATE TABLE node_compute_runs (
              id              TEXT PRIMARY KEY,
              lease_id        TEXT,
              replay_deadline TEXT,
              updated_at      TEXT NOT NULL
            );
            "#,
        )
        .unwrap();
        migration_v91(&conn).unwrap();
        conn.execute_batch(
            r#"
            INSERT INTO users (id)
            VALUES ('provider-a'), ('provider-b'), ('consumer-a');

            INSERT INTO codex_vault_emergency_grants
              (id, provider_user_id, consumer_user_id, status, max_lease_seconds,
               created_by_user_id, created_at, updated_at)
            VALUES
              ('grant-active', 'provider-a', 'consumer-a', 'active', 900,
               'provider-a', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
              ('grant-revoked', 'provider-b', 'consumer-a', 'revoked', 900,
               'provider-b', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');

            INSERT INTO codex_vault_emergency_leases
              (id, grant_id, provider_user_id, consumer_user_id, consumer_node_id,
               provider_slot_id, status, leased_at, expires_at, created_at, updated_at)
            VALUES
              ('lease-a', 'grant-active', 'provider-a', 'consumer-a', 'node-dup',
               'slot-a', 'active', '2026-02-01T00:00:00Z', '2099-01-01T00:00:00Z',
               '2026-02-01T00:00:00Z', '2026-02-01T00:00:00Z'),
              ('lease-b', 'grant-active', 'provider-a', 'consumer-a', 'node-dup',
               'slot-b', 'active', '2026-02-01T00:00:00Z', '2099-01-01T00:00:00Z',
               '2026-02-01T00:00:00Z', '2026-02-01T00:00:00Z'),
              ('lease-revoked', 'grant-revoked', 'provider-b', 'consumer-a', 'node-revoked',
               'slot-r', 'active', '2026-02-01T00:00:00Z', '2099-01-01T00:00:00Z',
               '2026-02-01T00:00:00Z', '2026-02-01T00:00:00Z'),
              ('lease-expired', 'grant-active', 'provider-a', 'consumer-a', 'node-expired',
               'slot-e', 'active', '1999-01-01T00:00:00Z', '2000-01-01T00:00:00Z',
               '1999-01-01T00:00:00Z', '1999-01-01T00:00:00Z');

            INSERT INTO node_compute_runs (id, lease_id, replay_deadline, updated_at)
            VALUES
              ('run-duplicate', 'lease-a', '2099-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
              ('run-winner', 'lease-b', '2099-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
              ('run-revoked', 'lease-revoked', NULL, '2026-01-01T00:00:00Z'),
              ('run-expired', 'lease-expired', 'invalid-deadline', '2026-01-01T00:00:00Z');
            "#,
        )
        .unwrap();

        migration_v100(&conn).unwrap();
        migration_v100(&conn).unwrap();

        let status = |lease_id: &str| -> (String, Option<String>) {
            conn.query_row(
                "SELECT status, cleared_at FROM codex_vault_emergency_leases WHERE id = ?1",
                [lease_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap()
        };
        assert_eq!(status("lease-a").0, "cleared");
        assert!(status("lease-a").1.is_some());
        assert_eq!(status("lease-b"), ("active".to_string(), None));
        assert_eq!(status("lease-revoked").0, "cleared");
        assert_eq!(status("lease-expired").0, "expired");

        let deadline = |run_id: &str| -> Option<String> {
            conn.query_row(
                "SELECT replay_deadline FROM node_compute_runs WHERE id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .unwrap()
        };
        for run_id in ["run-duplicate", "run-revoked", "run-expired"] {
            let fenced = deadline(run_id).expect("retired lease run must be fenced");
            assert_ne!(fenced, "2099-01-01T00:00:00Z");
            assert_ne!(fenced, "invalid-deadline");
            assert!(
                conn.query_row(
                    "SELECT julianday(?1) <= julianday('now')",
                    [&fenced],
                    |row| { row.get::<_, bool>(0) }
                )
                .unwrap(),
                "{run_id} should be fenced no later than migration time"
            );
        }
        assert_eq!(
            deadline("run-winner").as_deref(),
            Some("2099-01-01T00:00:00Z")
        );

        let duplicate = conn.execute(
            "INSERT INTO codex_vault_emergency_leases
             (id, grant_id, provider_user_id, consumer_user_id, consumer_node_id,
              provider_slot_id, status, leased_at, expires_at, created_at, updated_at)
             VALUES ('lease-c', 'grant-active', 'provider-a', 'consumer-a', 'node-dup',
                     'slot-c', 'active', ?1, ?2, ?1, ?1)",
            params!["2026-03-01T00:00:00Z", "2099-01-01T00:00:00Z"],
        );
        assert!(
            duplicate.is_err(),
            "partial unique index must reject a second active lease"
        );
    }
}
