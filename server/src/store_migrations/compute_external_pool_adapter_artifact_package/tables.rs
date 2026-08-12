use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_external_pool_adapter_artifact_package_receipts (
            package_receipt_id TEXT PRIMARY KEY CHECK(length(trim(package_receipt_id)) BETWEEN 1 AND 160),
            package_receipt_schema TEXT NOT NULL CHECK(package_receipt_schema=
                'compute_federation.external_pool_adapter_artifact_package_receipt.v1'),
            package_receipt_digest TEXT NOT NULL UNIQUE CHECK(length(package_receipt_digest)=64
                AND package_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            package_receipt_json TEXT NOT NULL CHECK(json_valid(package_receipt_json)
                AND json_type(package_receipt_json)='object'
                AND length(CAST(package_receipt_json AS BLOB))<=524288),
            package_material_digest TEXT NOT NULL CHECK(length(package_material_digest)=64
                AND package_material_digest NOT GLOB '*[^0-9a-f]*'),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            admission_id TEXT NOT NULL UNIQUE,
            admission_digest TEXT NOT NULL CHECK(length(admission_digest)=64
                AND admission_digest NOT GLOB '*[^0-9a-f]*'),
            source_receipt_digest TEXT NOT NULL UNIQUE CHECK(length(source_receipt_digest)=64
                AND source_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            provenance_receipt_id TEXT NOT NULL UNIQUE,
            provenance_receipt_digest TEXT NOT NULL UNIQUE CHECK(length(provenance_receipt_digest)=64
                AND provenance_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            archive_sha256 TEXT NOT NULL CHECK(length(archive_sha256)=64
                AND archive_sha256 NOT GLOB '*[^0-9a-f]*'),
            archive_size_bytes INTEGER NOT NULL CHECK(typeof(archive_size_bytes)='integer'
                AND archive_size_bytes BETWEEN 1 AND 33554432),
            manifest_canonical_json TEXT NOT NULL CHECK(json_valid(manifest_canonical_json)
                AND json_type(manifest_canonical_json)='object'
                AND length(CAST(manifest_canonical_json AS BLOB))<=65536),
            manifest_digest TEXT NOT NULL UNIQUE CHECK(length(manifest_digest)=64
                AND manifest_digest NOT GLOB '*[^0-9a-f]*'),
            entry_inventory_digest TEXT NOT NULL CHECK(length(entry_inventory_digest)=64
                AND entry_inventory_digest NOT GLOB '*[^0-9a-f]*'),
            entry_count INTEGER NOT NULL CHECK(typeof(entry_count)='integer' AND entry_count BETWEEN 1 AND 128),
            total_uncompressed_bytes INTEGER NOT NULL CHECK(typeof(total_uncompressed_bytes)='integer'
                AND total_uncompressed_bytes BETWEEN 1 AND 67108864),
            inspection_digest TEXT NOT NULL UNIQUE CHECK(length(inspection_digest)=64
                AND inspection_digest NOT GLOB '*[^0-9a-f]*'),
            adapter_id TEXT NOT NULL CHECK(length(trim(adapter_id)) BETWEEN 1 AND 160),
            release_version TEXT NOT NULL CHECK(length(trim(release_version)) BETWEEN 1 AND 80),
            runtime_kind TEXT NOT NULL CHECK(runtime_kind='server_sidecar_v1'),
            runtime_entrypoint TEXT NOT NULL CHECK(length(runtime_entrypoint) BETWEEN 1 AND 160),
            supported_capabilities_json TEXT NOT NULL CHECK(json_valid(supported_capabilities_json)
                AND json_type(supported_capabilities_json)='array'
                AND json_array_length(supported_capabilities_json)=6),
            capability_set_digest TEXT NOT NULL CHECK(length(capability_set_digest)=64
                AND capability_set_digest NOT GLOB '*[^0-9a-f]*'),
            credential_verifier_json TEXT NOT NULL CHECK(json_valid(credential_verifier_json)
                AND json_type(credential_verifier_json)='object'),
            credential_verifier_digest TEXT NOT NULL CHECK(length(credential_verifier_digest)=64
                AND credential_verifier_digest NOT GLOB '*[^0-9a-f]*'),
            inspected_by_admin_user_id TEXT NOT NULL CHECK(length(trim(inspected_by_admin_user_id)) BETWEEN 1 AND 160),
            confirmation TEXT NOT NULL CHECK(confirmation='confirm_external_pool_adapter_artifact_package_inspection'),
            idempotency_scope TEXT NOT NULL CHECK(length(trim(idempotency_scope)) BETWEEN 1 AND 200),
            idempotency_key TEXT NOT NULL CHECK(length(trim(idempotency_key)) BETWEEN 1 AND 160),
            inspected_at TEXT NOT NULL CHECK(inspected_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(inspected_at)=30 AND julianday(inspected_at) IS NOT NULL),
            recorded_at TEXT NOT NULL CHECK(recorded_at=inspected_at),
            evidence_scope TEXT NOT NULL CHECK(evidence_scope='bounded_static_zip_manifest_match'),
            artifact_format_effect TEXT NOT NULL CHECK(artifact_format_effect='static_format_verified'),
            artifact_security_effect TEXT NOT NULL CHECK(artifact_security_effect='none'),
            conformance_effect TEXT NOT NULL CHECK(conformance_effect='none'),
            adapter_effect TEXT NOT NULL CHECK(adapter_effect='none'),
            route_effect TEXT NOT NULL CHECK(route_effect='none'),
            UNIQUE(idempotency_scope,idempotency_key),
            FOREIGN KEY(admission_id) REFERENCES compute_external_pool_adapter_release_admissions(admission_id) ON DELETE RESTRICT,
            FOREIGN KEY(provenance_receipt_id) REFERENCES compute_external_pool_adapter_artifact_signed_provenance_receipts(provenance_receipt_id) ON DELETE RESTRICT
        );
        "#,
    )?;
    Ok(())
}
