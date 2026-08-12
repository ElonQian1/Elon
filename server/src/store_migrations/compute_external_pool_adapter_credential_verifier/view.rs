use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        DROP VIEW IF EXISTS compute_external_pool_adapter_credential_verifier_current;
        CREATE VIEW compute_external_pool_adapter_credential_verifier_current AS
        SELECT root.verifier_record_id,root.verifier_record_digest,
          root.verification_kind,root.verifier_id,root.verifier_revision,root.verifier_digest,
          root.verifier_operator,root.verifier_product,
          CASE WHEN revoked.transition_receipt_id IS NOT NULL THEN 'revoked'
               WHEN active.transition_receipt_id IS NOT NULL THEN 'active'
               ELSE 'pending_activation' END AS current_status,
          active.transition_receipt_id AS activation_receipt_id,
          active.transition_receipt_digest AS activation_receipt_digest,
          revoked.transition_receipt_id AS revocation_receipt_id,
          revoked.transition_receipt_digest AS revocation_receipt_digest
        FROM compute_external_pool_adapter_credential_verifiers root
        LEFT JOIN compute_external_pool_adapter_credential_verifier_transitions active
          ON active.verifier_record_id=root.verifier_record_id AND active.transition_kind='activation'
        LEFT JOIN compute_external_pool_adapter_credential_verifier_transitions revoked
          ON revoked.verifier_record_id=root.verifier_record_id AND revoked.transition_kind='revocation';
        "#,
    )?;
    Ok(())
}
