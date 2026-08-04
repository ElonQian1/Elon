use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v175(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_broker_reserve_receipts (
            reservation_id TEXT PRIMARY KEY CHECK (
                length(trim(reservation_id)) > 0
            ),
            consumer_account_id TEXT NOT NULL CHECK (
                length(trim(consumer_account_id)) > 0
            ),
            idempotency_key TEXT NOT NULL CHECK (
                length(trim(idempotency_key)) > 0
            ),
            request_digest TEXT NOT NULL CHECK (
                length(trim(request_digest)) > 0
            ),
            budget_adapter TEXT NOT NULL CHECK (
                budget_adapter = 'platform_balance_cny'
            ),
            budget_reservation_id TEXT NOT NULL UNIQUE,
            budget_reserved_fen INTEGER NOT NULL CHECK (
                budget_reserved_fen >= 0
            ),
            capacity_claim_id TEXT NOT NULL UNIQUE,
            capacity_claim_revision INTEGER NOT NULL CHECK (
                capacity_claim_revision > 0
            ),
            capacity_claim_digest TEXT NOT NULL CHECK (
                length(trim(capacity_claim_digest)) > 0
            ),
            job_id TEXT NOT NULL CHECK (length(trim(job_id)) > 0),
            source_job_revision INTEGER NOT NULL CHECK (
                source_job_revision > 0
            ),
            source_job_digest TEXT NOT NULL CHECK (
                length(trim(source_job_digest)) > 0
            ),
            reserved_job_revision INTEGER NOT NULL CHECK (
                reserved_job_revision > source_job_revision
            ),
            reserved_job_digest TEXT NOT NULL CHECK (
                length(trim(reserved_job_digest)) > 0
            ),
            reservation_revision INTEGER NOT NULL CHECK (
                reservation_revision > 0
            ),
            reservation_digest TEXT NOT NULL CHECK (
                length(trim(reservation_digest)) > 0
            ),
            created_at TEXT NOT NULL,
            UNIQUE (consumer_account_id, idempotency_key),
            FOREIGN KEY (budget_reservation_id)
                REFERENCES billing_reservations(id)
                ON DELETE RESTRICT,
            FOREIGN KEY (capacity_claim_id, capacity_claim_revision)
                REFERENCES compute_capacity_claim_versions(claim_id, revision)
                ON DELETE RESTRICT,
            FOREIGN KEY (job_id, source_job_revision)
                REFERENCES compute_job_versions(job_id, revision)
                ON DELETE RESTRICT,
            FOREIGN KEY (job_id, reserved_job_revision)
                REFERENCES compute_job_versions(job_id, revision)
                ON DELETE RESTRICT,
            FOREIGN KEY (reservation_id, reservation_revision)
                REFERENCES compute_reservation_versions(reservation_id, revision)
                ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_compute_broker_reserve_job
            ON compute_broker_reserve_receipts(job_id, created_at);

        CREATE TRIGGER IF NOT EXISTS trg_compute_broker_reserve_no_update
        BEFORE UPDATE ON compute_broker_reserve_receipts
        BEGIN
            SELECT RAISE(ABORT, 'compute broker reserve receipts are immutable');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_broker_reserve_no_delete
        BEFORE DELETE ON compute_broker_reserve_receipts
        BEGIN
            SELECT RAISE(ABORT, 'compute broker reserve receipts are immutable');
        END;
        "#,
    )?;
    Ok(())
}
