use anyhow::Result;
use rusqlite::Connection;

#[path = "esk_asset_batch_migration.rs"]
mod batch;
pub(crate) use batch::migration_v282;
#[path = "esk_quant_allocation_migration.rs"]
mod quant_allocation;
pub(crate) use quant_allocation::migration_v284;
#[path = "esk_quant_allocation_binding_migration.rs"]
mod quant_allocation_binding;
pub(crate) use quant_allocation_binding::migration_v285;

pub(crate) fn migration_v281(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS esk_asset_ledger_entries (
           entry_id          TEXT PRIMARY KEY,
           user_id           TEXT NOT NULL,
           amount_base_units INTEGER NOT NULL CHECK(amount_base_units > 0),
           entry_kind        TEXT NOT NULL CHECK(entry_kind = 'paper_allocation'),
           reference         TEXT NOT NULL CHECK(length(trim(reference)) > 0),
           idempotency_key   TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           actor             TEXT NOT NULL CHECK(actor = 'platform_admin'),
           created_at        TEXT NOT NULL,
           UNIQUE(entry_kind, idempotency_key),
           FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_esk_asset_ledger_user_time
           ON esk_asset_ledger_entries(user_id, created_at, entry_id);

         CREATE TABLE IF NOT EXISTS esk_sellback_requests (
           request_id        TEXT PRIMARY KEY,
           user_id           TEXT NOT NULL,
           amount_base_units INTEGER NOT NULL CHECK(amount_base_units > 0),
           idempotency_key   TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           submitted_at      TEXT NOT NULL,
           UNIQUE(user_id, idempotency_key),
           FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_esk_sellback_requests_user_time
           ON esk_sellback_requests(user_id, submitted_at DESC, request_id DESC);

         CREATE TABLE IF NOT EXISTS esk_sellback_request_events (
           event_id          TEXT PRIMARY KEY,
           request_id        TEXT NOT NULL,
           revision          INTEGER NOT NULL CHECK(revision > 0),
           status            TEXT NOT NULL CHECK(status IN ('submitted', 'canceled')),
           actor_user_id     TEXT NOT NULL CHECK(length(trim(actor_user_id)) > 0),
           created_at        TEXT NOT NULL,
           UNIQUE(request_id, revision),
           UNIQUE(request_id, status),
           FOREIGN KEY(request_id) REFERENCES esk_sellback_requests(request_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_esk_sellback_events_request_revision
           ON esk_sellback_request_events(request_id, revision DESC);

         CREATE TRIGGER IF NOT EXISTS trg_esk_asset_ledger_no_update
         BEFORE UPDATE ON esk_asset_ledger_entries BEGIN
           SELECT RAISE(ABORT, 'ESK asset ledger is append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_asset_ledger_no_delete
         BEFORE DELETE ON esk_asset_ledger_entries BEGIN
           SELECT RAISE(ABORT, 'ESK asset ledger is append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_sellback_requests_no_update
         BEFORE UPDATE ON esk_sellback_requests BEGIN
           SELECT RAISE(ABORT, 'ESK sellback requests are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_sellback_requests_no_delete
         BEFORE DELETE ON esk_sellback_requests BEGIN
           SELECT RAISE(ABORT, 'ESK sellback requests are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_sellback_events_no_update
         BEFORE UPDATE ON esk_sellback_request_events BEGIN
           SELECT RAISE(ABORT, 'ESK sellback events are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_sellback_events_no_delete
         BEFORE DELETE ON esk_sellback_request_events BEGIN
           SELECT RAISE(ABORT, 'ESK sellback events are append-only');
         END;",
    )?;
    Ok(())
}
