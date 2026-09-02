use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v286(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS esk_exchange_quotes (
           quote_id               TEXT PRIMARY KEY,
           user_id                TEXT NOT NULL,
           direction              TEXT NOT NULL CHECK(direction IN ('usdt_to_esk', 'esk_to_usdt')),
           input_units            INTEGER NOT NULL CHECK(input_units > 0),
           price_units            INTEGER NOT NULL CHECK(price_units > 0),
           fee_bps                INTEGER NOT NULL CHECK(fee_bps BETWEEN 0 AND 1000),
           config_revision        TEXT NOT NULL CHECK(length(config_revision) = 64),
           gross_output_units     INTEGER NOT NULL CHECK(gross_output_units > 0),
           fee_units              INTEGER NOT NULL CHECK(fee_units >= 0),
           net_output_units       INTEGER NOT NULL CHECK(net_output_units > 0),
           created_at             TEXT NOT NULL,
           expires_at             TEXT NOT NULL,
           CHECK(gross_output_units = fee_units + net_output_units),
           FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_esk_exchange_quotes_user_time
           ON esk_exchange_quotes(user_id, created_at DESC, quote_id DESC);

         CREATE TABLE IF NOT EXISTS esk_exchange_executions (
           execution_id           TEXT PRIMARY KEY,
           quote_id               TEXT NOT NULL UNIQUE,
           user_id                TEXT NOT NULL,
           idempotency_key        TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           executed_at            TEXT NOT NULL,
           UNIQUE(user_id, idempotency_key),
           FOREIGN KEY(quote_id) REFERENCES esk_exchange_quotes(quote_id) ON DELETE RESTRICT,
           FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_esk_exchange_executions_user_time
           ON esk_exchange_executions(user_id, executed_at DESC, execution_id DESC);

         CREATE TABLE IF NOT EXISTS esk_exchange_ledger_entries (
           entry_id               TEXT PRIMARY KEY,
           owner_kind             TEXT NOT NULL CHECK(owner_kind IN ('user', 'platform')),
           user_id                TEXT,
           asset                  TEXT NOT NULL CHECK(asset IN ('ESK', 'USDT')),
           amount_units           INTEGER NOT NULL CHECK(amount_units <> 0),
           entry_kind             TEXT NOT NULL CHECK(entry_kind IN (
             'paper_usdt_credit', 'paper_credit_offset',
             'exchange_user_debit', 'exchange_market_credit',
             'exchange_market_debit', 'exchange_user_credit', 'platform_fee'
           )),
           posting_group_id       TEXT NOT NULL CHECK(length(trim(posting_group_id)) > 0),
           reference              TEXT NOT NULL CHECK(length(trim(reference)) > 0),
           idempotency_key        TEXT,
           created_at             TEXT NOT NULL,
           CHECK(
             (owner_kind = 'user' AND user_id IS NOT NULL) OR
             (owner_kind = 'platform' AND user_id IS NULL)
           ),
           FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE RESTRICT
         );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_esk_exchange_ledger_idempotency
           ON esk_exchange_ledger_entries(idempotency_key)
           WHERE idempotency_key IS NOT NULL;
         CREATE INDEX IF NOT EXISTS idx_esk_exchange_ledger_user_asset_time
           ON esk_exchange_ledger_entries(user_id, asset, created_at, entry_id);
         CREATE INDEX IF NOT EXISTS idx_esk_exchange_ledger_group
           ON esk_exchange_ledger_entries(posting_group_id, entry_id);

         CREATE TRIGGER IF NOT EXISTS trg_esk_exchange_quotes_no_update
         BEFORE UPDATE ON esk_exchange_quotes BEGIN
           SELECT RAISE(ABORT, 'ESK exchange quotes are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_exchange_quotes_no_delete
         BEFORE DELETE ON esk_exchange_quotes BEGIN
           SELECT RAISE(ABORT, 'ESK exchange quotes are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_exchange_executions_no_update
         BEFORE UPDATE ON esk_exchange_executions BEGIN
           SELECT RAISE(ABORT, 'ESK exchange executions are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_exchange_executions_no_delete
         BEFORE DELETE ON esk_exchange_executions BEGIN
           SELECT RAISE(ABORT, 'ESK exchange executions are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_exchange_ledger_no_update
         BEFORE UPDATE ON esk_exchange_ledger_entries BEGIN
           SELECT RAISE(ABORT, 'ESK exchange ledger is append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_esk_exchange_ledger_no_delete
         BEFORE DELETE ON esk_exchange_ledger_entries BEGIN
           SELECT RAISE(ABORT, 'ESK exchange ledger is append-only');
         END;",
    )?;
    Ok(())
}
