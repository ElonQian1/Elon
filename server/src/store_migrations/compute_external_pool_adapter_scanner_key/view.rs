use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(r#"
    DROP VIEW IF EXISTS compute_external_pool_adapter_scanner_key_current;
    CREATE VIEW compute_external_pool_adapter_scanner_key_current AS
    SELECT root.key_record_id,root.key_record_digest,root.key_id,root.scanner_operator,root.scanner_product,
      CASE WHEN revoked.revocation_receipt_id IS NOT NULL THEN 'revoked'
           WHEN active.activation_receipt_id IS NOT NULL THEN 'active'
           ELSE 'pending_activation' END AS current_status,
      active.activation_receipt_id,active.activation_receipt_digest,
      revoked.revocation_receipt_id,revoked.revocation_receipt_digest
    FROM compute_external_pool_adapter_scanner_keys root
    LEFT JOIN compute_external_pool_adapter_scanner_key_activations active ON active.key_record_id=root.key_record_id
    LEFT JOIN compute_external_pool_adapter_scanner_key_revocations revoked ON revoked.key_record_id=root.key_record_id;
    "#)?;
    Ok(())
}
