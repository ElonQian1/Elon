use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS credential_verification_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_credential_verification_receipts
        BEGIN SELECT RAISE(ABORT,'credential verification receipts are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verification_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_credential_verification_receipts
        BEGIN SELECT RAISE(ABORT,'credential verification receipts are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verification_no_replace
        BEFORE INSERT ON compute_external_pool_adapter_credential_verification_receipts
        WHEN EXISTS(
          SELECT 1 FROM compute_external_pool_adapter_credential_verification_receipts old
          WHERE old.credential_verification_receipt_id=NEW.credential_verification_receipt_id
             OR old.credential_verification_receipt_digest=NEW.credential_verification_receipt_digest
             OR old.verifier_report_id=NEW.verifier_report_id
             OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT,'credential verification cannot replace immutable history'); END;

        CREATE TRIGGER IF NOT EXISTS credential_verification_exact_onboarding
        BEFORE INSERT ON compute_external_pool_adapter_credential_verification_receipts
        WHEN NOT EXISTS(
          SELECT 1 FROM compute_external_pool_onboarding_applications app
          WHERE app.application_id=NEW.application_id
            AND app.application_digest=NEW.application_digest
            AND app.provider_id=NEW.provider_id
            AND app.target_provider_policy_revision=NEW.provider_policy_revision
            AND app.target_provider_digest=NEW.provider_digest
            AND app.adapter_id=NEW.adapter_id
            AND app.adapter_release_version=NEW.adapter_release_version
            AND app.adapter_config_revision=NEW.adapter_config_revision
            AND app.adapter_config_digest=NEW.adapter_config_digest)
        BEGIN SELECT RAISE(ABORT,'credential verification requires exact onboarding lineage'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verification_exact_admission
        BEFORE INSERT ON compute_external_pool_adapter_credential_verification_receipts
        WHEN NOT EXISTS(
          SELECT 1 FROM compute_external_pool_adapter_release_admissions admission
          WHERE admission.admission_id=NEW.admission_id
            AND admission.admission_digest=NEW.admission_digest
            AND admission.adapter_id=NEW.adapter_id
            AND admission.release_version=NEW.adapter_release_version)
        BEGIN SELECT RAISE(ABORT,'credential verification requires exact admission lineage'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verification_requires_current_inputs
        BEFORE INSERT ON compute_external_pool_adapter_credential_verification_receipts
        WHEN NOT EXISTS(
          SELECT 1 FROM compute_providers provider
          WHERE provider.provider_id=NEW.provider_id
            AND provider.current_policy_revision=NEW.provider_policy_revision
            AND provider.current_provider_digest=NEW.provider_digest
            AND provider.status='registering')
          OR NOT EXISTS(
          SELECT 1 FROM compute_external_pool_adapter_release_admission_current admission
          WHERE admission.admission_id=NEW.admission_id
            AND admission.admission_digest=NEW.admission_digest
            AND admission.current_status='staged')
          OR NOT EXISTS(
          SELECT 1 FROM compute_external_pool_adapter_credential_verifier_key_current verifier
          WHERE verifier.key_record_id=NEW.credential_verifier_key_record_id
            AND verifier.key_record_digest=NEW.credential_verifier_key_record_digest
            AND verifier.verifier_record_id=NEW.credential_verifier_record_id
            AND verifier.verifier_record_digest=NEW.credential_verifier_record_digest
            AND verifier.key_id=NEW.credential_verifier_key_id
            AND verifier.current_status='active')
        BEGIN SELECT RAISE(ABORT,'credential verification requires current exact inputs'); END;
        CREATE TRIGGER IF NOT EXISTS credential_verification_fresh_report
        BEFORE INSERT ON compute_external_pool_adapter_credential_verification_receipts
        WHEN julianday(NEW.report_expires_at)<=julianday(NEW.verified_at)
        BEGIN SELECT RAISE(ABORT,'credential verification report expired before persistence'); END;

        CREATE TRIGGER IF NOT EXISTS credential_verification_json_projection
        AFTER INSERT ON compute_external_pool_adapter_credential_verification_receipts
        WHEN COALESCE(json_extract(NEW.receipt_json,'$.schema'),'')<>'compute_federation.external_pool_adapter_credential_verification_receipt.v1'
          OR COALESCE(json_extract(NEW.receipt_json,'$.credential_verification_receipt_id'),'')<>NEW.credential_verification_receipt_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.credential_verification_receipt_digest'),'')<>NEW.credential_verification_receipt_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.verification_material_digest'),'')<>NEW.verification_material_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.verification.binding.application_id'),'')<>NEW.application_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.verification.binding.application_digest'),'')<>NEW.application_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.verification.binding.provider_id'),'')<>NEW.provider_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.verification.binding.provider_digest'),'')<>NEW.provider_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.verification.binding.credential_locator_commitment'),'')<>NEW.credential_locator_commitment
          OR COALESCE(json_extract(NEW.receipt_json,'$.verification.binding.admission_id'),'')<>NEW.admission_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.verification.binding.admission_digest'),'')<>NEW.admission_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.verification.binding.credential_verifier_key_record_id'),'')<>NEW.credential_verifier_key_record_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.verification.binding.credential_verifier_key_record_digest'),'')<>NEW.credential_verifier_key_record_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.verification.binding.verifier_report_id'),'')<>NEW.verifier_report_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.verification.signature_message_digest'),'')<>NEW.signature_message_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.verification.signature_base64'),'')<>NEW.signature_base64
          OR COALESCE(json_extract(NEW.receipt_json,'$.verification.signature_digest'),'')<>NEW.signature_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.verification.recorded_by_admin_user_id'),'')<>NEW.recorded_by_admin_user_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.verification.idempotency_scope'),'')<>NEW.idempotency_scope
          OR COALESCE(json_extract(NEW.receipt_json,'$.verification.idempotency_key'),'')<>NEW.idempotency_key
        BEGIN SELECT RAISE(ABORT,'credential verification JSON projection mismatch'); END;
        "#,
    )?;
    Ok(())
}
