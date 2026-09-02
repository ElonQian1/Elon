use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v285(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS esk_quant_allocation_binding_events (
           event_id               TEXT PRIMARY KEY,
           request_id             TEXT NOT NULL,
           revision               INTEGER NOT NULL CHECK(revision > 1),
           status                 TEXT NOT NULL CHECK(status IN ('accepted', 'released')),
           actor_user_id          TEXT NOT NULL CHECK(length(trim(actor_user_id)) > 0),
           binding_id             TEXT NOT NULL CHECK(length(binding_id) = 40),
           receipt_id             TEXT NOT NULL CHECK(length(receipt_id) = 40),
           receipt_digest         TEXT NOT NULL CHECK(length(receipt_digest) = 71),
           receipt_key_id         TEXT NOT NULL CHECK(length(trim(receipt_key_id)) > 0),
           quant_binding_revision INTEGER NOT NULL CHECK(quant_binding_revision IN (1, 2)),
           occurred_at_unix       INTEGER NOT NULL CHECK(occurred_at_unix > 0),
           created_at             TEXT NOT NULL,
           CHECK((status = 'accepted' AND revision = 2 AND quant_binding_revision = 1)
              OR (status = 'released' AND revision = 3 AND quant_binding_revision = 2)),
           UNIQUE(request_id, revision),
           UNIQUE(request_id, status),
           UNIQUE(receipt_digest),
           UNIQUE(receipt_id),
           FOREIGN KEY(request_id) REFERENCES esk_quant_allocation_requests(request_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_esk_quant_binding_events_request_revision
           ON esk_quant_allocation_binding_events(request_id, revision DESC);
         CREATE TRIGGER IF NOT EXISTS trg_esk_quant_binding_events_no_update
         BEFORE UPDATE ON esk_quant_allocation_binding_events BEGIN
           SELECT RAISE(ABORT, 'ESK quant binding events are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_quant_binding_events_no_delete
         BEFORE DELETE ON esk_quant_allocation_binding_events BEGIN
           SELECT RAISE(ABORT, 'ESK quant binding events are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_quant_binding_events_binding_unique
         BEFORE INSERT ON esk_quant_allocation_binding_events
         WHEN EXISTS (
           SELECT 1 FROM esk_quant_allocation_binding_events existing
            WHERE existing.binding_id = NEW.binding_id
              AND existing.request_id <> NEW.request_id
         ) BEGIN
           SELECT RAISE(ABORT, 'ESK quant binding ID already belongs to another request');
         END;

         DROP VIEW IF EXISTS esk_quant_allocation_request_state_events;
         CREATE VIEW esk_quant_allocation_request_state_events AS
         SELECT request_id, revision, status, actor_user_id, created_at,
                NULL AS binding_id, NULL AS receipt_id, NULL AS receipt_digest,
                NULL AS receipt_key_id, NULL AS quant_binding_revision,
                NULL AS occurred_at_unix
           FROM esk_quant_allocation_request_events
         UNION ALL
         SELECT request_id, revision, status, actor_user_id, created_at,
                binding_id, receipt_id, receipt_digest, receipt_key_id,
                quant_binding_revision, occurred_at_unix
           FROM esk_quant_allocation_binding_events;",
    )?;
    Ok(())
}
