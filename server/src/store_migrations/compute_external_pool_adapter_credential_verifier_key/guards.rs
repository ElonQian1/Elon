use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS credential_verifier_key_root_no_update BEFORE UPDATE ON compute_external_pool_adapter_credential_verifier_keys BEGIN SELECT RAISE(ABORT,'credential-verifier-key roots are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verifier_key_root_no_delete BEFORE DELETE ON compute_external_pool_adapter_credential_verifier_keys BEGIN SELECT RAISE(ABORT,'credential-verifier-key roots are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verifier_key_revocation_no_update BEFORE UPDATE ON compute_external_pool_adapter_credential_verifier_key_revocations BEGIN SELECT RAISE(ABORT,'credential-verifier-key revocations are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verifier_key_revocation_no_delete BEFORE DELETE ON compute_external_pool_adapter_credential_verifier_key_revocations BEGIN SELECT RAISE(ABORT,'credential-verifier-key revocations are append-only'); END;

        CREATE TRIGGER IF NOT EXISTS credential_verifier_key_root_no_replace BEFORE INSERT ON compute_external_pool_adapter_credential_verifier_keys
        WHEN EXISTS(
          SELECT 1 FROM compute_external_pool_adapter_credential_verifier_keys old
          WHERE old.key_record_id=NEW.key_record_id
             OR old.key_record_digest=NEW.key_record_digest
             OR old.key_id=NEW.key_id
             OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT,'credential-verifier-key root cannot replace immutable history'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verifier_key_revocation_no_replace BEFORE INSERT ON compute_external_pool_adapter_credential_verifier_key_revocations
        WHEN EXISTS(
          SELECT 1 FROM compute_external_pool_adapter_credential_verifier_key_revocations old
          WHERE old.revocation_receipt_id=NEW.revocation_receipt_id
             OR old.revocation_receipt_digest=NEW.revocation_receipt_digest
             OR old.key_record_id=NEW.key_record_id
             OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT,'credential-verifier-key revocation cannot replace immutable history'); END;

        CREATE TRIGGER IF NOT EXISTS credential_verifier_key_registration_four_eyes BEFORE INSERT ON compute_external_pool_adapter_credential_verifier_keys
        WHEN EXISTS(
          SELECT 1 FROM compute_external_pool_adapter_credential_verifiers verifier
          WHERE verifier.verifier_record_id=NEW.verifier_record_id
            AND verifier.created_by_admin_user_id=NEW.created_by_admin_user_id)
        BEGIN SELECT RAISE(ABORT,'credential-verifier-key registration requires another administrator'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verifier_key_registration_requires_current_verifier BEFORE INSERT ON compute_external_pool_adapter_credential_verifier_keys
        WHEN NOT EXISTS(
          SELECT 1 FROM compute_external_pool_adapter_credential_verifier_current verifier
          WHERE verifier.verifier_record_id=NEW.verifier_record_id
            AND verifier.verifier_record_digest=NEW.verifier_record_digest
            AND verifier.verification_kind=NEW.verification_kind
            AND verifier.verifier_id=NEW.verifier_id
            AND verifier.verifier_revision=NEW.verifier_revision
            AND verifier.verifier_digest=NEW.verifier_digest
            AND verifier.current_status='active')
        BEGIN SELECT RAISE(ABORT,'credential-verifier-key registration requires an exact active verifier'); END;

        CREATE TRIGGER IF NOT EXISTS credential_verifier_key_role_separation BEFORE INSERT ON compute_external_pool_adapter_credential_verifier_keys
        WHEN EXISTS(SELECT 1 FROM compute_external_pool_adapter_artifact_signing_keys WHERE key_id=NEW.key_id)
          OR EXISTS(SELECT 1 FROM compute_external_pool_adapter_scanner_keys WHERE key_id=NEW.key_id)
          OR EXISTS(SELECT 1 FROM compute_external_pool_adapter_sandbox_verifier_keys WHERE key_id=NEW.key_id)
        BEGIN SELECT RAISE(ABORT,'credential verifier keys must be distinct from supplier, scanner, and sandbox verifier keys'); END;
        CREATE TRIGGER IF NOT EXISTS artifact_signing_key_credential_verifier_role_separation BEFORE INSERT ON compute_external_pool_adapter_artifact_signing_keys
        WHEN EXISTS(SELECT 1 FROM compute_external_pool_adapter_credential_verifier_keys WHERE key_id=NEW.key_id)
        BEGIN SELECT RAISE(ABORT,'supplier and credential verifier keys must be distinct'); END;
        CREATE TRIGGER IF NOT EXISTS scanner_key_credential_verifier_role_separation BEFORE INSERT ON compute_external_pool_adapter_scanner_keys
        WHEN EXISTS(SELECT 1 FROM compute_external_pool_adapter_credential_verifier_keys WHERE key_id=NEW.key_id)
        BEGIN SELECT RAISE(ABORT,'scanner and credential verifier keys must be distinct'); END;
        CREATE TRIGGER IF NOT EXISTS sandbox_verifier_key_credential_verifier_role_separation BEFORE INSERT ON compute_external_pool_adapter_sandbox_verifier_keys
        WHEN EXISTS(SELECT 1 FROM compute_external_pool_adapter_credential_verifier_keys WHERE key_id=NEW.key_id)
        BEGIN SELECT RAISE(ABORT,'sandbox and credential verifier keys must be distinct'); END;

        CREATE TRIGGER IF NOT EXISTS credential_verifier_key_root_json_projection AFTER INSERT ON compute_external_pool_adapter_credential_verifier_keys
        WHEN COALESCE(json_extract(NEW.key_record_json,'$.key_record_id'),'')<>NEW.key_record_id
          OR COALESCE(json_extract(NEW.key_record_json,'$.key_record_digest'),'')<>NEW.key_record_digest
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.verifier_record_id'),'')<>NEW.verifier_record_id
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.verifier_record_digest'),'')<>NEW.verifier_record_digest
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.verification_kind'),'')<>NEW.verification_kind
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.verifier_id'),'')<>NEW.verifier_id
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.verifier_revision'),-1)<>NEW.verifier_revision
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.verifier_digest'),'')<>NEW.verifier_digest
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.key_id'),'')<>NEW.key_id
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.public_key_pem'),'')<>NEW.public_key_pem
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.created_by_admin_user_id'),'')<>NEW.created_by_admin_user_id
          OR COALESCE(json_extract(NEW.key_record_json,'$.registration.idempotency_key'),'')<>NEW.idempotency_key
        BEGIN SELECT RAISE(ABORT,'credential-verifier-key root JSON projection mismatch'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verifier_key_revocation_json_projection AFTER INSERT ON compute_external_pool_adapter_credential_verifier_key_revocations
        WHEN COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation_receipt_id'),'')<>NEW.revocation_receipt_id
          OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation_receipt_digest'),'')<>NEW.revocation_receipt_digest
          OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation.key_record_id'),'')<>NEW.key_record_id
          OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation.key_record_digest'),'')<>NEW.key_record_digest
          OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation.verifier_record_id'),'')<>NEW.verifier_record_id
          OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation.verifier_record_digest'),'')<>NEW.verifier_record_digest
          OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation.key_id'),'')<>NEW.key_id
          OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation.revoked_by_admin_user_id'),'')<>NEW.revoked_by_admin_user_id
          OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation.reason'),'')<>NEW.reason
          OR COALESCE(json_extract(NEW.revocation_receipt_json,'$.revocation.idempotency_key'),'')<>NEW.idempotency_key
        BEGIN SELECT RAISE(ABORT,'credential-verifier-key revocation JSON projection mismatch'); END;
        "#,
    )?;
    Ok(())
}
