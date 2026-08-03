use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v170(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_offers (
            offer_id TEXT PRIMARY KEY,
            provider_id TEXT NOT NULL,
            provider_kind TEXT NOT NULL CHECK (
                provider_kind IN ('user_node', 'managed_cluster', 'external_pool')
            ),
            sku_id TEXT NOT NULL CHECK (length(trim(sku_id)) > 0),
            sku_digest TEXT NOT NULL CHECK (length(trim(sku_digest)) > 0),
            capacity_pool_id TEXT NOT NULL,
            current_offer_version INTEGER NOT NULL CHECK (current_offer_version > 0),
            current_offer_digest TEXT NOT NULL CHECK (length(trim(current_offer_digest)) > 0),
            current_provider_policy_revision INTEGER NOT NULL CHECK (
                current_provider_policy_revision > 0
            ),
            current_provider_digest TEXT NOT NULL CHECK (
                length(trim(current_provider_digest)) > 0
            ),
            status TEXT NOT NULL CHECK (
                status IN ('draft', 'active', 'draining', 'expired', 'revoked')
            ),
            valid_from TEXT NOT NULL,
            valid_until TEXT NOT NULL,
            first_created_at TEXT NOT NULL,
            current_version_created_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            CHECK (valid_from < valid_until),
            FOREIGN KEY (provider_id)
                REFERENCES compute_providers(provider_id)
                ON DELETE RESTRICT,
            FOREIGN KEY (provider_id, current_provider_policy_revision)
                REFERENCES compute_provider_versions(provider_id, policy_revision)
                ON DELETE RESTRICT,
            FOREIGN KEY (capacity_pool_id)
                REFERENCES compute_capacity_pools(pool_id)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_offer_versions (
            offer_id TEXT NOT NULL,
            offer_version INTEGER NOT NULL CHECK (offer_version > 0),
            offer_digest TEXT NOT NULL CHECK (length(trim(offer_digest)) > 0),
            provider_id TEXT NOT NULL,
            provider_policy_revision INTEGER NOT NULL CHECK (provider_policy_revision > 0),
            provider_digest TEXT NOT NULL CHECK (length(trim(provider_digest)) > 0),
            sku_id TEXT NOT NULL CHECK (length(trim(sku_id)) > 0),
            sku_digest TEXT NOT NULL CHECK (length(trim(sku_digest)) > 0),
            capacity_pool_id TEXT NOT NULL,
            capacity_epoch INTEGER NOT NULL CHECK (capacity_epoch > 0),
            pool_revision INTEGER NOT NULL CHECK (pool_revision > 0),
            pool_digest TEXT NOT NULL CHECK (length(trim(pool_digest)) > 0),
            status TEXT NOT NULL CHECK (
                status IN ('draft', 'active', 'draining', 'expired', 'revoked')
            ),
            valid_from TEXT NOT NULL,
            valid_until TEXT NOT NULL,
            offer_json TEXT NOT NULL CHECK (length(trim(offer_json)) > 0),
            created_at TEXT NOT NULL,
            PRIMARY KEY (offer_id, offer_version),
            UNIQUE (offer_id, offer_digest),
            CHECK (valid_from < valid_until),
            FOREIGN KEY (offer_id)
                REFERENCES compute_offers(offer_id)
                ON DELETE RESTRICT,
            FOREIGN KEY (provider_id, provider_policy_revision)
                REFERENCES compute_provider_versions(provider_id, policy_revision)
                ON DELETE RESTRICT,
            FOREIGN KEY (capacity_pool_id, capacity_epoch, pool_revision)
                REFERENCES compute_capacity_pool_versions(pool_id, capacity_epoch, pool_revision)
                ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_compute_offers_provider_status
            ON compute_offers(provider_id, status, valid_until, offer_id);

        CREATE INDEX IF NOT EXISTS idx_compute_offers_sku_status
            ON compute_offers(sku_id, sku_digest, status, valid_until, offer_id);

        CREATE INDEX IF NOT EXISTS idx_compute_offers_pool_status
            ON compute_offers(capacity_pool_id, status, valid_until, offer_id);

        CREATE TRIGGER IF NOT EXISTS trg_compute_offer_versions_no_update
        BEFORE UPDATE ON compute_offer_versions
        BEGIN
            SELECT RAISE(ABORT, 'compute offer versions are append-only');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_offer_versions_no_delete
        BEFORE DELETE ON compute_offer_versions
        BEGIN
            SELECT RAISE(ABORT, 'compute offer versions are append-only');
        END;
        "#,
    )?;
    Ok(())
}
