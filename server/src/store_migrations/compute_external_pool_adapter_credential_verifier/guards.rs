use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS credential_verifier_root_no_update BEFORE UPDATE ON compute_external_pool_adapter_credential_verifiers BEGIN SELECT RAISE(ABORT,'credential-verifier roots are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verifier_root_no_delete BEFORE DELETE ON compute_external_pool_adapter_credential_verifiers BEGIN SELECT RAISE(ABORT,'credential-verifier roots are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verifier_transition_no_update BEFORE UPDATE ON compute_external_pool_adapter_credential_verifier_transitions BEGIN SELECT RAISE(ABORT,'credential-verifier transitions are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verifier_transition_no_delete BEFORE DELETE ON compute_external_pool_adapter_credential_verifier_transitions BEGIN SELECT RAISE(ABORT,'credential-verifier transitions are append-only'); END;

        CREATE TRIGGER IF NOT EXISTS credential_verifier_root_no_replace BEFORE INSERT ON compute_external_pool_adapter_credential_verifiers
        WHEN EXISTS(
          SELECT 1 FROM compute_external_pool_adapter_credential_verifiers old
          WHERE old.verifier_record_id=NEW.verifier_record_id
             OR old.verifier_record_digest=NEW.verifier_record_digest
             OR (old.verification_kind=NEW.verification_kind
                 AND old.verifier_id=NEW.verifier_id
                 AND old.verifier_revision=NEW.verifier_revision)
             OR (old.idempotency_scope=NEW.idempotency_scope
                 AND old.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT,'credential-verifier root cannot replace immutable history'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verifier_transition_no_replace BEFORE INSERT ON compute_external_pool_adapter_credential_verifier_transitions
        WHEN EXISTS(
          SELECT 1 FROM compute_external_pool_adapter_credential_verifier_transitions old
          WHERE old.transition_receipt_id=NEW.transition_receipt_id
             OR old.transition_receipt_digest=NEW.transition_receipt_digest
             OR (old.verifier_record_id=NEW.verifier_record_id
                 AND old.transition_kind=NEW.transition_kind)
             OR (old.idempotency_scope=NEW.idempotency_scope
                 AND old.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT,'credential-verifier transition cannot replace immutable history'); END;

        CREATE TRIGGER IF NOT EXISTS credential_verifier_activation_four_eyes BEFORE INSERT ON compute_external_pool_adapter_credential_verifier_transitions
        WHEN NEW.transition_kind='activation' AND EXISTS(
          SELECT 1 FROM compute_external_pool_adapter_credential_verifiers root
          WHERE root.verifier_record_id=NEW.verifier_record_id AND root.created_by_admin_user_id=NEW.actor_user_id)
        BEGIN SELECT RAISE(ABORT,'credential-verifier activation requires another administrator'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verifier_activation_not_revoked BEFORE INSERT ON compute_external_pool_adapter_credential_verifier_transitions
        WHEN NEW.transition_kind='activation' AND EXISTS(
          SELECT 1 FROM compute_external_pool_adapter_credential_verifier_transitions prior
          WHERE prior.verifier_record_id=NEW.verifier_record_id AND prior.transition_kind='revocation')
        BEGIN SELECT RAISE(ABORT,'revoked credential verifier cannot be activated'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verifier_revocation_requires_activation BEFORE INSERT ON compute_external_pool_adapter_credential_verifier_transitions
        WHEN NEW.transition_kind='revocation' AND NOT EXISTS(
          SELECT 1 FROM compute_external_pool_adapter_credential_verifier_transitions prior
          WHERE prior.verifier_record_id=NEW.verifier_record_id AND prior.transition_kind='activation')
        BEGIN SELECT RAISE(ABORT,'credential-verifier revocation requires activation'); END;

        CREATE TRIGGER IF NOT EXISTS credential_verifier_root_json_projection AFTER INSERT ON compute_external_pool_adapter_credential_verifiers
        WHEN COALESCE(json_extract(NEW.verifier_record_json,'$.verifier_record_id'),'')<>NEW.verifier_record_id
          OR COALESCE(json_extract(NEW.verifier_record_json,'$.verifier_record_digest'),'')<>NEW.verifier_record_digest
          OR COALESCE(json_extract(NEW.verifier_record_json,'$.registration.verification_kind'),'')<>NEW.verification_kind
          OR COALESCE(json_extract(NEW.verifier_record_json,'$.registration.verifier_id'),'')<>NEW.verifier_id
          OR COALESCE(json_extract(NEW.verifier_record_json,'$.registration.verifier_revision'),-1)<>NEW.verifier_revision
          OR COALESCE(json_extract(NEW.verifier_record_json,'$.registration.verifier_digest'),'')<>NEW.verifier_digest
          OR COALESCE(json_extract(NEW.verifier_record_json,'$.registration.verifier_operator'),'')<>NEW.verifier_operator
          OR COALESCE(json_extract(NEW.verifier_record_json,'$.registration.verifier_product'),'')<>NEW.verifier_product
          OR COALESCE(json_extract(NEW.verifier_record_json,'$.registration.idempotency_key'),'')<>NEW.idempotency_key
        BEGIN SELECT RAISE(ABORT,'credential-verifier root JSON projection mismatch'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verifier_transition_json_projection AFTER INSERT ON compute_external_pool_adapter_credential_verifier_transitions
        WHEN COALESCE(json_extract(NEW.transition_receipt_json,'$.transition_receipt_id'),'')<>NEW.transition_receipt_id
          OR COALESCE(json_extract(NEW.transition_receipt_json,'$.transition_receipt_digest'),'')<>NEW.transition_receipt_digest
          OR COALESCE(json_extract(NEW.transition_receipt_json,'$.transition.verifier_record_id'),'')<>NEW.verifier_record_id
          OR COALESCE(json_extract(NEW.transition_receipt_json,'$.transition.verification_kind'),'')<>NEW.verification_kind
          OR COALESCE(json_extract(NEW.transition_receipt_json,'$.transition.verifier_id'),'')<>NEW.verifier_id
          OR COALESCE(json_extract(NEW.transition_receipt_json,'$.transition.verifier_revision'),-1)<>NEW.verifier_revision
          OR COALESCE(json_extract(NEW.transition_receipt_json,'$.transition.verifier_digest'),'')<>NEW.verifier_digest
          OR COALESCE(json_extract(NEW.transition_receipt_json,'$.transition.actor_user_id'),'')<>NEW.actor_user_id
          OR COALESCE(json_extract(NEW.transition_receipt_json,'$.transition.idempotency_key'),'')<>NEW.idempotency_key
          OR (NEW.transition_kind='activation' AND json_extract(NEW.transition_receipt_json,'$.transition.reason') IS NOT NULL)
          OR (NEW.transition_kind='revocation' AND COALESCE(json_extract(NEW.transition_receipt_json,'$.transition.reason'),'')<>NEW.reason)
        BEGIN SELECT RAISE(ABORT,'credential-verifier transition JSON projection mismatch'); END;
        "#,
    )?;
    Ok(())
}
