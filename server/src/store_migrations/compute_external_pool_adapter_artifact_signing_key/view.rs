use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        DROP VIEW IF EXISTS compute_external_pool_adapter_artifact_signing_key_current;
        CREATE VIEW compute_external_pool_adapter_artifact_signing_key_current AS
        SELECT root.key_record_id,
               root.key_record_digest,
               root.key_id,
               root.source_operator,
               root.algorithm,
               root.created_by_admin_user_id,
               root.created_at,
               CASE
                   WHEN revoked.revocation_receipt_id IS NOT NULL THEN 'revoked'
                   WHEN active.activation_receipt_id IS NOT NULL THEN 'active'
                   ELSE 'pending_activation'
               END AS current_status,
               active.activation_receipt_id,
               active.activation_receipt_digest,
               active.activated_by_admin_user_id,
               active.occurred_at AS activated_at,
               revoked.revocation_receipt_id,
               revoked.revocation_receipt_digest,
               revoked.revoked_by_admin_user_id,
               revoked.reason AS revocation_reason,
               revoked.occurred_at AS revoked_at
          FROM compute_external_pool_adapter_artifact_signing_keys root
          LEFT JOIN compute_external_pool_adapter_artifact_signing_key_activations active
            ON active.key_record_id=root.key_record_id
          LEFT JOIN compute_external_pool_adapter_artifact_signing_key_revocations revoked
            ON revoked.key_record_id=root.key_record_id;
        "#,
    )?;
    Ok(())
}
