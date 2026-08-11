use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS
            compute_external_pool_adapter_release_admission_terminal_receipts (
            terminal_receipt_id TEXT PRIMARY KEY CHECK(
                length(trim(terminal_receipt_id)) BETWEEN 1 AND 160),
            terminal_receipt_schema TEXT NOT NULL CHECK(terminal_receipt_schema=
                'compute_federation.external_pool_adapter_release_admission_terminal_receipt.v1'),
            terminal_receipt_digest TEXT NOT NULL UNIQUE CHECK(
                length(terminal_receipt_digest)=64
                AND terminal_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            terminal_receipt_json TEXT NOT NULL CHECK(
                json_valid(terminal_receipt_json)
                AND json_type(terminal_receipt_json)='object'
                AND length(CAST(terminal_receipt_json AS BLOB))<=524288),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            request_digest TEXT NOT NULL CHECK(
                length(request_digest)=64
                AND request_digest NOT GLOB '*[^0-9a-f]*'),
            admission_id TEXT NOT NULL UNIQUE CHECK(
                length(trim(admission_id)) BETWEEN 1 AND 160),
            admission_digest TEXT NOT NULL CHECK(
                length(admission_digest)=64
                AND admission_digest NOT GLOB '*[^0-9a-f]*'),
            adapter_id TEXT NOT NULL CHECK(length(trim(adapter_id)) BETWEEN 1 AND 160),
            release_version TEXT NOT NULL CHECK(
                length(trim(release_version)) BETWEEN 1 AND 80),
            prior_status TEXT NOT NULL CHECK(prior_status='staged'),
            terminal_status TEXT NOT NULL CHECK(
                terminal_status IN ('withdrawn','revoked','superseded')),
            successor_admission_id TEXT CHECK(successor_admission_id IS NULL OR
                length(trim(successor_admission_id)) BETWEEN 1 AND 160),
            successor_admission_digest TEXT CHECK(successor_admission_digest IS NULL OR (
                length(successor_admission_digest)=64
                AND successor_admission_digest NOT GLOB '*[^0-9a-f]*')),
            successor_release_version TEXT CHECK(successor_release_version IS NULL OR
                length(trim(successor_release_version)) BETWEEN 1 AND 80),
            actor_kind TEXT NOT NULL CHECK(actor_kind='platform_admin'),
            actor_id TEXT NOT NULL CHECK(length(trim(actor_id)) BETWEEN 1 AND 160),
            reason TEXT NOT NULL CHECK(
                length(reason) BETWEEN 8 AND 2000 AND reason=trim(reason)),
            confirmation TEXT NOT NULL CHECK(
                (terminal_status='withdrawn' AND confirmation=
                    'confirm_external_pool_adapter_release_admission_withdrawal')
                OR (terminal_status='revoked' AND confirmation=
                    'confirm_external_pool_adapter_release_admission_revocation')
                OR (terminal_status='superseded' AND confirmation=
                    'confirm_external_pool_adapter_release_admission_supersession')),
            idempotency_scope TEXT NOT NULL CHECK(
                length(trim(idempotency_scope)) BETWEEN 1 AND 200),
            idempotency_key TEXT NOT NULL CHECK(
                length(trim(idempotency_key)) BETWEEN 1 AND 160),
            occurred_at TEXT NOT NULL CHECK(occurred_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(occurred_at)=30
                AND substr(occurred_at,20,1)='.' AND substr(occurred_at,30,1)='Z'
                AND julianday(occurred_at) IS NOT NULL),
            recorded_at TEXT NOT NULL CHECK(recorded_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(recorded_at)=30
                AND substr(recorded_at,20,1)='.' AND substr(recorded_at,30,1)='Z'
                AND julianday(recorded_at) IS NOT NULL),
            currentness_effect TEXT NOT NULL CHECK(
                currentness_effect='admission_terminal'),
            artifact_intake_effect TEXT NOT NULL CHECK(artifact_intake_effect='blocked'),
            existing_artifact_source_effect TEXT NOT NULL CHECK(
                existing_artifact_source_effect='historical_only'),
            adapter_effect TEXT NOT NULL CHECK(adapter_effect='none'),
            route_effect TEXT NOT NULL CHECK(route_effect='none'),
            CHECK(occurred_at=recorded_at),
            CHECK(
                (terminal_status IN ('withdrawn','revoked')
                    AND successor_admission_id IS NULL
                    AND successor_admission_digest IS NULL
                    AND successor_release_version IS NULL)
                OR (terminal_status='superseded'
                    AND successor_admission_id IS NOT NULL
                    AND successor_admission_digest IS NOT NULL
                    AND successor_release_version IS NOT NULL
                    AND successor_admission_id<>admission_id
                    AND successor_release_version<>release_version)),
            UNIQUE(idempotency_scope, idempotency_key),
            FOREIGN KEY(admission_id)
                REFERENCES compute_external_pool_adapter_release_admissions(admission_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(admission_digest)
                REFERENCES compute_external_pool_adapter_release_admissions(admission_digest)
                ON DELETE RESTRICT,
            FOREIGN KEY(successor_admission_id)
                REFERENCES compute_external_pool_adapter_release_admissions(admission_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(successor_admission_digest)
                REFERENCES compute_external_pool_adapter_release_admissions(admission_digest)
                ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_external_pool_adapter_release_terminal_status
            ON compute_external_pool_adapter_release_admission_terminal_receipts(
                terminal_status, recorded_at DESC, terminal_receipt_id);
        CREATE INDEX IF NOT EXISTS idx_external_pool_adapter_release_terminal_successor
            ON compute_external_pool_adapter_release_admission_terminal_receipts(
                successor_admission_id, terminal_receipt_id)
            WHERE successor_admission_id IS NOT NULL;
        "#,
    )?;
    Ok(())
}
