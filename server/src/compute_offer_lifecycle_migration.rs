use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v183(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_offer_lifecycle_events (
           event_id                 TEXT PRIMARY KEY,
           offer_id                 TEXT NOT NULL,
           provider_id              TEXT NOT NULL,
           pool_id                  TEXT NOT NULL,
           previous_status          TEXT NOT NULL CHECK(previous_status IN ('active', 'draining')),
           target_status            TEXT NOT NULL CHECK(target_status IN ('draining', 'expired', 'revoked')),
           previous_offer_version   INTEGER NOT NULL CHECK(previous_offer_version > 0),
           previous_offer_digest    TEXT NOT NULL CHECK(length(previous_offer_digest) = 64),
           target_offer_version     INTEGER NOT NULL CHECK(target_offer_version > 1),
           target_offer_digest      TEXT NOT NULL CHECK(length(target_offer_digest) = 64),
           reason                   TEXT NOT NULL CHECK(length(trim(reason)) > 0),
           event_digest             TEXT NOT NULL CHECK(length(event_digest) = 64),
           idempotency_scope        TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key          TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           changed_by_user_id       TEXT NOT NULL CHECK(length(trim(changed_by_user_id)) > 0),
           changed_at               TEXT NOT NULL,
           created_at               TEXT NOT NULL,
           CHECK(target_offer_version = previous_offer_version + 1),
           CHECK(previous_status <> target_status),
           UNIQUE(offer_id, target_status),
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(offer_id, previous_offer_version)
             REFERENCES compute_offer_versions(offer_id, offer_version) ON DELETE RESTRICT,
           FOREIGN KEY(offer_id, target_offer_version)
             REFERENCES compute_offer_versions(offer_id, offer_version) ON DELETE RESTRICT,
           FOREIGN KEY(provider_id)
             REFERENCES compute_providers(provider_id) ON DELETE RESTRICT,
           FOREIGN KEY(pool_id)
             REFERENCES compute_capacity_pools(pool_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_offer_lifecycle_provider
           ON compute_offer_lifecycle_events(provider_id, changed_at DESC, event_id);
         CREATE TRIGGER IF NOT EXISTS trg_compute_offer_lifecycle_no_update
         BEFORE UPDATE ON compute_offer_lifecycle_events
         BEGIN
           SELECT RAISE(ABORT, 'compute offer lifecycle events are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_offer_lifecycle_no_delete
         BEFORE DELETE ON compute_offer_lifecycle_events
         BEGIN
           SELECT RAISE(ABORT, 'compute offer lifecycle events are append-only');
         END;",
    )?;
    Ok(())
}
