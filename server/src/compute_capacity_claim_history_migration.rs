use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v173(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_capacity_claim_versions (
            claim_id TEXT NOT NULL,
            revision INTEGER NOT NULL CHECK (revision > 0),
            claim_digest TEXT NOT NULL CHECK (length(trim(claim_digest)) > 0),
            status TEXT NOT NULL CHECK (
                status IN ('pending', 'held', 'active', 'consumed', 'released', 'expired', 'canceled')
            ),
            request_digest TEXT NOT NULL CHECK (length(trim(request_digest)) > 0),
            claim_json TEXT NOT NULL CHECK (length(trim(claim_json)) > 0),
            recorded_at TEXT NOT NULL,
            PRIMARY KEY (claim_id, revision),
            UNIQUE (claim_id, claim_digest),
            FOREIGN KEY (claim_id)
                REFERENCES compute_capacity_claims(claim_id)
                ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_compute_capacity_claim_versions_status
            ON compute_capacity_claim_versions(status, recorded_at, claim_id, revision);

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_claim_versions_no_update
        BEFORE UPDATE ON compute_capacity_claim_versions
        BEGIN
            SELECT RAISE(ABORT, 'compute capacity claim versions are append-only');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_claim_versions_no_delete
        BEFORE DELETE ON compute_capacity_claim_versions
        BEGIN
            SELECT RAISE(ABORT, 'compute capacity claim versions are append-only');
        END;
        "#,
    )?;
    Ok(())
}
