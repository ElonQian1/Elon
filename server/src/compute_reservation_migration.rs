use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v174(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_reservations (
            reservation_id TEXT PRIMARY KEY CHECK (
                length(trim(reservation_id)) > 0
            ),
            consumer_account_id TEXT NOT NULL CHECK (
                length(trim(consumer_account_id)) > 0
            ),
            idempotency_key TEXT NOT NULL CHECK (
                length(trim(idempotency_key)) > 0
            ),
            current_revision INTEGER NOT NULL CHECK (current_revision > 0),
            current_reservation_digest TEXT NOT NULL CHECK (
                length(trim(current_reservation_digest)) > 0
            ),
            status TEXT NOT NULL CHECK (
                status IN ('pending', 'active', 'consumed', 'released', 'expired')
            ),
            job_id TEXT NOT NULL,
            job_revision INTEGER NOT NULL CHECK (job_revision > 0),
            job_digest TEXT NOT NULL CHECK (length(trim(job_digest)) > 0),
            provider_id TEXT NOT NULL CHECK (length(trim(provider_id)) > 0),
            offer_id TEXT NOT NULL CHECK (length(trim(offer_id)) > 0),
            offer_version INTEGER NOT NULL CHECK (offer_version > 0),
            offer_digest TEXT NOT NULL CHECK (length(trim(offer_digest)) > 0),
            price_snapshot_id TEXT NOT NULL CHECK (
                length(trim(price_snapshot_id)) > 0
            ),
            capacity_claim_id TEXT NOT NULL CHECK (
                length(trim(capacity_claim_id)) > 0
            ),
            capacity_claim_revision INTEGER NOT NULL CHECK (
                capacity_claim_revision > 0
            ),
            capacity_claim_digest TEXT NOT NULL CHECK (
                length(trim(capacity_claim_digest)) > 0
            ),
            consumer_authorization_ref TEXT NOT NULL CHECK (
                length(trim(consumer_authorization_ref)) > 0
            ),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            consumed_at TEXT,
            released_at TEXT,
            recorded_at TEXT NOT NULL,
            UNIQUE (consumer_account_id, idempotency_key),
            FOREIGN KEY (job_id, job_revision)
                REFERENCES compute_job_versions(job_id, revision)
                ON DELETE RESTRICT,
            FOREIGN KEY (offer_id, offer_version)
                REFERENCES compute_offer_versions(offer_id, offer_version)
                ON DELETE RESTRICT,
            FOREIGN KEY (price_snapshot_id)
                REFERENCES compute_price_snapshots(snapshot_id)
                ON DELETE RESTRICT,
            FOREIGN KEY (capacity_claim_id, capacity_claim_revision)
                REFERENCES compute_capacity_claim_versions(claim_id, revision)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_reservation_versions (
            reservation_id TEXT NOT NULL,
            revision INTEGER NOT NULL CHECK (revision > 0),
            reservation_digest TEXT NOT NULL CHECK (
                length(trim(reservation_digest)) > 0
            ),
            status TEXT NOT NULL CHECK (
                status IN ('pending', 'active', 'consumed', 'released', 'expired')
            ),
            job_id TEXT NOT NULL,
            job_revision INTEGER NOT NULL CHECK (job_revision > 0),
            job_digest TEXT NOT NULL CHECK (length(trim(job_digest)) > 0),
            provider_id TEXT NOT NULL CHECK (length(trim(provider_id)) > 0),
            offer_id TEXT NOT NULL CHECK (length(trim(offer_id)) > 0),
            offer_version INTEGER NOT NULL CHECK (offer_version > 0),
            offer_digest TEXT NOT NULL CHECK (length(trim(offer_digest)) > 0),
            price_snapshot_id TEXT NOT NULL CHECK (
                length(trim(price_snapshot_id)) > 0
            ),
            capacity_claim_id TEXT NOT NULL CHECK (
                length(trim(capacity_claim_id)) > 0
            ),
            capacity_claim_revision INTEGER NOT NULL CHECK (
                capacity_claim_revision > 0
            ),
            capacity_claim_digest TEXT NOT NULL CHECK (
                length(trim(capacity_claim_digest)) > 0
            ),
            reservation_json TEXT NOT NULL CHECK (
                length(trim(reservation_json)) > 0
            ),
            recorded_at TEXT NOT NULL,
            PRIMARY KEY (reservation_id, revision),
            UNIQUE (reservation_id, reservation_digest),
            FOREIGN KEY (reservation_id)
                REFERENCES compute_reservations(reservation_id)
                ON DELETE RESTRICT,
            FOREIGN KEY (job_id, job_revision)
                REFERENCES compute_job_versions(job_id, revision)
                ON DELETE RESTRICT,
            FOREIGN KEY (offer_id, offer_version)
                REFERENCES compute_offer_versions(offer_id, offer_version)
                ON DELETE RESTRICT,
            FOREIGN KEY (price_snapshot_id)
                REFERENCES compute_price_snapshots(snapshot_id)
                ON DELETE RESTRICT,
            FOREIGN KEY (capacity_claim_id, capacity_claim_revision)
                REFERENCES compute_capacity_claim_versions(claim_id, revision)
                ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_compute_reservations_consumer_status
            ON compute_reservations(
                consumer_account_id, status, updated_at, reservation_id
            );

        CREATE INDEX IF NOT EXISTS idx_compute_reservations_job_status
            ON compute_reservations(job_id, status, updated_at, reservation_id);

        CREATE INDEX IF NOT EXISTS idx_compute_reservations_claim
            ON compute_reservations(capacity_claim_id, capacity_claim_revision);

        CREATE TRIGGER IF NOT EXISTS trg_compute_reservation_versions_no_update
        BEFORE UPDATE ON compute_reservation_versions
        BEGIN
            SELECT RAISE(ABORT, 'compute reservation versions are append-only');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_reservation_versions_no_delete
        BEFORE DELETE ON compute_reservation_versions
        BEGIN
            SELECT RAISE(ABORT, 'compute reservation versions are append-only');
        END;
        "#,
    )?;
    Ok(())
}
