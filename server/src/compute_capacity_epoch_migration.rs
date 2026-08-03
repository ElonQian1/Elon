use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v168(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_capacity_pool_epoch_events (
            event_id TEXT PRIMARY KEY,
            pool_id TEXT NOT NULL,
            previous_capacity_epoch INTEGER NOT NULL CHECK (previous_capacity_epoch > 0),
            target_capacity_epoch INTEGER NOT NULL CHECK (target_capacity_epoch > 0),
            previous_status TEXT NOT NULL CHECK (previous_status = 'retired'),
            target_status TEXT NOT NULL CHECK (target_status = 'registering'),
            reason_code TEXT NOT NULL CHECK (length(trim(reason_code)) > 0),
            subject_kind TEXT NOT NULL CHECK (length(trim(subject_kind)) > 0),
            subject_id TEXT NOT NULL CHECK (length(trim(subject_id)) > 0),
            idempotency_scope TEXT NOT NULL CHECK (length(trim(idempotency_scope)) > 0),
            idempotency_key TEXT NOT NULL CHECK (length(trim(idempotency_key)) > 0),
            request_digest TEXT NOT NULL CHECK (length(trim(request_digest)) > 0),
            occurred_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            CHECK (target_capacity_epoch = previous_capacity_epoch + 1),
            UNIQUE (idempotency_scope, idempotency_key),
            UNIQUE (pool_id, previous_capacity_epoch),
            FOREIGN KEY (pool_id)
                REFERENCES compute_capacity_pools(pool_id)
                ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_compute_capacity_pool_epoch_history
            ON compute_capacity_pool_epoch_events(
                pool_id,
                target_capacity_epoch,
                recorded_at,
                event_id
            );

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_pool_epoch_no_update
        BEFORE UPDATE ON compute_capacity_pool_epoch_events
        BEGIN
            SELECT RAISE(ABORT, 'compute capacity pool epoch events are append-only');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_pool_epoch_no_delete
        BEFORE DELETE ON compute_capacity_pool_epoch_events
        BEGIN
            SELECT RAISE(ABORT, 'compute capacity pool epoch events are append-only');
        END;
        "#,
    )?;
    Ok(())
}
