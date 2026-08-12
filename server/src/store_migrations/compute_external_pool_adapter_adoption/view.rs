use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        DROP VIEW IF EXISTS compute_external_pool_adapter_adoption_current;
        CREATE VIEW compute_external_pool_adapter_adoption_current AS
        SELECT adoption.adoption_receipt_id,
               adoption.adoption_receipt_digest,
               CASE WHEN terminal.terminal_receipt_id IS NULL
                          AND sandbox.current_status='verified_current'
                          AND credential.current_status='verified_current'
                    THEN 'adopted_current' ELSE 'historical_only' END AS current_status,
               COALESCE(sandbox.current_status,'not_current') AS sandbox_conformance_status,
               COALESCE(credential.current_status,'not_current') AS credential_verification_status,
               CASE WHEN terminal.terminal_receipt_id IS NULL THEN 'none' ELSE 'revoked' END AS terminal_status
          FROM compute_external_pool_adapter_adoption_receipts adoption
          LEFT JOIN compute_external_pool_adapter_adoption_terminal_receipts terminal
            ON terminal.adoption_receipt_id=adoption.adoption_receipt_id
           AND terminal.adoption_receipt_digest=adoption.adoption_receipt_digest
          LEFT JOIN compute_external_pool_adapter_sandbox_conformance_current sandbox
            ON sandbox.admission_id=adoption.admission_id
           AND sandbox.sandbox_conformance_receipt_id=adoption.sandbox_conformance_receipt_id
           AND sandbox.sandbox_conformance_receipt_digest=adoption.sandbox_conformance_receipt_digest
          LEFT JOIN compute_external_pool_adapter_credential_verification_current credential
            ON credential.credential_verification_receipt_id=adoption.credential_verification_receipt_id
           AND credential.credential_verification_receipt_digest=adoption.credential_verification_receipt_digest;
        "#,
    )?;
    Ok(())
}
