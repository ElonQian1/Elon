use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        DROP VIEW IF EXISTS compute_external_pool_adapter_credential_verifier_key_current;
        CREATE VIEW compute_external_pool_adapter_credential_verifier_key_current AS
        SELECT root.key_record_id,root.key_record_digest,root.verifier_record_id,
          root.verifier_record_digest,root.verification_kind,root.verifier_id,
          root.verifier_revision,root.verifier_digest,root.key_id,
          CASE WHEN verifier.current_status<>'active' THEN 'verifier_not_current'
               WHEN revoked.revocation_receipt_id IS NOT NULL THEN 'revoked'
               ELSE 'active' END AS current_status,
          revoked.revocation_receipt_id,revoked.revocation_receipt_digest
        FROM compute_external_pool_adapter_credential_verifier_keys root
        JOIN compute_external_pool_adapter_credential_verifier_current verifier
          ON verifier.verifier_record_id=root.verifier_record_id
         AND verifier.verifier_record_digest=root.verifier_record_digest
         AND verifier.verification_kind=root.verification_kind
         AND verifier.verifier_id=root.verifier_id
         AND verifier.verifier_revision=root.verifier_revision
         AND verifier.verifier_digest=root.verifier_digest
        LEFT JOIN compute_external_pool_adapter_credential_verifier_key_revocations revoked
          ON revoked.key_record_id=root.key_record_id;
        "#,
    )?;
    Ok(())
}
