use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_external_pool_adapter_installation_terminal_receipts (
            terminal_receipt_id TEXT PRIMARY KEY NOT NULL CHECK(
                length(trim(terminal_receipt_id)) BETWEEN 1 AND 200),
            terminal_receipt_digest TEXT NOT NULL UNIQUE CHECK(
                length(terminal_receipt_digest)=64
                AND terminal_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            terminal_receipt_schema TEXT NOT NULL CHECK(terminal_receipt_schema=
                'compute_federation.external_pool_adapter_installation_terminal_receipt.v1'),
            receipt_json TEXT NOT NULL CHECK(
                json_valid(receipt_json) AND json_type(receipt_json)='object'
                AND length(CAST(receipt_json AS BLOB))<=262144),
            terminal_material_digest TEXT NOT NULL CHECK(
                length(terminal_material_digest)=64
                AND terminal_material_digest NOT GLOB '*[^0-9a-f]*'),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            installation_receipt_id TEXT NOT NULL UNIQUE,
            installation_receipt_digest TEXT NOT NULL CHECK(
                length(installation_receipt_digest)=64
                AND installation_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            terminal_kind TEXT NOT NULL CHECK(terminal_kind='revoked'),
            revoked_by_admin_user_id TEXT NOT NULL CHECK(
                length(trim(revoked_by_admin_user_id)) BETWEEN 1 AND 200),
            reason TEXT NOT NULL CHECK(
                length(trim(reason)) BETWEEN 1 AND 1000 AND reason=trim(reason)),
            confirmation TEXT NOT NULL CHECK(confirmation=
                'confirm_external_pool_adapter_installation_revocation'),
            idempotency_scope TEXT NOT NULL CHECK(
                length(trim(idempotency_scope)) BETWEEN 1 AND 240),
            idempotency_key TEXT NOT NULL CHECK(
                length(trim(idempotency_key)) BETWEEN 1 AND 240),
            revoked_at TEXT NOT NULL CHECK(revoked_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(revoked_at)=30
                AND substr(revoked_at,20,1)='.' AND substr(revoked_at,30,1)='Z'
                AND julianday(revoked_at) IS NOT NULL),
            recorded_at TEXT NOT NULL CHECK(recorded_at=revoked_at),
            installation_effect TEXT NOT NULL CHECK(
                installation_effect='installed_instance_revoked'),
            credential_effect TEXT NOT NULL CHECK(credential_effect='none'),
            provider_effect TEXT NOT NULL CHECK(provider_effect='none'),
            route_effect TEXT NOT NULL CHECK(route_effect='none'),
            execution_effect TEXT NOT NULL CHECK(execution_effect='none'),
            settlement_effect TEXT NOT NULL CHECK(settlement_effect='none'),
            UNIQUE(idempotency_scope,idempotency_key),
            FOREIGN KEY(installation_receipt_id)
                REFERENCES compute_external_pool_adapter_installation_receipts(
                    installation_receipt_id) ON DELETE RESTRICT
        );
        "#,
    )?;
    Ok(())
}
