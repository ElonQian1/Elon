use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_external_pool_adapter_artifact_source_receipts (
            source_receipt_id TEXT PRIMARY KEY CHECK(
                length(trim(source_receipt_id)) BETWEEN 1 AND 160),
            source_receipt_schema TEXT NOT NULL CHECK(source_receipt_schema=
                'compute_federation.external_pool_adapter_artifact_source_receipt.v1'),
            source_receipt_digest TEXT NOT NULL UNIQUE CHECK(
                length(source_receipt_digest)=64
                AND source_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            source_receipt_json TEXT NOT NULL CHECK(
                json_valid(source_receipt_json)
                AND json_type(source_receipt_json)='object'
                AND length(CAST(source_receipt_json AS BLOB))<=524288),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            admission_id TEXT NOT NULL UNIQUE CHECK(
                length(trim(admission_id)) BETWEEN 1 AND 160),
            admission_digest TEXT NOT NULL CHECK(
                length(admission_digest)=64
                AND admission_digest NOT GLOB '*[^0-9a-f]*'),
            request_id TEXT NOT NULL CHECK(length(trim(request_id)) BETWEEN 1 AND 160),
            request_digest TEXT NOT NULL CHECK(
                length(request_digest)=64
                AND request_digest NOT GLOB '*[^0-9a-f]*'),
            request_material_digest TEXT NOT NULL CHECK(
                length(request_material_digest)=64
                AND request_material_digest NOT GLOB '*[^0-9a-f]*'),
            review_id TEXT NOT NULL CHECK(length(trim(review_id)) BETWEEN 1 AND 160),
            review_digest TEXT NOT NULL CHECK(
                length(review_digest)=64
                AND review_digest NOT GLOB '*[^0-9a-f]*'),
            adapter_id TEXT NOT NULL CHECK(length(trim(adapter_id)) BETWEEN 1 AND 160),
            release_version TEXT NOT NULL CHECK(
                length(trim(release_version)) BETWEEN 1 AND 80),
            candidate_artifact_ref TEXT NOT NULL CHECK(
                substr(candidate_artifact_ref,1,13)='artifact-ref:'
                AND length(candidate_artifact_ref) BETWEEN 14 AND 173
                AND substr(candidate_artifact_ref,14) NOT GLOB '*[^0-9A-Za-z._-]*'),
            declared_implementation_sha256 TEXT NOT NULL CHECK(
                length(declared_implementation_sha256)=64
                AND declared_implementation_sha256 NOT GLOB '*[^0-9a-f]*'),
            intake_sha256 TEXT NOT NULL CHECK(
                length(intake_sha256)=64
                AND intake_sha256 NOT GLOB '*[^0-9a-f]*'),
            reopened_sha256 TEXT NOT NULL CHECK(
                length(reopened_sha256)=64
                AND reopened_sha256 NOT GLOB '*[^0-9a-f]*'),
            artifact_size_bytes INTEGER NOT NULL CHECK(
                typeof(artifact_size_bytes)='integer'
                AND artifact_size_bytes BETWEEN 1 AND 33554432),
            storage_root_kind TEXT NOT NULL CHECK(storage_root_kind='server_data_dir'),
            storage_namespace TEXT NOT NULL CHECK(storage_namespace=
                'compute-federation/external-pool-adapter-artifacts/v1/quarantine'),
            content_address_algorithm TEXT NOT NULL CHECK(
                content_address_algorithm='sha256'),
            content_address_digest TEXT NOT NULL CHECK(
                length(content_address_digest)=64
                AND content_address_digest NOT GLOB '*[^0-9a-f]*'),
            custody_state TEXT NOT NULL CHECK(custody_state='quarantined'),
            intake_kind TEXT NOT NULL CHECK(intake_kind='admin_authenticated_raw_body'),
            evidence_scope TEXT NOT NULL CHECK(evidence_scope='byte_digest_match_only'),
            artifact_ref_resolution_effect TEXT NOT NULL CHECK(
                artifact_ref_resolution_effect='none'),
            adapter_effect TEXT NOT NULL CHECK(adapter_effect='none'),
            route_effect TEXT NOT NULL CHECK(route_effect='none'),
            recorded_by_admin_user_id TEXT NOT NULL CHECK(
                length(trim(recorded_by_admin_user_id)) BETWEEN 1 AND 160),
            intake_confirmation TEXT NOT NULL CHECK(intake_confirmation=
                'confirm_external_pool_adapter_artifact_source_intake'),
            recorded_at TEXT NOT NULL CHECK(recorded_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(recorded_at)=30
                AND substr(recorded_at,20,1)='.' AND substr(recorded_at,30,1)='Z'
                AND julianday(recorded_at) IS NOT NULL),
            intake_material_digest TEXT NOT NULL CHECK(
                length(intake_material_digest)=64
                AND intake_material_digest NOT GLOB '*[^0-9a-f]*'),
            idempotency_scope TEXT NOT NULL CHECK(
                length(trim(idempotency_scope)) BETWEEN 1 AND 200),
            idempotency_key TEXT NOT NULL CHECK(
                length(trim(idempotency_key)) BETWEEN 1 AND 160),
            created_at TEXT NOT NULL CHECK(created_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(created_at)=30
                AND substr(created_at,20,1)='.' AND substr(created_at,30,1)='Z'
                AND julianday(created_at) IS NOT NULL),
            CHECK(declared_implementation_sha256=intake_sha256
                AND intake_sha256=reopened_sha256
                AND reopened_sha256=content_address_digest),
            CHECK(created_at=recorded_at),
            UNIQUE(idempotency_scope, idempotency_key),
            FOREIGN KEY(admission_id)
                REFERENCES compute_external_pool_adapter_release_admissions(admission_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(request_id)
                REFERENCES compute_external_pool_adapter_release_requests(request_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(review_id)
                REFERENCES compute_external_pool_adapter_release_reviews(review_id)
                ON DELETE RESTRICT
        );
        "#,
    )?;
    Ok(())
}
