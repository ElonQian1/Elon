use anyhow::Result;
use rusqlite::Connection;

/// Re-entrant even when an outer migration runner has no transaction of its own.
pub(crate) fn migration_v287(conn: &Connection) -> Result<()> {
    conn.execute_batch("SAVEPOINT esk_platform_v287")?;
    let result = create_tables(conn).and_then(|()| {
        for table in [
            "esk_platform_policy",
            "esk_platform_allocations",
            "esk_platform_approvals",
            "esk_platform_ledger_entries",
            "esk_platform_cancellations",
        ] {
            for (suffix, operation) in [("update", "UPDATE"), ("delete", "DELETE")] {
                conn.execute_batch(&format!(
                    "CREATE TRIGGER IF NOT EXISTS trg_{table}_no_{suffix}
                     BEFORE {operation} ON {table} BEGIN
                       SELECT RAISE(ABORT, 'ESK platform records are append-only');
                     END;"
                ))?;
            }
        }
        Ok(())
    });
    if let Err(error) = result {
        conn.execute_batch("ROLLBACK TO esk_platform_v287; RELEASE esk_platform_v287")?;
        return Err(error);
    }
    if let Err(error) = conn.execute_batch("RELEASE esk_platform_v287") {
        let _ = conn.execute_batch("ROLLBACK TO esk_platform_v287; RELEASE esk_platform_v287");
        return Err(error.into());
    }
    Ok(())
}

fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS esk_platform_policy (
           singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
           policy_digest TEXT NOT NULL UNIQUE CHECK(length(policy_digest) = 64),
           source_fingerprint TEXT NOT NULL CHECK(length(source_fingerprint) = 64),
           source_json TEXT NOT NULL,
           issuance_limit_base_units INTEGER NOT NULL
             CHECK(typeof(issuance_limit_base_units) = 'integer' AND issuance_limit_base_units > 0),
           pinned_by_user_id TEXT NOT NULL,
           created_at TEXT NOT NULL,
           FOREIGN KEY(pinned_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
         );
         CREATE TABLE IF NOT EXISTS esk_platform_allocations (
           allocation_id TEXT PRIMARY KEY,
           payment_key TEXT NOT NULL CHECK(length(payment_key) = 64),
           policy_digest TEXT NOT NULL,
           user_id TEXT NOT NULL,
           amount_base_units INTEGER NOT NULL
             CHECK(typeof(amount_base_units) = 'integer' AND amount_base_units > 0),
           request_digest TEXT NOT NULL CHECK(length(request_digest) = 64),
           input_json TEXT NOT NULL,
           prepared_by TEXT NOT NULL,
           prepared_at TEXT NOT NULL,
           FOREIGN KEY(policy_digest) REFERENCES esk_platform_policy(policy_digest) ON DELETE RESTRICT,
           FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE RESTRICT,
           FOREIGN KEY(prepared_by) REFERENCES users(id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_esk_platform_allocations_user
           ON esk_platform_allocations(user_id, prepared_at, allocation_id);
         CREATE INDEX IF NOT EXISTS idx_esk_platform_allocations_payment
           ON esk_platform_allocations(payment_key);
         CREATE TABLE IF NOT EXISTS esk_platform_approvals (
           approval_id TEXT PRIMARY KEY,
           allocation_id TEXT NOT NULL UNIQUE,
           request_digest TEXT NOT NULL CHECK(length(request_digest) = 64),
           approved_by TEXT NOT NULL,
           created_at TEXT NOT NULL,
           FOREIGN KEY(allocation_id) REFERENCES esk_platform_allocations(allocation_id) ON DELETE RESTRICT,
           FOREIGN KEY(approved_by) REFERENCES users(id) ON DELETE RESTRICT
         );
         CREATE TABLE IF NOT EXISTS esk_platform_ledger_entries (
           entry_id TEXT PRIMARY KEY,
           allocation_id TEXT NOT NULL UNIQUE,
           approval_id TEXT NOT NULL UNIQUE,
           user_id TEXT NOT NULL,
           amount_base_units INTEGER NOT NULL
             CHECK(typeof(amount_base_units) = 'integer' AND amount_base_units > 0),
           created_at TEXT NOT NULL,
           FOREIGN KEY(allocation_id) REFERENCES esk_platform_allocations(allocation_id) ON DELETE RESTRICT,
           FOREIGN KEY(approval_id) REFERENCES esk_platform_approvals(approval_id) ON DELETE RESTRICT,
           FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_esk_platform_ledger_user
           ON esk_platform_ledger_entries(user_id, created_at DESC, entry_id DESC);
         CREATE TABLE IF NOT EXISTS esk_platform_cancellations (
           allocation_id TEXT PRIMARY KEY NOT NULL,
           request_digest TEXT NOT NULL CHECK(length(request_digest) = 64),
           canceled_by TEXT NOT NULL,
           created_at TEXT NOT NULL,
           FOREIGN KEY(allocation_id) REFERENCES esk_platform_allocations(allocation_id) ON DELETE RESTRICT,
           FOREIGN KEY(canceled_by) REFERENCES users(id) ON DELETE RESTRICT
         );
         CREATE TRIGGER IF NOT EXISTS trg_esk_platform_payment_current
         BEFORE INSERT ON esk_platform_allocations
         WHEN EXISTS (
           SELECT 1 FROM esk_platform_allocations a WHERE a.payment_key = NEW.payment_key
             AND NOT EXISTS (SELECT 1 FROM esk_platform_cancellations c WHERE c.allocation_id = a.allocation_id)
         ) BEGIN
           SELECT RAISE(ABORT, 'ESK platform payment already has a current allocation');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_platform_approval_binding
         BEFORE INSERT ON esk_platform_approvals
         WHEN NOT EXISTS (
           SELECT 1 FROM esk_platform_allocations a JOIN users u ON u.id = NEW.approved_by
            WHERE a.allocation_id = NEW.allocation_id AND a.request_digest = NEW.request_digest
              AND u.status = 'active' AND u.role IN ('admin', 'owner')
              AND NOT EXISTS (SELECT 1 FROM esk_platform_cancellations c WHERE c.allocation_id = a.allocation_id)
         ) BEGIN
           SELECT RAISE(ABORT, 'ESK platform approval binding is invalid');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_platform_entry_binding
         BEFORE INSERT ON esk_platform_ledger_entries
         WHEN NOT EXISTS (
           SELECT 1 FROM esk_platform_allocations a
             JOIN esk_platform_approvals p ON p.allocation_id = a.allocation_id
             JOIN users u ON u.id = a.user_id
            WHERE a.allocation_id = NEW.allocation_id AND a.user_id = NEW.user_id
              AND a.amount_base_units = NEW.amount_base_units
              AND p.approval_id = NEW.approval_id AND p.request_digest = a.request_digest
              AND p.created_at = NEW.created_at AND u.status = 'active'
              AND NOT EXISTS (SELECT 1 FROM esk_platform_cancellations c WHERE c.allocation_id = a.allocation_id)
         ) BEGIN
           SELECT RAISE(ABORT, 'ESK platform entry binding is invalid');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_platform_cancellation_binding
         BEFORE INSERT ON esk_platform_cancellations
         WHEN NOT EXISTS (
           SELECT 1 FROM esk_platform_allocations a JOIN users u ON u.id = NEW.canceled_by
            WHERE a.allocation_id = NEW.allocation_id AND a.request_digest = NEW.request_digest
              AND u.status = 'active' AND u.role IN ('admin', 'owner')
              AND NOT EXISTS (SELECT 1 FROM esk_platform_approvals p WHERE p.allocation_id = a.allocation_id)
              AND NOT EXISTS (SELECT 1 FROM esk_platform_ledger_entries l WHERE l.allocation_id = a.allocation_id)
         ) BEGIN
           SELECT RAISE(ABORT, 'ESK platform cancellation binding is invalid');
         END;",
    )?;
    Ok(())
}
