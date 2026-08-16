//! Owner-approved, digest-bound Rust cache GC request state.

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn migration_v276(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS node_rust_cache_gc_requests (
           request_id             TEXT PRIMARY KEY,
           node_id                TEXT NOT NULL,
           owner_user_id          TEXT NOT NULL,
           status                 TEXT NOT NULL CHECK(status IN (
                                      'requested', 'plan_ready', 'approved', 'rejected',
                                      'executing', 'completed', 'partial', 'failed', 'expired'
                                  )),
           force_aged             INTEGER NOT NULL CHECK(force_aged IN (0, 1)),
           workspace_only         INTEGER NOT NULL CHECK(workspace_only IN (0, 1)),
           recover_missing        INTEGER NOT NULL CHECK(recover_missing IN (0, 1)),
           shared_aliases_only    INTEGER NOT NULL CHECK(shared_aliases_only IN (0, 1)),
           plan_id                TEXT,
           plan_digest            TEXT,
           plan_summary_json      TEXT,
           result_summary_json    TEXT,
           failure_code           TEXT,
           created_at             TEXT NOT NULL,
           updated_at             TEXT NOT NULL,
           expires_at             TEXT NOT NULL,
           CHECK(length(request_id) = 32),
           CHECK(plan_id IS NULL OR length(plan_id) = 32),
           CHECK(plan_digest IS NULL OR length(plan_digest) = 64),
           CHECK(plan_summary_json IS NULL OR length(plan_summary_json) <= 131072),
           CHECK(result_summary_json IS NULL OR length(result_summary_json) <= 131072)
         );
         CREATE INDEX IF NOT EXISTS idx_node_rust_cache_gc_owner_node_updated
           ON node_rust_cache_gc_requests(owner_user_id, node_id, updated_at DESC);
         CREATE INDEX IF NOT EXISTS idx_node_rust_cache_gc_node_status_created
           ON node_rust_cache_gc_requests(node_id, status, created_at);
         CREATE UNIQUE INDEX IF NOT EXISTS idx_node_rust_cache_gc_one_active
           ON node_rust_cache_gc_requests(node_id)
           WHERE status IN ('requested','plan_ready','approved','executing');",
    )?;
    Ok(())
}
