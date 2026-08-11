use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_commitments_no_replace
        BEFORE INSERT ON compute_capacity_commitments
        WHEN EXISTS (
            SELECT 1 FROM compute_capacity_commitments existing
             WHERE existing.commitment_id=NEW.commitment_id
                OR existing.commitment_digest=NEW.commitment_digest
                OR existing.claim_id=NEW.claim_id
                OR existing.hold_transaction_id=NEW.hold_transaction_id
                OR (existing.idempotency_scope=NEW.idempotency_scope
                    AND existing.idempotency_key=NEW.idempotency_key)
        )
        BEGIN
            SELECT RAISE(ABORT, 'capacity commitment cannot replace history');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_commitments_no_update
        BEFORE UPDATE ON compute_capacity_commitments
        BEGIN
            SELECT RAISE(ABORT, 'capacity commitments are immutable');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_commitments_no_delete
        BEFORE DELETE ON compute_capacity_commitments
        BEGIN
            SELECT RAISE(ABORT, 'capacity commitments are immutable');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_commitment_terminal_no_replace
        BEFORE INSERT ON compute_capacity_commitment_terminal_receipts
        WHEN EXISTS (
            SELECT 1 FROM compute_capacity_commitment_terminal_receipts existing
             WHERE existing.terminal_receipt_id=NEW.terminal_receipt_id
                OR existing.terminal_receipt_digest=NEW.terminal_receipt_digest
                OR existing.commitment_id=NEW.commitment_id
                OR existing.terminal_transaction_id=NEW.terminal_transaction_id
                OR (existing.idempotency_scope=NEW.idempotency_scope
                    AND existing.idempotency_key=NEW.idempotency_key)
        )
        BEGIN
            SELECT RAISE(ABORT, 'capacity commitment terminal cannot replace history');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_commitment_terminal_no_update
        BEFORE UPDATE ON compute_capacity_commitment_terminal_receipts
        BEGIN
            SELECT RAISE(ABORT, 'capacity commitment terminal receipts are immutable');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_commitment_terminal_no_delete
        BEFORE DELETE ON compute_capacity_commitment_terminal_receipts
        BEGIN
            SELECT RAISE(ABORT, 'capacity commitment terminal receipts are immutable');
        END;
        "#,
    )?;
    Ok(())
}
