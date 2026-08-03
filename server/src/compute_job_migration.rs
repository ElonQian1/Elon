use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v172(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_jobs (
            job_id TEXT PRIMARY KEY CHECK (length(trim(job_id)) > 0),
            consumer_account_id TEXT NOT NULL CHECK (
                length(trim(consumer_account_id)) > 0
            ),
            project_id TEXT,
            merchant_id TEXT,
            idempotency_key TEXT NOT NULL CHECK (
                length(trim(idempotency_key)) > 0
            ),
            current_revision INTEGER NOT NULL CHECK (current_revision > 0),
            current_job_digest TEXT NOT NULL CHECK (
                length(trim(current_job_digest)) > 0
            ),
            status TEXT NOT NULL CHECK (
                status IN (
                    'submitted', 'quoted', 'reserved', 'running',
                    'verification_pending', 'settled', 'failed', 'canceled'
                )
            ),
            selected_provider_id TEXT,
            selected_offer_id TEXT,
            selected_offer_version INTEGER,
            selected_offer_digest TEXT,
            price_snapshot_id TEXT,
            max_consumer_charge_micros INTEGER NOT NULL CHECK (
                max_consumer_charge_micros >= 0
            ),
            currency TEXT NOT NULL CHECK (length(trim(currency)) > 0),
            submitted_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            UNIQUE (consumer_account_id, idempotency_key),
            CHECK (project_id IS NULL OR length(trim(project_id)) > 0),
            CHECK (merchant_id IS NULL OR length(trim(merchant_id)) > 0),
            CHECK (
                (
                    selected_provider_id IS NULL
                    AND selected_offer_id IS NULL
                    AND selected_offer_version IS NULL
                    AND selected_offer_digest IS NULL
                    AND price_snapshot_id IS NULL
                ) OR (
                    selected_provider_id IS NOT NULL
                    AND length(trim(selected_provider_id)) > 0
                    AND selected_offer_id IS NOT NULL
                    AND length(trim(selected_offer_id)) > 0
                    AND selected_offer_version IS NOT NULL
                    AND selected_offer_version > 0
                    AND selected_offer_digest IS NOT NULL
                    AND length(trim(selected_offer_digest)) > 0
                    AND price_snapshot_id IS NOT NULL
                    AND length(trim(price_snapshot_id)) > 0
                )
            ),
            CHECK (status <> 'submitted' OR selected_offer_id IS NULL),
            CHECK (
                status NOT IN (
                    'quoted', 'reserved', 'running',
                    'verification_pending', 'settled'
                ) OR selected_offer_id IS NOT NULL
            ),
            FOREIGN KEY (selected_provider_id)
                REFERENCES compute_providers(provider_id)
                ON DELETE RESTRICT,
            FOREIGN KEY (selected_offer_id, selected_offer_version)
                REFERENCES compute_offer_versions(offer_id, offer_version)
                ON DELETE RESTRICT,
            FOREIGN KEY (price_snapshot_id)
                REFERENCES compute_price_snapshots(snapshot_id)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_job_versions (
            job_id TEXT NOT NULL,
            revision INTEGER NOT NULL CHECK (revision > 0),
            job_digest TEXT NOT NULL CHECK (length(trim(job_digest)) > 0),
            status TEXT NOT NULL CHECK (
                status IN (
                    'submitted', 'quoted', 'reserved', 'running',
                    'verification_pending', 'settled', 'failed', 'canceled'
                )
            ),
            selected_provider_id TEXT,
            selected_offer_id TEXT,
            selected_offer_version INTEGER,
            selected_offer_digest TEXT,
            price_snapshot_id TEXT,
            job_json TEXT NOT NULL CHECK (length(trim(job_json)) > 0),
            created_at TEXT NOT NULL,
            PRIMARY KEY (job_id, revision),
            UNIQUE (job_id, job_digest),
            CHECK (
                (
                    selected_provider_id IS NULL
                    AND selected_offer_id IS NULL
                    AND selected_offer_version IS NULL
                    AND selected_offer_digest IS NULL
                    AND price_snapshot_id IS NULL
                ) OR (
                    selected_provider_id IS NOT NULL
                    AND length(trim(selected_provider_id)) > 0
                    AND selected_offer_id IS NOT NULL
                    AND length(trim(selected_offer_id)) > 0
                    AND selected_offer_version IS NOT NULL
                    AND selected_offer_version > 0
                    AND selected_offer_digest IS NOT NULL
                    AND length(trim(selected_offer_digest)) > 0
                    AND price_snapshot_id IS NOT NULL
                    AND length(trim(price_snapshot_id)) > 0
                )
            ),
            CHECK (status <> 'submitted' OR selected_offer_id IS NULL),
            CHECK (
                status NOT IN (
                    'quoted', 'reserved', 'running',
                    'verification_pending', 'settled'
                ) OR selected_offer_id IS NOT NULL
            ),
            FOREIGN KEY (job_id)
                REFERENCES compute_jobs(job_id)
                ON DELETE RESTRICT,
            FOREIGN KEY (selected_provider_id)
                REFERENCES compute_providers(provider_id)
                ON DELETE RESTRICT,
            FOREIGN KEY (selected_offer_id, selected_offer_version)
                REFERENCES compute_offer_versions(offer_id, offer_version)
                ON DELETE RESTRICT,
            FOREIGN KEY (price_snapshot_id)
                REFERENCES compute_price_snapshots(snapshot_id)
                ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_compute_jobs_consumer_status
            ON compute_jobs(consumer_account_id, status, updated_at, job_id);

        CREATE INDEX IF NOT EXISTS idx_compute_jobs_project_status
            ON compute_jobs(project_id, status, updated_at, job_id);

        CREATE INDEX IF NOT EXISTS idx_compute_jobs_merchant_status
            ON compute_jobs(merchant_id, status, updated_at, job_id);

        CREATE INDEX IF NOT EXISTS idx_compute_jobs_provider_status
            ON compute_jobs(selected_provider_id, status, updated_at, job_id);

        CREATE TRIGGER IF NOT EXISTS trg_compute_job_versions_no_update
        BEFORE UPDATE ON compute_job_versions
        BEGIN
            SELECT RAISE(ABORT, 'compute job versions are append-only');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_job_versions_no_delete
        BEFORE DELETE ON compute_job_versions
        BEGIN
            SELECT RAISE(ABORT, 'compute job versions are append-only');
        END;
        "#,
    )?;
    Ok(())
}
