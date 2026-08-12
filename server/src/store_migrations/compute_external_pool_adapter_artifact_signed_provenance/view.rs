use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        DROP VIEW IF EXISTS compute_external_pool_adapter_artifact_signed_provenance_current;
        CREATE VIEW compute_external_pool_adapter_artifact_signed_provenance_current AS
        SELECT provenance.provenance_receipt_id,
               provenance.provenance_receipt_digest,
               provenance.admission_id,
               provenance.admission_digest,
               provenance.source_receipt_id,
               provenance.source_receipt_digest,
               provenance.key_record_id,
               provenance.key_record_digest,
               provenance.key_id,
               CASE
                   WHEN admission.current_status='staged'
                    AND signer.current_status='active' THEN 'verified_current'
                   ELSE 'historical_only'
               END AS current_status,
               admission.current_status AS admission_current_status,
               signer.current_status AS signer_current_status
          FROM compute_external_pool_adapter_artifact_signed_provenance_receipts provenance
          JOIN compute_external_pool_adapter_release_admission_current admission
            ON admission.admission_id=provenance.admission_id
           AND admission.admission_digest=provenance.admission_digest
          JOIN compute_external_pool_adapter_artifact_signing_key_current signer
            ON signer.key_record_id=provenance.key_record_id
           AND signer.key_record_digest=provenance.key_record_digest
           AND signer.key_id=provenance.key_id;
        "#,
    )?;
    Ok(())
}
