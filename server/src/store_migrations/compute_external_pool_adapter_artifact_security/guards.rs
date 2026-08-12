use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(r#"
    CREATE TRIGGER IF NOT EXISTS artifact_security_no_update
    BEFORE UPDATE ON compute_external_pool_adapter_artifact_security_receipts
    BEGIN SELECT RAISE(ABORT,'Artifact security receipts are immutable'); END;

    CREATE TRIGGER IF NOT EXISTS artifact_security_no_delete
    BEFORE DELETE ON compute_external_pool_adapter_artifact_security_receipts
    BEGIN SELECT RAISE(ABORT,'Artifact security receipts are append-only'); END;

    CREATE TRIGGER IF NOT EXISTS artifact_security_no_replace
    BEFORE INSERT ON compute_external_pool_adapter_artifact_security_receipts
    WHEN EXISTS (SELECT 1 FROM compute_external_pool_adapter_artifact_security_receipts old
      WHERE old.security_receipt_id=NEW.security_receipt_id OR old.security_receipt_digest=NEW.security_receipt_digest
         OR old.admission_id=NEW.admission_id OR old.package_receipt_id=NEW.package_receipt_id
         OR old.sbom_digest=NEW.sbom_digest OR old.inspection_digest=NEW.inspection_digest
         OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key))
    BEGIN SELECT RAISE(ABORT,'Artifact security receipt cannot be replaced'); END;

    CREATE TRIGGER IF NOT EXISTS artifact_security_exact_package
    BEFORE INSERT ON compute_external_pool_adapter_artifact_security_receipts
    WHEN NOT EXISTS (
      SELECT 1 FROM compute_external_pool_adapter_artifact_package_current current
      JOIN compute_external_pool_adapter_artifact_package_receipts package
        ON package.package_receipt_id=current.package_receipt_id
      WHERE current.current_status='verified_current'
        AND package.admission_id=NEW.admission_id AND package.admission_digest=NEW.admission_digest
        AND package.source_receipt_digest=NEW.source_receipt_digest
        AND package.provenance_receipt_digest=NEW.provenance_receipt_digest
        AND package.package_receipt_id=NEW.package_receipt_id
        AND package.package_receipt_digest=NEW.package_receipt_digest
        AND package.archive_sha256=NEW.archive_sha256 AND package.archive_size_bytes=NEW.archive_size_bytes
        AND package.inspection_digest=NEW.package_inspection_digest AND package.manifest_digest=NEW.manifest_digest)
    BEGIN SELECT RAISE(ABORT,'Artifact security receipt lacks exact current package authority'); END;

    CREATE TRIGGER IF NOT EXISTS artifact_security_json_projection
    AFTER INSERT ON compute_external_pool_adapter_artifact_security_receipts
    WHEN COALESCE(json_extract(NEW.security_receipt_json,'$.security_receipt_id'),'')<>NEW.security_receipt_id
      OR COALESCE(json_extract(NEW.security_receipt_json,'$.security_receipt_digest'),'')<>NEW.security_receipt_digest
      OR COALESCE(json_extract(NEW.security_receipt_json,'$.security_material_digest'),'')<>NEW.security_material_digest
      OR COALESCE(json_extract(NEW.security_receipt_json,'$.security.admission_id'),'')<>NEW.admission_id
      OR COALESCE(json_extract(NEW.security_receipt_json,'$.security.package_receipt_id'),'')<>NEW.package_receipt_id
      OR COALESCE(json_extract(NEW.security_receipt_json,'$.security.sbom_digest'),'')<>NEW.sbom_digest
      OR COALESCE(json_extract(NEW.security_receipt_json,'$.security.inspection_digest'),'')<>NEW.inspection_digest
      OR COALESCE(json_extract(NEW.security_receipt_json,'$.security.idempotency_scope'),'')<>NEW.idempotency_scope
      OR COALESCE(json_extract(NEW.security_receipt_json,'$.security.idempotency_key'),'')<>NEW.idempotency_key
    BEGIN SELECT RAISE(ABORT,'Artifact security JSON projection mismatch'); END;
    "#)?;
    Ok(())
}
