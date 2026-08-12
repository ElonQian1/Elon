PRAGMA foreign_keys=ON;
CREATE TABLE compute_external_pool_adapter_registry_releases(
  registry_release_id TEXT PRIMARY KEY, registry_release_digest TEXT,
  registry_release_material_digest TEXT, admission_id TEXT, admission_digest TEXT,
  package_receipt_id TEXT, package_receipt_digest TEXT, source_receipt_id TEXT,
  source_receipt_digest TEXT, adapter_id TEXT, release_version TEXT, route_kind TEXT,
  supported_provider_kinds_json TEXT, implementation_digest TEXT,
  declared_implementation_sha256 TEXT, supported_capabilities_json TEXT,
  capability_set_digest TEXT, credential_verifier_json TEXT, credential_verifier_digest TEXT,
  archive_sha256 TEXT, archive_size_bytes INTEGER, manifest_digest TEXT,
  entry_inventory_digest TEXT, entry_count INTEGER, total_uncompressed_bytes INTEGER,
  installation_content_digest TEXT);
CREATE VIEW compute_external_pool_adapter_registry_release_current AS
  SELECT registry_release_id,registry_release_digest,'release_current' current_status
    FROM compute_external_pool_adapter_registry_releases;
CREATE TABLE compute_external_pool_adapter_vulnerability_reattestation_receipts(
  reattestation_receipt_id TEXT PRIMARY KEY, reattestation_receipt_digest TEXT,
  reattestation_material_digest TEXT, registry_release_id TEXT, registry_release_digest TEXT,
  sequence INTEGER, verified_at TEXT, intelligence_snapshot_digest TEXT,
  intelligence_expires_at TEXT, security_receipt_id TEXT, security_receipt_digest TEXT,
  security_material_digest TEXT, sbom_digest TEXT, component_inventory_digest TEXT,
  component_count INTEGER, dependency_inventory_digest TEXT);
CREATE VIEW compute_external_pool_adapter_vulnerability_reattestation_current AS
  SELECT reattestation_receipt_id,reattestation_receipt_digest,registry_release_id,
         registry_release_digest,'verified_current' current_status
    FROM compute_external_pool_adapter_vulnerability_reattestation_receipts;
CREATE TABLE compute_external_pool_adapter_sandbox_verifier_keys(
  key_record_id TEXT, key_record_digest TEXT, key_id TEXT,
  PRIMARY KEY(key_record_id,key_record_digest,key_id));
CREATE VIEW compute_external_pool_adapter_sandbox_verifier_key_current AS
  SELECT key_record_id,key_record_digest,key_id,'' verifier_operator,'' verifier_product,
         'active' current_status
    FROM compute_external_pool_adapter_sandbox_verifier_keys;
