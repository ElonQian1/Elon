use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS artifact_signed_provenance_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_artifact_signed_provenance_receipts
        BEGIN SELECT RAISE(ABORT, 'Artifact signed-provenance receipts are immutable'); END;

        CREATE TRIGGER IF NOT EXISTS artifact_signed_provenance_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_artifact_signed_provenance_receipts
        BEGIN SELECT RAISE(ABORT, 'Artifact signed-provenance receipts are append-only'); END;

        CREATE TRIGGER IF NOT EXISTS artifact_signed_provenance_no_replace
        BEFORE INSERT ON compute_external_pool_adapter_artifact_signed_provenance_receipts
        WHEN EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_artifact_signed_provenance_receipts old
             WHERE old.provenance_receipt_id=NEW.provenance_receipt_id
                OR old.provenance_receipt_digest=NEW.provenance_receipt_digest
                OR old.admission_id=NEW.admission_id
                OR old.source_receipt_id=NEW.source_receipt_id
                OR old.signature_digest=NEW.signature_digest
                OR (old.idempotency_scope=NEW.idempotency_scope
                    AND old.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT, 'Artifact signed-provenance receipt cannot be replaced'); END;

        CREATE TRIGGER IF NOT EXISTS artifact_signed_provenance_exact_authorities
        BEFORE INSERT ON compute_external_pool_adapter_artifact_signed_provenance_receipts
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_external_pool_adapter_release_admission_current admission
              JOIN compute_external_pool_adapter_artifact_source_receipts source
                ON source.admission_id=admission.admission_id
              JOIN compute_external_pool_adapter_artifact_signing_key_current signer
                ON signer.key_record_id=NEW.key_record_id
             WHERE admission.admission_id=NEW.admission_id
               AND admission.admission_digest=NEW.admission_digest
               AND admission.adapter_id=NEW.adapter_id
               AND admission.release_version=NEW.release_version
               AND admission.current_status='staged'
               AND source.source_receipt_id=NEW.source_receipt_id
               AND source.source_receipt_digest=NEW.source_receipt_digest
               AND source.admission_digest=NEW.admission_digest
               AND source.adapter_id=NEW.adapter_id
               AND source.release_version=NEW.release_version
               AND source.reopened_sha256=NEW.artifact_sha256
               AND source.artifact_size_bytes=NEW.artifact_size_bytes
               AND signer.key_record_digest=NEW.key_record_digest
               AND signer.key_id=NEW.key_id
               AND signer.source_operator=NEW.source_operator
               AND signer.algorithm=NEW.signature_algorithm
               AND signer.current_status='active')
        BEGIN SELECT RAISE(ABORT, 'Artifact signed provenance lacks exact current authorities'); END;

        CREATE TRIGGER IF NOT EXISTS artifact_signed_provenance_json_projection
        AFTER INSERT ON compute_external_pool_adapter_artifact_signed_provenance_receipts
        WHEN COALESCE(json_extract(NEW.provenance_receipt_json,'$.provenance_receipt_id'),'')<>
             NEW.provenance_receipt_id
          OR COALESCE(json_extract(NEW.provenance_receipt_json,
             '$.provenance_receipt_digest'),'')<>NEW.provenance_receipt_digest
          OR COALESCE(json_extract(NEW.provenance_receipt_json,
             '$.verification_material_digest'),'')<>NEW.verification_material_digest
          OR COALESCE(json_extract(NEW.provenance_receipt_json,
             '$.provenance.binding.admission_id'),'')<>NEW.admission_id
          OR COALESCE(json_extract(NEW.provenance_receipt_json,
             '$.provenance.binding.source_receipt_id'),'')<>NEW.source_receipt_id
          OR COALESCE(json_extract(NEW.provenance_receipt_json,
             '$.provenance.binding.key_record_id'),'')<>NEW.key_record_id
          OR COALESCE(json_extract(NEW.provenance_receipt_json,
             '$.provenance.signature_message_digest'),'')<>NEW.signature_message_digest
          OR COALESCE(json_extract(NEW.provenance_receipt_json,
             '$.provenance.signature_base64'),'')<>NEW.signature_base64
          OR COALESCE(json_extract(NEW.provenance_receipt_json,
             '$.provenance.signature_digest'),'')<>NEW.signature_digest
          OR COALESCE(json_extract(NEW.provenance_receipt_json,
             '$.provenance.verified_by_admin_user_id'),'')<>NEW.verified_by_admin_user_id
          OR COALESCE(json_extract(NEW.provenance_receipt_json,
             '$.provenance.idempotency_scope'),'')<>NEW.idempotency_scope
          OR COALESCE(json_extract(NEW.provenance_receipt_json,
             '$.provenance.idempotency_key'),'')<>NEW.idempotency_key
        BEGIN SELECT RAISE(ABORT, 'Artifact signed-provenance JSON projection mismatch'); END;
        "#,
    )?;
    Ok(())
}
