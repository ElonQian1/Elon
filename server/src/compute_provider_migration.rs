use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v169(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_providers (
            provider_id TEXT PRIMARY KEY,
            provider_kind TEXT NOT NULL CHECK (
                provider_kind IN ('user_node', 'managed_cluster', 'external_pool')
            ),
            owner_account_id TEXT NOT NULL CHECK (length(trim(owner_account_id)) > 0),
            settlement_account_id TEXT,
            display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
            status TEXT NOT NULL CHECK (
                status IN ('registering', 'active', 'draining', 'disabled', 'quarantined')
            ),
            trust_tier TEXT NOT NULL CHECK (length(trim(trust_tier)) > 0),
            home_region TEXT,
            current_policy_revision INTEGER NOT NULL CHECK (current_policy_revision > 0),
            current_provider_digest TEXT NOT NULL CHECK (
                length(trim(current_provider_digest)) > 0
            ),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            CHECK (
                settlement_account_id IS NULL
                OR length(trim(settlement_account_id)) > 0
            ),
            CHECK (home_region IS NULL OR length(trim(home_region)) > 0)
        );

        CREATE TABLE IF NOT EXISTS compute_provider_versions (
            provider_id TEXT NOT NULL,
            policy_revision INTEGER NOT NULL CHECK (policy_revision > 0),
            provider_digest TEXT NOT NULL CHECK (length(trim(provider_digest)) > 0),
            provider_json TEXT NOT NULL CHECK (length(trim(provider_json)) > 0),
            created_at TEXT NOT NULL,
            PRIMARY KEY (provider_id, policy_revision),
            UNIQUE (provider_id, provider_digest),
            FOREIGN KEY (provider_id)
                REFERENCES compute_providers(provider_id)
                ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_compute_providers_owner_status
            ON compute_providers(owner_account_id, status, provider_id);

        CREATE INDEX IF NOT EXISTS idx_compute_providers_kind_status
            ON compute_providers(provider_kind, status, provider_id);

        CREATE TRIGGER IF NOT EXISTS trg_compute_provider_versions_no_update
        BEFORE UPDATE ON compute_provider_versions
        BEGIN
            SELECT RAISE(ABORT, 'compute provider versions are append-only');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_provider_versions_no_delete
        BEFORE DELETE ON compute_provider_versions
        BEGIN
            SELECT RAISE(ABORT, 'compute provider versions are append-only');
        END;
        "#,
    )?;
    Ok(())
}
