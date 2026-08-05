use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v203(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_activation_plan_reviews (
           review_id                         TEXT PRIMARY KEY,
           plan_id                           TEXT NOT NULL UNIQUE,
           request_id                        TEXT NOT NULL UNIQUE,
           provider_id                       TEXT NOT NULL,
           pool_id                           TEXT NOT NULL,
           plan_digest                       TEXT NOT NULL CHECK(length(plan_digest) = 64),
           prepared_by_user_id               TEXT NOT NULL CHECK(length(trim(prepared_by_user_id)) > 0),
           reviewed_by_user_id               TEXT NOT NULL CHECK(length(trim(reviewed_by_user_id)) > 0),
           review_note                       TEXT,
           request_digest                    TEXT NOT NULL CHECK(length(request_digest) = 64),
           review_digest                     TEXT NOT NULL CHECK(length(review_digest) = 64),
           idempotency_scope                 TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                   TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           reviewed_at                       TEXT NOT NULL,
           created_at                        TEXT NOT NULL,
           CHECK(prepared_by_user_id <> reviewed_by_user_id),
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(plan_id) REFERENCES compute_activation_plans(plan_id) ON DELETE RESTRICT,
           FOREIGN KEY(request_id) REFERENCES compute_activation_evidence_requests(request_id) ON DELETE RESTRICT,
           FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id) ON DELETE RESTRICT,
           FOREIGN KEY(pool_id) REFERENCES compute_capacity_pools(pool_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_activation_plan_reviews_reviewer
           ON compute_activation_plan_reviews(reviewed_by_user_id, reviewed_at DESC, review_id);
         CREATE TRIGGER IF NOT EXISTS trg_compute_activation_plan_reviews_no_update
         BEFORE UPDATE ON compute_activation_plan_reviews
         BEGIN
           SELECT RAISE(ABORT, 'compute activation plan reviews are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_activation_plan_reviews_no_delete
         BEFORE DELETE ON compute_activation_plan_reviews
         BEGIN
           SELECT RAISE(ABORT, 'compute activation plan reviews are append-only');
         END;",
    )?;
    Ok(())
}
