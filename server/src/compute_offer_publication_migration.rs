use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v182(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_offer_publications (
           publication_id                 TEXT PRIMARY KEY,
           offer_id                       TEXT NOT NULL UNIQUE,
           provider_id                    TEXT NOT NULL,
           pool_id                        TEXT NOT NULL,
           source_offer_version           INTEGER NOT NULL CHECK(source_offer_version > 0),
           source_offer_digest            TEXT NOT NULL CHECK(length(source_offer_digest) = 64),
           active_offer_version           INTEGER NOT NULL CHECK(active_offer_version > 1),
           active_offer_digest            TEXT NOT NULL CHECK(length(active_offer_digest) = 64),
           provider_policy_revision       INTEGER NOT NULL CHECK(provider_policy_revision > 0),
           provider_digest                TEXT NOT NULL CHECK(length(provider_digest) = 64),
           publication_digest             TEXT NOT NULL CHECK(length(publication_digest) = 64),
           idempotency_scope              TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key                TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           approved_by_user_id            TEXT NOT NULL CHECK(length(trim(approved_by_user_id)) > 0),
           published_at                   TEXT NOT NULL,
           created_at                     TEXT NOT NULL,
           CHECK(active_offer_version = source_offer_version + 1),
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(offer_id, source_offer_version)
             REFERENCES compute_offer_versions(offer_id, offer_version) ON DELETE RESTRICT,
           FOREIGN KEY(offer_id, active_offer_version)
             REFERENCES compute_offer_versions(offer_id, offer_version) ON DELETE RESTRICT,
           FOREIGN KEY(provider_id)
             REFERENCES compute_providers(provider_id) ON DELETE RESTRICT,
           FOREIGN KEY(pool_id)
             REFERENCES compute_capacity_pools(pool_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_offer_publications_provider
           ON compute_offer_publications(provider_id, published_at DESC, publication_id);
         CREATE TRIGGER IF NOT EXISTS trg_compute_offer_publications_no_update
         BEFORE UPDATE ON compute_offer_publications
         BEGIN
           SELECT RAISE(ABORT, 'compute offer publications are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_offer_publications_no_delete
         BEFORE DELETE ON compute_offer_publications
         BEGIN
           SELECT RAISE(ABORT, 'compute offer publications are append-only');
         END;",
    )?;
    Ok(())
}
