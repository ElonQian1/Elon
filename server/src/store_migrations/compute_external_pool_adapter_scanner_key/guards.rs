use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(r#"
    CREATE TRIGGER IF NOT EXISTS scanner_key_root_no_update BEFORE UPDATE ON compute_external_pool_adapter_scanner_keys BEGIN SELECT RAISE(ABORT,'scanner-key roots are immutable'); END;
    CREATE TRIGGER IF NOT EXISTS scanner_key_root_no_delete BEFORE DELETE ON compute_external_pool_adapter_scanner_keys BEGIN SELECT RAISE(ABORT,'scanner-key roots are append-only'); END;
    CREATE TRIGGER IF NOT EXISTS scanner_key_activation_no_update BEFORE UPDATE ON compute_external_pool_adapter_scanner_key_activations BEGIN SELECT RAISE(ABORT,'scanner-key activations are immutable'); END;
    CREATE TRIGGER IF NOT EXISTS scanner_key_activation_no_delete BEFORE DELETE ON compute_external_pool_adapter_scanner_key_activations BEGIN SELECT RAISE(ABORT,'scanner-key activations are append-only'); END;
    CREATE TRIGGER IF NOT EXISTS scanner_key_revocation_no_update BEFORE UPDATE ON compute_external_pool_adapter_scanner_key_revocations BEGIN SELECT RAISE(ABORT,'scanner-key revocations are immutable'); END;
    CREATE TRIGGER IF NOT EXISTS scanner_key_revocation_no_delete BEFORE DELETE ON compute_external_pool_adapter_scanner_key_revocations BEGIN SELECT RAISE(ABORT,'scanner-key revocations are append-only'); END;

    CREATE TRIGGER IF NOT EXISTS scanner_key_role_separation BEFORE INSERT ON compute_external_pool_adapter_scanner_keys
    WHEN EXISTS(SELECT 1 FROM compute_external_pool_adapter_artifact_signing_keys WHERE key_id=NEW.key_id)
    BEGIN SELECT RAISE(ABORT,'scanner and supplier signing keys must be distinct'); END;
    CREATE TRIGGER IF NOT EXISTS artifact_signing_key_scanner_role_separation BEFORE INSERT ON compute_external_pool_adapter_artifact_signing_keys
    WHEN EXISTS(SELECT 1 FROM compute_external_pool_adapter_scanner_keys WHERE key_id=NEW.key_id)
    BEGIN SELECT RAISE(ABORT,'supplier and scanner signing keys must be distinct'); END;
    CREATE TRIGGER IF NOT EXISTS scanner_key_activation_four_eyes BEFORE INSERT ON compute_external_pool_adapter_scanner_key_activations
    WHEN EXISTS(SELECT 1 FROM compute_external_pool_adapter_scanner_keys root WHERE root.key_record_id=NEW.key_record_id AND root.created_by_admin_user_id=NEW.activated_by_admin_user_id)
    BEGIN SELECT RAISE(ABORT,'scanner-key activation requires another administrator'); END;
    CREATE TRIGGER IF NOT EXISTS scanner_key_activation_not_revoked BEFORE INSERT ON compute_external_pool_adapter_scanner_key_activations
    WHEN EXISTS(SELECT 1 FROM compute_external_pool_adapter_scanner_key_revocations WHERE key_record_id=NEW.key_record_id)
    BEGIN SELECT RAISE(ABORT,'revoked scanner key cannot be activated'); END;
    CREATE TRIGGER IF NOT EXISTS scanner_key_revocation_requires_activation BEFORE INSERT ON compute_external_pool_adapter_scanner_key_revocations
    WHEN NOT EXISTS(SELECT 1 FROM compute_external_pool_adapter_scanner_key_activations WHERE key_record_id=NEW.key_record_id)
    BEGIN SELECT RAISE(ABORT,'scanner-key revocation requires activation'); END;

    CREATE TRIGGER IF NOT EXISTS scanner_key_root_json_projection AFTER INSERT ON compute_external_pool_adapter_scanner_keys
    WHEN COALESCE(json_extract(NEW.key_record_json,'$.key_record_id'),'')<>NEW.key_record_id
      OR COALESCE(json_extract(NEW.key_record_json,'$.key_record_digest'),'')<>NEW.key_record_digest
      OR COALESCE(json_extract(NEW.key_record_json,'$.registration.key_id'),'')<>NEW.key_id
      OR COALESCE(json_extract(NEW.key_record_json,'$.registration.scanner_operator'),'')<>NEW.scanner_operator
      OR COALESCE(json_extract(NEW.key_record_json,'$.registration.scanner_product'),'')<>NEW.scanner_product
      OR COALESCE(json_extract(NEW.key_record_json,'$.registration.public_key_pem'),'')<>NEW.public_key_pem
      OR COALESCE(json_extract(NEW.key_record_json,'$.registration.idempotency_key'),'')<>NEW.idempotency_key
    BEGIN SELECT RAISE(ABORT,'scanner-key root JSON projection mismatch'); END;
    CREATE TRIGGER IF NOT EXISTS scanner_key_activation_json_projection AFTER INSERT ON compute_external_pool_adapter_scanner_key_activations
    WHEN COALESCE(json_extract(NEW.activation_receipt_json,'$.activation_receipt_id'),'')<>NEW.activation_receipt_id
      OR COALESCE(json_extract(NEW.activation_receipt_json,'$.activation_receipt_digest'),'')<>NEW.activation_receipt_digest
      OR COALESCE(json_extract(NEW.activation_receipt_json,'$.activation.key_record_id'),'')<>NEW.key_record_id
      OR COALESCE(json_extract(NEW.activation_receipt_json,'$.activation.activated_by_admin_user_id'),'')<>NEW.activated_by_admin_user_id
      OR COALESCE(json_extract(NEW.activation_receipt_json,'$.activation.idempotency_key'),'')<>NEW.idempotency_key
    BEGIN SELECT RAISE(ABORT,'scanner-key activation JSON projection mismatch'); END;
    CREATE TRIGGER IF NOT EXISTS scanner_key_revocation_json_projection AFTER INSERT ON compute_external_pool_adapter_scanner_key_revocations
    WHEN COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation_receipt_id'),'')<>NEW.revocation_receipt_id
      OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation_receipt_digest'),'')<>NEW.revocation_receipt_digest
      OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation.key_record_id'),'')<>NEW.key_record_id
      OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation.reason'),'')<>NEW.reason
      OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation.idempotency_key'),'')<>NEW.idempotency_key
    BEGIN SELECT RAISE(ABORT,'scanner-key revocation JSON projection mismatch'); END;
    "#)?;
    Ok(())
}
