use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        DROP VIEW IF EXISTS compute_external_pool_adapter_artifact_package_current;
        CREATE VIEW compute_external_pool_adapter_artifact_package_current AS
        SELECT package.package_receipt_id, package.package_receipt_digest,
               package.admission_id, package.admission_digest,
               package.provenance_receipt_id, package.provenance_receipt_digest,
               CASE WHEN provenance.current_status='verified_current'
                    THEN 'verified_current' ELSE 'historical_only' END AS current_status,
               provenance.admission_current_status,
               provenance.signer_current_status
          FROM compute_external_pool_adapter_artifact_package_receipts package
          JOIN compute_external_pool_adapter_artifact_signed_provenance_current provenance
            ON provenance.provenance_receipt_id=package.provenance_receipt_id
           AND provenance.provenance_receipt_digest=package.provenance_receipt_digest;
        "#,
    )?;
    Ok(())
}
