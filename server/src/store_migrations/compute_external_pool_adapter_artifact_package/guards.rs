use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS artifact_package_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_artifact_package_receipts
        BEGIN SELECT RAISE(ABORT, 'Artifact package receipts are immutable'); END;

        CREATE TRIGGER IF NOT EXISTS artifact_package_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_artifact_package_receipts
        BEGIN SELECT RAISE(ABORT, 'Artifact package receipts are append-only'); END;

        CREATE TRIGGER IF NOT EXISTS artifact_package_no_replace
        BEFORE INSERT ON compute_external_pool_adapter_artifact_package_receipts
        WHEN EXISTS (SELECT 1 FROM compute_external_pool_adapter_artifact_package_receipts old
          WHERE old.package_receipt_id=NEW.package_receipt_id
             OR old.package_receipt_digest=NEW.package_receipt_digest
             OR old.admission_id=NEW.admission_id
             OR old.provenance_receipt_id=NEW.provenance_receipt_id
             OR old.manifest_digest=NEW.manifest_digest
             OR old.inspection_digest=NEW.inspection_digest
             OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT, 'Artifact package receipt cannot be replaced'); END;

        CREATE TRIGGER IF NOT EXISTS artifact_package_exact_authorities
        BEFORE INSERT ON compute_external_pool_adapter_artifact_package_receipts
        WHEN NOT EXISTS (
          SELECT 1
            FROM compute_external_pool_adapter_release_admission_current admission
            JOIN compute_external_pool_adapter_release_admissions stored_admission
              ON stored_admission.admission_id=admission.admission_id
            JOIN compute_external_pool_adapter_artifact_source_receipts source
              ON source.admission_id=admission.admission_id
            JOIN compute_external_pool_adapter_artifact_signed_provenance_current provenance
              ON provenance.admission_id=admission.admission_id
            JOIN compute_external_pool_adapter_artifact_signed_provenance_receipts signed
              ON signed.provenance_receipt_id=provenance.provenance_receipt_id
           WHERE admission.admission_id=NEW.admission_id
             AND admission.admission_digest=NEW.admission_digest
             AND admission.current_status='staged'
             AND source.source_receipt_digest=NEW.source_receipt_digest
             AND source.reopened_sha256=NEW.archive_sha256
             AND source.artifact_size_bytes=NEW.archive_size_bytes
             AND provenance.provenance_receipt_id=NEW.provenance_receipt_id
             AND provenance.provenance_receipt_digest=NEW.provenance_receipt_digest
             AND provenance.current_status='verified_current'
             AND signed.source_receipt_digest=NEW.source_receipt_digest
             AND signed.artifact_sha256=NEW.archive_sha256
             AND signed.artifact_size_bytes=NEW.archive_size_bytes
             AND stored_admission.adapter_id=NEW.adapter_id
             AND stored_admission.release_version=NEW.release_version
             AND stored_admission.capabilities_json=NEW.supported_capabilities_json
             AND stored_admission.capability_set_digest=NEW.capability_set_digest
             AND stored_admission.verifier_digest=NEW.credential_verifier_digest
             AND stored_admission.verifier_verification_kind=json_extract(NEW.credential_verifier_json,'$.verification_kind')
             AND stored_admission.verifier_id=json_extract(NEW.credential_verifier_json,'$.verifier_id')
             AND stored_admission.verifier_revision=json_extract(NEW.credential_verifier_json,'$.verifier_revision'))
        BEGIN SELECT RAISE(ABORT, 'Artifact package lacks exact current authorities'); END;

        CREATE TRIGGER IF NOT EXISTS artifact_package_json_projection
        AFTER INSERT ON compute_external_pool_adapter_artifact_package_receipts
        WHEN COALESCE(json_extract(NEW.package_receipt_json,'$.package_receipt_id'),'')<>NEW.package_receipt_id
          OR COALESCE(json_extract(NEW.package_receipt_json,'$.package_receipt_digest'),'')<>NEW.package_receipt_digest
          OR COALESCE(json_extract(NEW.package_receipt_json,'$.package_material_digest'),'')<>NEW.package_material_digest
          OR COALESCE(json_extract(NEW.package_receipt_json,'$.package.admission_id'),'')<>NEW.admission_id
          OR COALESCE(json_extract(NEW.package_receipt_json,'$.package.provenance_receipt_id'),'')<>NEW.provenance_receipt_id
          OR COALESCE(json_extract(NEW.package_receipt_json,'$.package.archive_sha256'),'')<>NEW.archive_sha256
          OR COALESCE(json_extract(NEW.package_receipt_json,'$.package.manifest_digest'),'')<>NEW.manifest_digest
          OR COALESCE(json_extract(NEW.package_receipt_json,'$.package.inspection_digest'),'')<>NEW.inspection_digest
          OR COALESCE(json_extract(NEW.package_receipt_json,'$.package.idempotency_scope'),'')<>NEW.idempotency_scope
          OR COALESCE(json_extract(NEW.package_receipt_json,'$.package.idempotency_key'),'')<>NEW.idempotency_key
        BEGIN SELECT RAISE(ABORT, 'Artifact package JSON projection mismatch'); END;
        "#,
    )?;
    Ok(())
}
