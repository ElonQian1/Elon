use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        DROP VIEW IF EXISTS compute_external_pool_adapter_sandbox_verifier_key_current;
        CREATE VIEW compute_external_pool_adapter_sandbox_verifier_key_current AS
        SELECT root.key_record_id,root.key_record_digest,root.key_id,root.verifier_operator,root.verifier_product,
          CASE WHEN revoked.transition_receipt_id IS NOT NULL THEN 'revoked'
               WHEN active.transition_receipt_id IS NOT NULL THEN 'active'
               ELSE 'pending_activation' END AS current_status,
          active.transition_receipt_id AS activation_receipt_id,
          active.transition_receipt_digest AS activation_receipt_digest,
          revoked.transition_receipt_id AS revocation_receipt_id,
          revoked.transition_receipt_digest AS revocation_receipt_digest
        FROM compute_external_pool_adapter_sandbox_verifier_keys root
        LEFT JOIN compute_external_pool_adapter_sandbox_verifier_key_transitions active
          ON active.key_record_id=root.key_record_id AND active.transition_kind='activation'
        LEFT JOIN compute_external_pool_adapter_sandbox_verifier_key_transitions revoked
          ON revoked.key_record_id=root.key_record_id AND revoked.transition_kind='revocation';
        "#,
    )?;
    Ok(())
}
