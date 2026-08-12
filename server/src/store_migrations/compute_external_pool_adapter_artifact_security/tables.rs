use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(r#"
    CREATE TABLE IF NOT EXISTS compute_external_pool_adapter_artifact_security_receipts (
      security_receipt_id TEXT PRIMARY KEY CHECK(length(trim(security_receipt_id)) BETWEEN 1 AND 160),
      security_receipt_schema TEXT NOT NULL CHECK(security_receipt_schema='compute_federation.external_pool_adapter_artifact_security_receipt.v1'),
      security_receipt_digest TEXT NOT NULL UNIQUE CHECK(length(security_receipt_digest)=64 AND security_receipt_digest NOT GLOB '*[^0-9a-f]*'),
      security_receipt_json TEXT NOT NULL CHECK(json_valid(security_receipt_json) AND json_type(security_receipt_json)='object' AND length(CAST(security_receipt_json AS BLOB))<=1048576),
      security_material_digest TEXT NOT NULL CHECK(length(security_material_digest)=64 AND security_material_digest NOT GLOB '*[^0-9a-f]*'),
      canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
      digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
      admission_id TEXT NOT NULL UNIQUE,
      admission_digest TEXT NOT NULL CHECK(length(admission_digest)=64 AND admission_digest NOT GLOB '*[^0-9a-f]*'),
      source_receipt_digest TEXT NOT NULL UNIQUE CHECK(length(source_receipt_digest)=64 AND source_receipt_digest NOT GLOB '*[^0-9a-f]*'),
      provenance_receipt_digest TEXT NOT NULL UNIQUE CHECK(length(provenance_receipt_digest)=64 AND provenance_receipt_digest NOT GLOB '*[^0-9a-f]*'),
      package_receipt_id TEXT NOT NULL UNIQUE,
      package_receipt_digest TEXT NOT NULL UNIQUE CHECK(length(package_receipt_digest)=64 AND package_receipt_digest NOT GLOB '*[^0-9a-f]*'),
      archive_sha256 TEXT NOT NULL CHECK(length(archive_sha256)=64 AND archive_sha256 NOT GLOB '*[^0-9a-f]*'),
      archive_size_bytes INTEGER NOT NULL CHECK(typeof(archive_size_bytes)='integer' AND archive_size_bytes BETWEEN 1 AND 33554432),
      package_inspection_digest TEXT NOT NULL UNIQUE CHECK(length(package_inspection_digest)=64 AND package_inspection_digest NOT GLOB '*[^0-9a-f]*'),
      manifest_digest TEXT NOT NULL CHECK(length(manifest_digest)=64 AND manifest_digest NOT GLOB '*[^0-9a-f]*'),
      sbom_canonical_json TEXT NOT NULL CHECK(json_valid(sbom_canonical_json) AND json_type(sbom_canonical_json)='object' AND length(CAST(sbom_canonical_json AS BLOB))<=262144),
      sbom_digest TEXT NOT NULL UNIQUE CHECK(length(sbom_digest)=64 AND sbom_digest NOT GLOB '*[^0-9a-f]*'),
      component_inventory_digest TEXT NOT NULL CHECK(length(component_inventory_digest)=64 AND component_inventory_digest NOT GLOB '*[^0-9a-f]*'),
      component_count INTEGER NOT NULL CHECK(typeof(component_count)='integer' AND component_count BETWEEN 1 AND 128),
      license_inventory_digest TEXT NOT NULL CHECK(length(license_inventory_digest)=64 AND license_inventory_digest NOT GLOB '*[^0-9a-f]*'),
      license_count INTEGER NOT NULL CHECK(typeof(license_count)='integer' AND license_count BETWEEN 1 AND component_count),
      scanned_file_inventory_digest TEXT NOT NULL CHECK(length(scanned_file_inventory_digest)=64 AND scanned_file_inventory_digest NOT GLOB '*[^0-9a-f]*'),
      scanned_file_count INTEGER NOT NULL CHECK(typeof(scanned_file_count)='integer' AND scanned_file_count BETWEEN 2 AND 128),
      scanner_rule_set_id TEXT NOT NULL CHECK(scanner_rule_set_id='elon_adapter_static_safety_v1'),
      scanner_rule_set_digest TEXT NOT NULL CHECK(length(scanner_rule_set_digest)=64 AND scanner_rule_set_digest NOT GLOB '*[^0-9a-f]*'),
      license_policy_id TEXT NOT NULL CHECK(license_policy_id='declared_single_spdx_identifier_v1'),
      finding_count INTEGER NOT NULL CHECK(finding_count=0),
      inspection_digest TEXT NOT NULL UNIQUE CHECK(length(inspection_digest)=64 AND inspection_digest NOT GLOB '*[^0-9a-f]*'),
      scanned_by_admin_user_id TEXT NOT NULL CHECK(length(trim(scanned_by_admin_user_id)) BETWEEN 1 AND 160),
      confirmation TEXT NOT NULL CHECK(confirmation='confirm_external_pool_adapter_artifact_static_security_scan'),
      idempotency_scope TEXT NOT NULL CHECK(length(trim(idempotency_scope)) BETWEEN 1 AND 200),
      idempotency_key TEXT NOT NULL CHECK(length(trim(idempotency_key)) BETWEEN 1 AND 160),
      scanned_at TEXT NOT NULL CHECK(scanned_at GLOB '????-??-??T??:??:??.?????????Z' AND length(scanned_at)=30 AND julianday(scanned_at) IS NOT NULL),
      recorded_at TEXT NOT NULL CHECK(recorded_at=scanned_at),
      evidence_scope TEXT NOT NULL CHECK(evidence_scope='exact_sbom_license_and_local_static_rules'),
      artifact_format_effect TEXT NOT NULL CHECK(artifact_format_effect='static_format_verified'),
      artifact_security_effect TEXT NOT NULL CHECK(artifact_security_effect='static_policy_verified'),
      vulnerability_intelligence_effect TEXT NOT NULL CHECK(vulnerability_intelligence_effect='none'),
      conformance_effect TEXT NOT NULL CHECK(conformance_effect='none'),
      adapter_effect TEXT NOT NULL CHECK(adapter_effect='none'),
      route_effect TEXT NOT NULL CHECK(route_effect='none'),
      UNIQUE(idempotency_scope,idempotency_key),
      FOREIGN KEY(package_receipt_id) REFERENCES compute_external_pool_adapter_artifact_package_receipts(package_receipt_id) ON DELETE RESTRICT
    );
    "#)?;
    Ok(())
}
