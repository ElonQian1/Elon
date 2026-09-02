use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v282(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS esk_paper_allocation_batches (
           batch_id          TEXT PRIMARY KEY,
           request_digest    TEXT NOT NULL CHECK(
             length(request_digest) = 64 AND request_digest NOT GLOB '*[^0-9a-f]*'
           ),
           entry_count       INTEGER NOT NULL CHECK(entry_count BETWEEN 1 AND 100),
           total_base_units  INTEGER NOT NULL CHECK(total_base_units > 0),
           actor             TEXT NOT NULL CHECK(actor = 'platform_admin'),
           created_at        TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_esk_paper_allocation_batches_time
           ON esk_paper_allocation_batches(created_at DESC, batch_id DESC);

         CREATE TABLE IF NOT EXISTS esk_paper_allocation_batch_entries (
           batch_id          TEXT NOT NULL,
           ordinal           INTEGER NOT NULL CHECK(ordinal >= 0 AND ordinal < 100),
           ledger_entry_id   TEXT NOT NULL UNIQUE,
           PRIMARY KEY(batch_id, ordinal),
           FOREIGN KEY(batch_id)
             REFERENCES esk_paper_allocation_batches(batch_id) ON DELETE RESTRICT,
           FOREIGN KEY(ledger_entry_id)
             REFERENCES esk_asset_ledger_entries(entry_id) ON DELETE RESTRICT
         );

         CREATE TRIGGER IF NOT EXISTS trg_esk_paper_allocation_batches_no_update
         BEFORE UPDATE ON esk_paper_allocation_batches BEGIN
           SELECT RAISE(ABORT, 'ESK paper allocation batches are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_paper_allocation_batches_no_delete
         BEFORE DELETE ON esk_paper_allocation_batches BEGIN
           SELECT RAISE(ABORT, 'ESK paper allocation batches are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_paper_allocation_batch_entries_no_update
         BEFORE UPDATE ON esk_paper_allocation_batch_entries BEGIN
           SELECT RAISE(ABORT, 'ESK paper allocation batch entries are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_paper_allocation_batch_entries_no_delete
         BEFORE DELETE ON esk_paper_allocation_batch_entries BEGIN
           SELECT RAISE(ABORT, 'ESK paper allocation batch entries are append-only');
         END;",
    )?;
    Ok(())
}
