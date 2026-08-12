use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS
            compute_external_pool_adapter_artifact_signed_provenance_receipts (
            provenance_receipt_id TEXT PRIMARY KEY CHECK(
                length(trim(provenance_receipt_id)) BETWEEN 1 AND 160),
            provenance_receipt_schema TEXT NOT NULL CHECK(provenance_receipt_schema=
                'compute_federation.external_pool_adapter_artifact_signed_provenance_receipt.v1'),
            provenance_receipt_digest TEXT NOT NULL UNIQUE CHECK(
                length(provenance_receipt_digest)=64
                AND provenance_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            provenance_receipt_json TEXT NOT NULL CHECK(
                json_valid(provenance_receipt_json)
                AND json_type(provenance_receipt_json)='object'
                AND length(CAST(provenance_receipt_json AS BLOB))<=262144),
            verification_material_digest TEXT NOT NULL CHECK(
                length(verification_material_digest)=64
                AND verification_material_digest NOT GLOB '*[^0-9a-f]*'),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            admission_id TEXT NOT NULL UNIQUE,
            admission_digest TEXT NOT NULL CHECK(
                length(admission_digest)=64 AND admission_digest NOT GLOB '*[^0-9a-f]*'),
            adapter_id TEXT NOT NULL CHECK(length(trim(adapter_id)) BETWEEN 1 AND 160),
            release_version TEXT NOT NULL CHECK(length(trim(release_version)) BETWEEN 1 AND 80),
            candidate_artifact_ref_digest TEXT NOT NULL CHECK(
                length(candidate_artifact_ref_digest)=64
                AND candidate_artifact_ref_digest NOT GLOB '*[^0-9a-f]*'),
            source_receipt_id TEXT NOT NULL UNIQUE,
            source_receipt_digest TEXT NOT NULL UNIQUE CHECK(
                length(source_receipt_digest)=64
                AND source_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            artifact_sha256 TEXT NOT NULL CHECK(
                length(artifact_sha256)=64 AND artifact_sha256 NOT GLOB '*[^0-9a-f]*'),
            artifact_size_bytes INTEGER NOT NULL CHECK(
                typeof(artifact_size_bytes)='integer'
                AND artifact_size_bytes BETWEEN 1 AND 33554432),
            key_record_id TEXT NOT NULL,
            key_record_digest TEXT NOT NULL CHECK(
                length(key_record_digest)=64 AND key_record_digest NOT GLOB '*[^0-9a-f]*'),
            key_id TEXT NOT NULL CHECK(length(key_id)=64 AND key_id NOT GLOB '*[^0-9a-f]*'),
            source_operator TEXT NOT NULL CHECK(
                length(source_operator) BETWEEN 1 AND 160 AND source_operator=trim(source_operator)),
            signature_algorithm TEXT NOT NULL CHECK(signature_algorithm='rsa-pkcs1v15-sha256'),
            signature_message_digest TEXT NOT NULL CHECK(
                length(signature_message_digest)=64
                AND signature_message_digest NOT GLOB '*[^0-9a-f]*'),
            signature_base64 TEXT NOT NULL CHECK(
                length(signature_base64) BETWEEN 1 AND 1368),
            signature_digest TEXT NOT NULL UNIQUE CHECK(
                length(signature_digest)=64 AND signature_digest NOT GLOB '*[^0-9a-f]*'),
            verified_by_admin_user_id TEXT NOT NULL CHECK(
                length(trim(verified_by_admin_user_id)) BETWEEN 1 AND 160),
            confirmation TEXT NOT NULL CHECK(confirmation=
                'confirm_external_pool_adapter_artifact_signed_provenance'),
            idempotency_scope TEXT NOT NULL CHECK(
                length(trim(idempotency_scope)) BETWEEN 1 AND 200),
            idempotency_key TEXT NOT NULL CHECK(
                length(trim(idempotency_key)) BETWEEN 1 AND 160),
            verified_at TEXT NOT NULL CHECK(verified_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(verified_at)=30
                AND julianday(verified_at) IS NOT NULL),
            recorded_at TEXT NOT NULL CHECK(recorded_at=verified_at),
            evidence_scope TEXT NOT NULL CHECK(evidence_scope=
                'rsa_signature_over_exact_artifact_binding'),
            artifact_ref_resolution_effect TEXT NOT NULL CHECK(
                artifact_ref_resolution_effect='none'),
            artifact_format_effect TEXT NOT NULL CHECK(artifact_format_effect='none'),
            conformance_effect TEXT NOT NULL CHECK(conformance_effect='none'),
            adapter_effect TEXT NOT NULL CHECK(adapter_effect='none'),
            route_effect TEXT NOT NULL CHECK(route_effect='none'),
            UNIQUE(idempotency_scope, idempotency_key),
            FOREIGN KEY(admission_id)
                REFERENCES compute_external_pool_adapter_release_admissions(admission_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(source_receipt_id)
                REFERENCES compute_external_pool_adapter_artifact_source_receipts(source_receipt_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(key_record_id, key_record_digest, key_id)
                REFERENCES compute_external_pool_adapter_artifact_signing_keys(
                    key_record_id, key_record_digest, key_id) ON DELETE RESTRICT
        );
        "#,
    )?;
    Ok(())
}
