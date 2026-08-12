use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS sandbox_verifier_key_root_no_update BEFORE UPDATE ON compute_external_pool_adapter_sandbox_verifier_keys BEGIN SELECT RAISE(ABORT,'sandbox-verifier-key roots are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS sandbox_verifier_key_root_no_delete BEFORE DELETE ON compute_external_pool_adapter_sandbox_verifier_keys BEGIN SELECT RAISE(ABORT,'sandbox-verifier-key roots are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS sandbox_verifier_key_transition_no_update BEFORE UPDATE ON compute_external_pool_adapter_sandbox_verifier_key_transitions BEGIN SELECT RAISE(ABORT,'sandbox-verifier-key transitions are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS sandbox_verifier_key_transition_no_delete BEFORE DELETE ON compute_external_pool_adapter_sandbox_verifier_key_transitions BEGIN SELECT RAISE(ABORT,'sandbox-verifier-key transitions are append-only'); END;

        CREATE TRIGGER IF NOT EXISTS sandbox_verifier_key_role_separation BEFORE INSERT ON compute_external_pool_adapter_sandbox_verifier_keys
        WHEN EXISTS(SELECT 1 FROM compute_external_pool_adapter_artifact_signing_keys WHERE key_id=NEW.key_id)
          OR EXISTS(SELECT 1 FROM compute_external_pool_adapter_scanner_keys WHERE key_id=NEW.key_id)
        BEGIN SELECT RAISE(ABORT,'sandbox verifier, supplier, and scanner keys must be distinct'); END;
        CREATE TRIGGER IF NOT EXISTS artifact_signing_key_sandbox_verifier_role_separation BEFORE INSERT ON compute_external_pool_adapter_artifact_signing_keys
        WHEN EXISTS(SELECT 1 FROM compute_external_pool_adapter_sandbox_verifier_keys WHERE key_id=NEW.key_id)
        BEGIN SELECT RAISE(ABORT,'supplier and sandbox verifier keys must be distinct'); END;
        CREATE TRIGGER IF NOT EXISTS scanner_key_sandbox_verifier_role_separation BEFORE INSERT ON compute_external_pool_adapter_scanner_keys
        WHEN EXISTS(SELECT 1 FROM compute_external_pool_adapter_sandbox_verifier_keys WHERE key_id=NEW.key_id)
        BEGIN SELECT RAISE(ABORT,'scanner and sandbox verifier keys must be distinct'); END;

        CREATE TRIGGER IF NOT EXISTS sandbox_verifier_key_activation_four_eyes BEFORE INSERT ON compute_external_pool_adapter_sandbox_verifier_key_transitions
        WHEN NEW.transition_kind='activation' AND EXISTS(
          SELECT 1 FROM compute_external_pool_adapter_sandbox_verifier_keys root
          WHERE root.key_record_id=NEW.key_record_id AND root.created_by_admin_user_id=NEW.actor_user_id)
        BEGIN SELECT RAISE(ABORT,'sandbox-verifier-key activation requires another administrator'); END;
        CREATE TRIGGER IF NOT EXISTS sandbox_verifier_key_activation_not_revoked BEFORE INSERT ON compute_external_pool_adapter_sandbox_verifier_key_transitions
        WHEN NEW.transition_kind='activation' AND EXISTS(
          SELECT 1 FROM compute_external_pool_adapter_sandbox_verifier_key_transitions prior
          WHERE prior.key_record_id=NEW.key_record_id AND prior.transition_kind='revocation')
        BEGIN SELECT RAISE(ABORT,'revoked sandbox verifier key cannot be activated'); END;
        CREATE TRIGGER IF NOT EXISTS sandbox_verifier_key_revocation_requires_activation BEFORE INSERT ON compute_external_pool_adapter_sandbox_verifier_key_transitions
        WHEN NEW.transition_kind='revocation' AND NOT EXISTS(
          SELECT 1 FROM compute_external_pool_adapter_sandbox_verifier_key_transitions prior
          WHERE prior.key_record_id=NEW.key_record_id AND prior.transition_kind='activation')
        BEGIN SELECT RAISE(ABORT,'sandbox-verifier-key revocation requires activation'); END;

        CREATE TRIGGER IF NOT EXISTS sandbox_verifier_key_root_json_projection AFTER INSERT ON compute_external_pool_adapter_sandbox_verifier_keys
        WHEN COALESCE(json_extract(NEW.key_record_json,'$.key_record_id'),'')<>NEW.key_record_id
          OR COALESCE(json_extract(NEW.key_record_json,'$.key_record_digest'),'')<>NEW.key_record_digest
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.key_id'),'')<>NEW.key_id
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.verifier_operator'),'')<>NEW.verifier_operator
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.verifier_product'),'')<>NEW.verifier_product
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.public_key_pem'),'')<>NEW.public_key_pem
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.idempotency_key'),'')<>NEW.idempotency_key
        BEGIN SELECT RAISE(ABORT,'sandbox-verifier-key root JSON projection mismatch'); END;
        CREATE TRIGGER IF NOT EXISTS sandbox_verifier_key_transition_json_projection AFTER INSERT ON compute_external_pool_adapter_sandbox_verifier_key_transitions
        WHEN COALESCE(json_extract(NEW.transition_receipt_json,'$.transition_receipt_id'),'')<>NEW.transition_receipt_id
          OR COALESCE(json_extract(NEW.transition_receipt_json,'$.transition_receipt_digest'),'')<>NEW.transition_receipt_digest
          OR COALESCE(json_extract(NEW.transition_receipt_json,'$.transition.key_record_id'),'')<>NEW.key_record_id
          OR COALESCE(json_extract(NEW.transition_receipt_json,'$.transition.actor_user_id'),'')<>NEW.actor_user_id
          OR COALESCE(json_extract(NEW.transition_receipt_json,'$.transition.idempotency_key'),'')<>NEW.idempotency_key
          OR (NEW.transition_kind='activation' AND json_extract(NEW.transition_receipt_json,'$.transition.reason') IS NOT NULL)
          OR (NEW.transition_kind='revocation' AND COALESCE(json_extract(NEW.transition_receipt_json,'$.transition.reason'),'')<>NEW.reason)
        BEGIN SELECT RAISE(ABORT,'sandbox-verifier-key transition JSON projection mismatch'); END;
        "#,
    )?;
    Ok(())
}
