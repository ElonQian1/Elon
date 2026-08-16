//! Durable, bounded storage for cache fleet health history.

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn migration_v275(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS node_rust_cache_fleet_reports (
           envelope_id          TEXT PRIMARY KEY,
           node_id              TEXT NOT NULL,
           owner_user_id        TEXT NOT NULL,
           report_sha256        TEXT NOT NULL,
           report_json          TEXT NOT NULL,
           platform_health      TEXT NOT NULL,
           gc_review_recommended INTEGER NOT NULL CHECK(gc_review_recommended IN (0, 1)),
           active_writer_count  INTEGER NOT NULL CHECK(active_writer_count >= 0),
           managed_size_bytes   INTEGER CHECK(managed_size_bytes IS NULL OR managed_size_bytes >= 0),
           generated_at         TEXT NOT NULL,
           received_at          TEXT NOT NULL,
           CHECK(length(envelope_id) = 32),
           CHECK(length(report_sha256) = 64),
           CHECK(length(report_json) <= 1048576),
           UNIQUE(node_id, report_sha256)
         );
         CREATE INDEX IF NOT EXISTS idx_node_rust_cache_reports_owner_node_received
           ON node_rust_cache_fleet_reports(owner_user_id, node_id, received_at DESC);
         CREATE INDEX IF NOT EXISTS idx_node_rust_cache_reports_node_received
           ON node_rust_cache_fleet_reports(node_id, received_at DESC);",
    )?;
    Ok(())
}
