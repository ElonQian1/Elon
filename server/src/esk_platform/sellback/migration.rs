use anyhow::Result;
use rusqlite::Connection;

/// Independent append-only requests; no settlement, token issuance or Paper writes.
pub(crate) fn migration_v288(conn: &Connection) -> Result<()> {
    conn.execute_batch("SAVEPOINT esk_platform_sellback_v288")?;
    let result = create_tables(conn).and_then(|()| {
        for table in [
            "esk_platform_sellback_requests",
            "esk_platform_sellback_cancellations",
        ] {
            for (suffix, operation) in [("update", "UPDATE"), ("delete", "DELETE")] {
                conn.execute_batch(&format!(
                    "CREATE TRIGGER IF NOT EXISTS trg_{table}_no_{suffix}
                     BEFORE {operation} ON {table} BEGIN
                       SELECT RAISE(ABORT, 'ESK sellback records are append-only');
                     END;"
                ))?;
            }
        }
        Ok(())
    });
    if let Err(error) = result {
        conn.execute_batch(
            "ROLLBACK TO esk_platform_sellback_v288; RELEASE esk_platform_sellback_v288",
        )?;
        return Err(error);
    }
    if let Err(error) = conn.execute_batch("RELEASE esk_platform_sellback_v288") {
        let _ = conn.execute_batch(
            "ROLLBACK TO esk_platform_sellback_v288; RELEASE esk_platform_sellback_v288",
        );
        return Err(error.into());
    }
    Ok(())
}

fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS esk_platform_sellback_requests (
           request_id TEXT PRIMARY KEY NOT NULL CHECK(length(request_id) = 39),
           user_id TEXT NOT NULL,
           idempotency_key TEXT NOT NULL CHECK(length(idempotency_key) BETWEEN 1 AND 96),
           amount_base_units INTEGER NOT NULL
             CHECK(typeof(amount_base_units) = 'integer' AND amount_base_units > 0),
           request_digest TEXT NOT NULL CHECK(length(request_digest) = 64),
           input_json TEXT NOT NULL CHECK(length(input_json) <= 4096),
           policy_json TEXT NOT NULL CHECK(length(policy_json) <= 131072),
           platform_policy_digest TEXT NOT NULL,
           source_fingerprint TEXT NOT NULL CHECK(length(source_fingerprint) = 64),
           created_at TEXT NOT NULL,
           UNIQUE(user_id, idempotency_key),
           FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE RESTRICT,
           FOREIGN KEY(platform_policy_digest) REFERENCES esk_platform_policy(policy_digest) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_esk_platform_sellback_requests_user
           ON esk_platform_sellback_requests(user_id, created_at DESC, request_id DESC);
         CREATE TABLE IF NOT EXISTS esk_platform_sellback_cancellations (
           cancel_event_id TEXT PRIMARY KEY NOT NULL CHECK(length(cancel_event_id) = 39),
           request_id TEXT NOT NULL UNIQUE,
           request_digest TEXT NOT NULL CHECK(length(request_digest) = 64),
           canceled_by TEXT NOT NULL,
           created_at TEXT NOT NULL,
           FOREIGN KEY(request_id) REFERENCES esk_platform_sellback_requests(request_id) ON DELETE RESTRICT,
           FOREIGN KEY(canceled_by) REFERENCES users(id) ON DELETE RESTRICT
         );
         CREATE TRIGGER IF NOT EXISTS trg_esk_platform_sellback_request_binding
         BEFORE INSERT ON esk_platform_sellback_requests
         WHEN NOT EXISTS (
           SELECT 1 FROM esk_platform_policy p JOIN users u ON u.id = NEW.user_id
            WHERE p.policy_digest = NEW.platform_policy_digest
              AND p.source_fingerprint = NEW.source_fingerprint
              AND u.status = 'active' AND u.id <> 'local-owner'
         ) BEGIN
           SELECT RAISE(ABORT, 'ESK sellback request binding is invalid');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_platform_sellback_cancel_binding
         BEFORE INSERT ON esk_platform_sellback_cancellations
         WHEN NOT EXISTS (
           SELECT 1 FROM esk_platform_sellback_requests r JOIN users u ON u.id = NEW.canceled_by
            WHERE r.request_id = NEW.request_id AND r.user_id = NEW.canceled_by
              AND r.request_digest = NEW.request_digest
              AND u.status = 'active' AND u.id <> 'local-owner'
         ) BEGIN
           SELECT RAISE(ABORT, 'ESK sellback cancellation binding is invalid');
         END;"
    )?;
    Ok(())
}
