use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v202(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_settlement_release_batch_runs (
           batch_run_id          TEXT PRIMARY KEY,
           requested_by_user_id  TEXT NOT NULL CHECK(length(trim(requested_by_user_id)) > 0),
           requested_limit       INTEGER NOT NULL CHECK(requested_limit BETWEEN 1 AND 100),
           cursor_present        INTEGER NOT NULL CHECK(cursor_present IN (0, 1)),
           request_json          TEXT NOT NULL CHECK(length(trim(request_json)) > 0),
           request_digest        TEXT NOT NULL CHECK(length(request_digest) = 64),
           candidate_page_json   TEXT NOT NULL CHECK(length(trim(candidate_page_json)) > 0),
           candidate_page_digest TEXT NOT NULL CHECK(length(candidate_page_digest) = 64),
           idempotency_scope     TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key       TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           started_at            TEXT NOT NULL,
           UNIQUE(idempotency_scope, idempotency_key)
         );
         CREATE INDEX IF NOT EXISTS idx_compute_settlement_release_batch_runs_time
           ON compute_settlement_release_batch_runs(started_at DESC, batch_run_id DESC);

         CREATE TABLE IF NOT EXISTS compute_settlement_release_batch_completions (
           batch_run_id  TEXT PRIMARY KEY,
           report_json   TEXT NOT NULL CHECK(length(trim(report_json)) > 0),
           report_digest TEXT NOT NULL CHECK(length(report_digest) = 64),
           completed_at  TEXT NOT NULL,
           FOREIGN KEY(batch_run_id)
             REFERENCES compute_settlement_release_batch_runs(batch_run_id) ON DELETE RESTRICT
         );

         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_release_batch_runs_no_update
         BEFORE UPDATE ON compute_settlement_release_batch_runs
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement release batch runs are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_release_batch_runs_no_delete
         BEFORE DELETE ON compute_settlement_release_batch_runs
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement release batch runs are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_release_batch_completions_no_update
         BEFORE UPDATE ON compute_settlement_release_batch_completions
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement release batch completions are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_settlement_release_batch_completions_no_delete
         BEFORE DELETE ON compute_settlement_release_batch_completions
         BEGIN
           SELECT RAISE(ABORT, 'compute settlement release batch completions are append-only');
         END;",
    )?;
    Ok(())
}
