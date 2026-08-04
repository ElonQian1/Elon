use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v176(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_broker_finish_receipts (
            reservation_id TEXT PRIMARY KEY,
            consumer_account_id TEXT NOT NULL CHECK (
                length(trim(consumer_account_id)) > 0
            ),
            idempotency_key TEXT NOT NULL CHECK (
                length(trim(idempotency_key)) > 0
            ),
            request_digest TEXT NOT NULL CHECK (
                length(trim(request_digest)) > 0
            ),
            terminal_action TEXT NOT NULL CHECK (
                terminal_action IN ('release', 'expire')
            ),
            budget_reservation_id TEXT NOT NULL,
            budget_terminal_status TEXT NOT NULL CHECK (
                budget_terminal_status IN ('released_no_usage', 'expired_released')
            ),
            budget_refunded_fen INTEGER NOT NULL CHECK (
                budget_refunded_fen >= 0
            ),
            job_id TEXT NOT NULL,
            source_job_revision INTEGER NOT NULL CHECK (
                source_job_revision > 0
            ),
            source_job_digest TEXT NOT NULL,
            terminal_job_revision INTEGER NOT NULL CHECK (
                terminal_job_revision > source_job_revision
            ),
            terminal_job_digest TEXT NOT NULL,
            source_claim_id TEXT NOT NULL,
            source_claim_revision INTEGER NOT NULL CHECK (
                source_claim_revision > 0
            ),
            source_claim_digest TEXT NOT NULL,
            terminal_claim_revision INTEGER NOT NULL CHECK (
                terminal_claim_revision > source_claim_revision
            ),
            terminal_claim_digest TEXT NOT NULL,
            source_reservation_revision INTEGER NOT NULL CHECK (
                source_reservation_revision > 0
            ),
            source_reservation_digest TEXT NOT NULL,
            terminal_reservation_revision INTEGER NOT NULL CHECK (
                terminal_reservation_revision > source_reservation_revision
            ),
            terminal_reservation_digest TEXT NOT NULL,
            occurred_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            UNIQUE (consumer_account_id, idempotency_key),
            FOREIGN KEY (budget_reservation_id)
                REFERENCES billing_reservations(id) ON DELETE RESTRICT,
            FOREIGN KEY (job_id, source_job_revision)
                REFERENCES compute_job_versions(job_id, revision) ON DELETE RESTRICT,
            FOREIGN KEY (job_id, terminal_job_revision)
                REFERENCES compute_job_versions(job_id, revision) ON DELETE RESTRICT,
            FOREIGN KEY (source_claim_id, source_claim_revision)
                REFERENCES compute_capacity_claim_versions(claim_id, revision) ON DELETE RESTRICT,
            FOREIGN KEY (source_claim_id, terminal_claim_revision)
                REFERENCES compute_capacity_claim_versions(claim_id, revision) ON DELETE RESTRICT,
            FOREIGN KEY (reservation_id, source_reservation_revision)
                REFERENCES compute_reservation_versions(reservation_id, revision) ON DELETE RESTRICT,
            FOREIGN KEY (reservation_id, terminal_reservation_revision)
                REFERENCES compute_reservation_versions(reservation_id, revision) ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_compute_broker_finish_recorded
            ON compute_broker_finish_receipts(recorded_at, reservation_id);

        CREATE TRIGGER IF NOT EXISTS trg_compute_broker_finish_no_update
        BEFORE UPDATE ON compute_broker_finish_receipts
        BEGIN
            SELECT RAISE(ABORT, 'compute broker finish receipts are immutable');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_broker_finish_no_delete
        BEFORE DELETE ON compute_broker_finish_receipts
        BEGIN
            SELECT RAISE(ABORT, 'compute broker finish receipts are immutable');
        END;
        "#,
    )?;
    Ok(())
}
