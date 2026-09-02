use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v284(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS esk_quant_allocation_requests (
           request_id              TEXT PRIMARY KEY,
           user_id                 TEXT NOT NULL,
           amount_base_units       INTEGER NOT NULL CHECK(amount_base_units > 0),
           idempotency_key         TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           risk_disclosure_revision TEXT NOT NULL CHECK(risk_disclosure_revision = 'esk-quant-paper-allocation-v2'),
           submitted_at            TEXT NOT NULL,
           UNIQUE(user_id, idempotency_key),
           FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_esk_quant_allocation_requests_user_time
           ON esk_quant_allocation_requests(user_id, submitted_at DESC, request_id DESC);

         CREATE TABLE IF NOT EXISTS esk_quant_allocation_request_events (
           event_id          TEXT PRIMARY KEY,
           request_id        TEXT NOT NULL,
           revision          INTEGER NOT NULL CHECK(revision > 0),
           status            TEXT NOT NULL CHECK(status IN ('submitted', 'canceled')),
           actor_user_id     TEXT NOT NULL CHECK(length(trim(actor_user_id)) > 0),
           created_at        TEXT NOT NULL,
           UNIQUE(request_id, revision),
           UNIQUE(request_id, status),
           FOREIGN KEY(request_id) REFERENCES esk_quant_allocation_requests(request_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_esk_quant_allocation_events_request_revision
           ON esk_quant_allocation_request_events(request_id, revision DESC);

         CREATE TRIGGER IF NOT EXISTS trg_esk_quant_allocation_requests_no_update
         BEFORE UPDATE ON esk_quant_allocation_requests BEGIN
           SELECT RAISE(ABORT, 'ESK quant allocation requests are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_quant_allocation_requests_no_delete
         BEFORE DELETE ON esk_quant_allocation_requests BEGIN
           SELECT RAISE(ABORT, 'ESK quant allocation requests are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_quant_allocation_events_no_update
         BEFORE UPDATE ON esk_quant_allocation_request_events BEGIN
           SELECT RAISE(ABORT, 'ESK quant allocation events are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_quant_allocation_events_no_delete
         BEFORE DELETE ON esk_quant_allocation_request_events BEGIN
           SELECT RAISE(ABORT, 'ESK quant allocation events are append-only');
         END;",
    )?;
    Ok(())
}
