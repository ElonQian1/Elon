use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(r#"
    DROP VIEW IF EXISTS compute_external_pool_adapter_artifact_security_current;
    CREATE VIEW compute_external_pool_adapter_artifact_security_current AS
    SELECT security.security_receipt_id,security.security_receipt_digest,
           security.admission_id,security.package_receipt_id,security.package_receipt_digest,
           CASE WHEN package.current_status='verified_current' THEN 'verified_current' ELSE 'historical_only' END AS current_status,
           package.admission_current_status,package.signer_current_status
      FROM compute_external_pool_adapter_artifact_security_receipts security
      JOIN compute_external_pool_adapter_artifact_package_current package
        ON package.package_receipt_id=security.package_receipt_id
       AND package.package_receipt_digest=security.package_receipt_digest;
    "#)?;
    Ok(())
}
