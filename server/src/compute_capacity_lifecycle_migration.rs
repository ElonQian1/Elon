use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v167(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_capacity_pool_lifecycle_events (
            event_id TEXT PRIMARY KEY,
            pool_id TEXT NOT NULL,
            capacity_epoch INTEGER NOT NULL CHECK (capacity_epoch > 0),
            previous_status TEXT NOT NULL CHECK (
                previous_status IN ('registering', 'active', 'draining', 'retired', 'quarantined')
            ),
            target_status TEXT NOT NULL CHECK (
                target_status IN ('registering', 'active', 'draining', 'retired', 'quarantined')
            ),
            reason_code TEXT NOT NULL CHECK (length(trim(reason_code)) > 0),
            subject_kind TEXT NOT NULL CHECK (length(trim(subject_kind)) > 0),
            subject_id TEXT NOT NULL CHECK (length(trim(subject_id)) > 0),
            idempotency_scope TEXT NOT NULL CHECK (length(trim(idempotency_scope)) > 0),
            idempotency_key TEXT NOT NULL CHECK (length(trim(idempotency_key)) > 0),
            request_digest TEXT NOT NULL CHECK (length(trim(request_digest)) > 0),
            occurred_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            CHECK (previous_status <> target_status),
            UNIQUE (idempotency_scope, idempotency_key),
            FOREIGN KEY (pool_id)
                REFERENCES compute_capacity_pools(pool_id)
                ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_compute_capacity_pool_lifecycle_history
            ON compute_capacity_pool_lifecycle_events(
                pool_id,
                capacity_epoch,
                recorded_at,
                event_id
            );

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_pool_lifecycle_no_update
        BEFORE UPDATE ON compute_capacity_pool_lifecycle_events
        BEGIN
            SELECT RAISE(ABORT, 'compute capacity pool lifecycle events are append-only');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_pool_lifecycle_no_delete
        BEFORE DELETE ON compute_capacity_pool_lifecycle_events
        BEGIN
            SELECT RAISE(ABORT, 'compute capacity pool lifecycle events are append-only');
        END;
        "#,
    )?;
    Ok(())
}
