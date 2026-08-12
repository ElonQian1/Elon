use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        DROP VIEW IF EXISTS compute_external_pool_adapter_sandbox_conformance_current;
        CREATE VIEW compute_external_pool_adapter_sandbox_conformance_current AS
        SELECT report.sandbox_conformance_receipt_id,
               report.sandbox_conformance_receipt_digest,
               report.admission_id,
               CASE WHEN vulnerability.current_status='verified_current'
                          AND verifier.current_status='active'
                          AND julianday(report.report_expires_at)>julianday('now')
                    THEN 'verified_current' ELSE 'historical_only' END AS current_status,
               vulnerability.current_status AS vulnerability_report_status,
               verifier.current_status AS sandbox_verifier_key_status,
               CASE WHEN julianday(report.report_expires_at)>julianday('now')
                    THEN 'current' ELSE 'expired' END AS report_validity_status
          FROM compute_external_pool_adapter_sandbox_conformance_reports report
          JOIN compute_external_pool_adapter_vulnerability_report_current vulnerability
            ON vulnerability.admission_id=report.admission_id
           AND vulnerability.vulnerability_report_receipt_id=report.vulnerability_report_receipt_id
           AND vulnerability.vulnerability_report_receipt_digest=report.vulnerability_report_receipt_digest
          JOIN compute_external_pool_adapter_sandbox_verifier_key_current verifier
            ON verifier.key_record_id=report.sandbox_verifier_key_record_id
           AND verifier.key_record_digest=report.sandbox_verifier_key_record_digest
           AND verifier.key_id=report.sandbox_verifier_key_id;
        "#,
    )?;
    Ok(())
}
