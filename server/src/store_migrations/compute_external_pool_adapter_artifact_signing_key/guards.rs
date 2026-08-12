use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS signing_key_root_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_artifact_signing_keys
        BEGIN SELECT RAISE(ABORT, 'signing-key roots are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS signing_key_root_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_artifact_signing_keys
        BEGIN SELECT RAISE(ABORT, 'signing-key roots are append-only'); END;

        CREATE TRIGGER IF NOT EXISTS signing_key_activation_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_artifact_signing_key_activations
        BEGIN SELECT RAISE(ABORT, 'signing-key activations are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS signing_key_activation_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_artifact_signing_key_activations
        BEGIN SELECT RAISE(ABORT, 'signing-key activations are append-only'); END;

        CREATE TRIGGER IF NOT EXISTS signing_key_revocation_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_artifact_signing_key_revocations
        BEGIN SELECT RAISE(ABORT, 'signing-key revocations are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS signing_key_revocation_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_artifact_signing_key_revocations
        BEGIN SELECT RAISE(ABORT, 'signing-key revocations are append-only'); END;

        CREATE TRIGGER IF NOT EXISTS signing_key_activation_four_eyes
        BEFORE INSERT ON compute_external_pool_adapter_artifact_signing_key_activations
        WHEN EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_artifact_signing_keys root
             WHERE root.key_record_id=NEW.key_record_id
               AND root.created_by_admin_user_id=NEW.activated_by_admin_user_id)
        BEGIN SELECT RAISE(ABORT, 'signing-key activation requires another administrator'); END;

        CREATE TRIGGER IF NOT EXISTS signing_key_activation_not_after_revocation
        BEFORE INSERT ON compute_external_pool_adapter_artifact_signing_key_activations
        WHEN EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_artifact_signing_key_revocations revoked
             WHERE revoked.key_record_id=NEW.key_record_id)
        BEGIN SELECT RAISE(ABORT, 'revoked signing key cannot be activated'); END;

        CREATE TRIGGER IF NOT EXISTS signing_key_revocation_requires_activation
        BEFORE INSERT ON compute_external_pool_adapter_artifact_signing_key_revocations
        WHEN NOT EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_artifact_signing_key_activations active
             WHERE active.key_record_id=NEW.key_record_id)
        BEGIN SELECT RAISE(ABORT, 'signing-key revocation requires activation'); END;

        CREATE TRIGGER IF NOT EXISTS signing_key_root_json_projection
        AFTER INSERT ON compute_external_pool_adapter_artifact_signing_keys
        WHEN COALESCE(json_extract(NEW.key_record_json,'$.key_record_id'),'')<>NEW.key_record_id
          OR COALESCE(json_extract(NEW.key_record_json,'$.key_record_digest'),'')<>NEW.key_record_digest
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration_material_digest'),'')<>
             NEW.registration_material_digest
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.source_operator'),'')<>
             NEW.source_operator
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.key_id'),'')<>NEW.key_id
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.public_key_pem'),'')<>
             NEW.public_key_pem
          OR COALESCE(json_extract(NEW.key_record_json,
             '$.registration.created_by_admin_user_id'),'')<>NEW.created_by_admin_user_id
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.idempotency_scope'),'')<>
             NEW.idempotency_scope
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.idempotency_key'),'')<>
             NEW.idempotency_key
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.created_at'),'')<>
             NEW.created_at
        BEGIN SELECT RAISE(ABORT, 'signing-key root JSON projection mismatch'); END;

        CREATE TRIGGER IF NOT EXISTS signing_key_activation_json_projection
        AFTER INSERT ON compute_external_pool_adapter_artifact_signing_key_activations
        WHEN COALESCE(json_extract(NEW.activation_receipt_json,'$.activation_receipt_id'),'')<>
             NEW.activation_receipt_id
          OR COALESCE(json_extract(NEW.activation_receipt_json,'$.activation_receipt_digest'),'')<>
             NEW.activation_receipt_digest
          OR COALESCE(json_extract(NEW.activation_receipt_json,'$.activation.key_record_id'),'')<>
             NEW.key_record_id
          OR COALESCE(json_extract(NEW.activation_receipt_json,
             '$.activation.activated_by_admin_user_id'),'')<>NEW.activated_by_admin_user_id
          OR COALESCE(json_extract(NEW.activation_receipt_json,'$.activation.idempotency_scope'),'')<>
             NEW.idempotency_scope
          OR COALESCE(json_extract(NEW.activation_receipt_json,'$.activation.idempotency_key'),'')<>
             NEW.idempotency_key
        BEGIN SELECT RAISE(ABORT, 'signing-key activation JSON projection mismatch'); END;

        CREATE TRIGGER IF NOT EXISTS signing_key_revocation_json_projection
        AFTER INSERT ON compute_external_pool_adapter_artifact_signing_key_revocations
        WHEN COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation_receipt_id'),'')<>
             NEW.revocation_receipt_id
          OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation_receipt_digest'),'')<>
             NEW.revocation_receipt_digest
          OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation.key_record_id'),'')<>
             NEW.key_record_id
          OR COALESCE(json_extract(NEW.revocation_receipt_json,
             '$.revocation.revoked_by_admin_user_id'),'')<>NEW.revoked_by_admin_user_id
          OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation.reason'),'')<>NEW.reason
          OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation.idempotency_scope'),'')<>
             NEW.idempotency_scope
          OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation.idempotency_key'),'')<>
             NEW.idempotency_key
        BEGIN SELECT RAISE(ABORT, 'signing-key revocation JSON projection mismatch'); END;
        "#,
    )?;
    Ok(())
}
