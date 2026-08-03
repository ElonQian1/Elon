use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v171(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_price_snapshots (
            snapshot_id TEXT PRIMARY KEY,
            snapshot_digest TEXT NOT NULL UNIQUE CHECK (
                length(trim(snapshot_digest)) > 0
            ),
            quote_id TEXT NOT NULL UNIQUE CHECK (length(trim(quote_id)) > 0),
            pricing_mode TEXT NOT NULL CHECK (
                pricing_mode IN ('spot', 'index_locked', 'capacity_forward', 'capacity_future')
            ),
            sku_id TEXT NOT NULL CHECK (length(trim(sku_id)) > 0),
            sku_digest TEXT NOT NULL CHECK (length(trim(sku_digest)) > 0),
            provider_id TEXT NOT NULL,
            offer_id TEXT NOT NULL,
            offer_version INTEGER NOT NULL CHECK (offer_version > 0),
            offer_digest TEXT NOT NULL CHECK (length(trim(offer_digest)) > 0),
            delivery_window_id TEXT NOT NULL CHECK (
                length(trim(delivery_window_id)) > 0
            ),
            delivery_window_digest TEXT NOT NULL CHECK (
                length(trim(delivery_window_digest)) > 0
            ),
            currency TEXT NOT NULL CHECK (length(trim(currency)) > 0),
            consumer_max_amount_micros INTEGER NOT NULL CHECK (
                consumer_max_amount_micros >= 0
            ),
            provider_max_amount_micros INTEGER NOT NULL CHECK (
                provider_max_amount_micros >= 0
                AND provider_max_amount_micros <= consumer_max_amount_micros
            ),
            price_source_kind TEXT NOT NULL CHECK (
                price_source_kind IN ('trade', 'index', 'mark', 'fallback_curve')
            ),
            price_source_id TEXT NOT NULL CHECK (length(trim(price_source_id)) > 0),
            price_source_version INTEGER NOT NULL CHECK (price_source_version > 0),
            price_source_digest TEXT NOT NULL CHECK (
                length(trim(price_source_digest)) > 0
            ),
            trade_id TEXT,
            instrument_id TEXT,
            quoted_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            snapshot_json TEXT NOT NULL CHECK (length(trim(snapshot_json)) > 0),
            created_at TEXT NOT NULL,
            CHECK (quoted_at < expires_at),
            CHECK (trade_id IS NULL OR length(trim(trade_id)) > 0),
            CHECK (instrument_id IS NULL OR length(trim(instrument_id)) > 0),
            FOREIGN KEY (provider_id)
                REFERENCES compute_providers(provider_id)
                ON DELETE RESTRICT,
            FOREIGN KEY (offer_id, offer_version)
                REFERENCES compute_offer_versions(offer_id, offer_version)
                ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_compute_price_snapshots_offer
            ON compute_price_snapshots(offer_id, offer_version, expires_at, snapshot_id);

        CREATE INDEX IF NOT EXISTS idx_compute_price_snapshots_sku_expiry
            ON compute_price_snapshots(sku_id, sku_digest, expires_at, snapshot_id);

        CREATE INDEX IF NOT EXISTS idx_compute_price_snapshots_provider_expiry
            ON compute_price_snapshots(provider_id, expires_at, snapshot_id);

        CREATE TRIGGER IF NOT EXISTS trg_compute_price_snapshots_no_update
        BEFORE UPDATE ON compute_price_snapshots
        BEGIN
            SELECT RAISE(ABORT, 'compute price snapshots are immutable');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_price_snapshots_no_delete
        BEFORE DELETE ON compute_price_snapshots
        BEGIN
            SELECT RAISE(ABORT, 'compute price snapshots are immutable');
        END;
        "#,
    )?;
    Ok(())
}
