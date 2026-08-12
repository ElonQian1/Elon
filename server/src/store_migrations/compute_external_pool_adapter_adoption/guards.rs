use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_adoption_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_adoption_receipts
        BEGIN SELECT RAISE(ABORT,'Adapter adoption receipts are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_adoption_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_adoption_receipts
        BEGIN SELECT RAISE(ABORT,'Adapter adoption receipts are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_adoption_no_replace
        BEFORE INSERT ON compute_external_pool_adapter_adoption_receipts
        WHEN EXISTS(SELECT 1 FROM compute_external_pool_adapter_adoption_receipts old
          WHERE old.adoption_receipt_id=NEW.adoption_receipt_id
             OR old.adoption_receipt_digest=NEW.adoption_receipt_digest
             OR (old.application_id=NEW.application_id AND old.admission_id=NEW.admission_id)
             OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT,'Adapter adoption cannot replace immutable history'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_adoption_terminal_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_adoption_terminal_receipts
        BEGIN SELECT RAISE(ABORT,'Adapter adoption terminal receipts are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_adoption_terminal_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_adoption_terminal_receipts
        BEGIN SELECT RAISE(ABORT,'Adapter adoption terminal receipts are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_adoption_terminal_no_replace
        BEFORE INSERT ON compute_external_pool_adapter_adoption_terminal_receipts
        WHEN EXISTS(SELECT 1 FROM compute_external_pool_adapter_adoption_terminal_receipts old
          WHERE old.terminal_receipt_id=NEW.terminal_receipt_id
             OR old.terminal_receipt_digest=NEW.terminal_receipt_digest
             OR old.adoption_receipt_id=NEW.adoption_receipt_id
             OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT,'Adapter adoption terminal cannot replace immutable history'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_adoption_requires_current_roots
        BEFORE INSERT ON compute_external_pool_adapter_adoption_receipts
        WHEN NOT EXISTS(SELECT 1 FROM compute_external_pool_adapter_sandbox_conformance_current current
          WHERE current.admission_id=NEW.admission_id
            AND current.sandbox_conformance_receipt_id=NEW.sandbox_conformance_receipt_id
            AND current.sandbox_conformance_receipt_digest=NEW.sandbox_conformance_receipt_digest
            AND current.current_status='verified_current')
          OR NOT EXISTS(SELECT 1 FROM compute_external_pool_adapter_credential_verification_current current
          WHERE current.credential_verification_receipt_id=NEW.credential_verification_receipt_id
            AND current.credential_verification_receipt_digest=NEW.credential_verification_receipt_digest
            AND current.current_status='verified_current')
        BEGIN SELECT RAISE(ABORT,'Adapter adoption requires exact current V239 and V243 roots'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_adoption_exact_lineage
        BEFORE INSERT ON compute_external_pool_adapter_adoption_receipts
        WHEN NOT EXISTS(SELECT 1 FROM compute_external_pool_adapter_sandbox_conformance_reports sandbox
          JOIN compute_external_pool_adapter_credential_verification_receipts credential
            ON credential.admission_id=sandbox.admission_id
           AND credential.admission_digest=sandbox.admission_digest
           AND credential.adapter_id=sandbox.adapter_id
           AND credential.adapter_release_version=sandbox.release_version
          WHERE sandbox.sandbox_conformance_receipt_id=NEW.sandbox_conformance_receipt_id
            AND sandbox.sandbox_conformance_receipt_digest=NEW.sandbox_conformance_receipt_digest
            AND credential.credential_verification_receipt_id=NEW.credential_verification_receipt_id
            AND credential.credential_verification_receipt_digest=NEW.credential_verification_receipt_digest
            AND credential.application_id=NEW.application_id
            AND credential.application_digest=NEW.application_digest
            AND credential.provider_id=NEW.provider_id
            AND credential.provider_policy_revision=NEW.provider_policy_revision
            AND credential.provider_digest=NEW.provider_digest
            AND credential.adapter_config_revision=NEW.adapter_config_revision
            AND credential.adapter_config_digest=NEW.adapter_config_digest
            AND credential.credential_locator_commitment=NEW.credential_locator_commitment
            AND sandbox.capability_set_digest=NEW.capability_set_digest
            AND COALESCE(json_extract(sandbox.receipt_json,'$.conformance.binding.declared_implementation_sha256'),'')=NEW.declared_implementation_sha256
            AND COALESCE(json_extract(credential.receipt_json,'$.verification.binding.declared_implementation_sha256'),'')=NEW.declared_implementation_sha256
            AND COALESCE(json_extract(credential.receipt_json,'$.verification.binding.capability_set_digest'),'')=NEW.capability_set_digest
            AND COALESCE(json_extract(credential.receipt_json,'$.verification.binding.provider_owner_account_id'),'')=NEW.provider_owner_account_id)
        BEGIN SELECT RAISE(ABORT,'Adapter adoption requires exact shared upstream lineage'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_adoption_fresh
        BEFORE INSERT ON compute_external_pool_adapter_adoption_receipts
        WHEN julianday(NEW.sandbox_report_expires_at)<=julianday(NEW.adopted_at)
          OR julianday(NEW.credential_report_expires_at)<=julianday(NEW.adopted_at)
        BEGIN SELECT RAISE(ABORT,'Adapter adoption upstream evidence expired before adoption'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_adoption_json_projection
        AFTER INSERT ON compute_external_pool_adapter_adoption_receipts
        WHEN COALESCE(json_extract(NEW.receipt_json,'$.schema'),'')<>'compute_federation.external_pool_adapter_adoption_receipt.v1'
          OR COALESCE(json_extract(NEW.receipt_json,'$.adoption_receipt_id'),'')<>NEW.adoption_receipt_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.adoption_receipt_digest'),'')<>NEW.adoption_receipt_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.adoption_material_digest'),'')<>NEW.adoption_material_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.adoption.binding.application_id'),'')<>NEW.application_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.adoption.binding.application_digest'),'')<>NEW.application_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.adoption.binding.provider_id'),'')<>NEW.provider_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.adoption.binding.provider_digest'),'')<>NEW.provider_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.adoption.binding.admission_id'),'')<>NEW.admission_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.adoption.binding.admission_digest'),'')<>NEW.admission_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.adoption.binding.sandbox_conformance_receipt_id'),'')<>NEW.sandbox_conformance_receipt_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.adoption.binding.sandbox_conformance_receipt_digest'),'')<>NEW.sandbox_conformance_receipt_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.adoption.binding.credential_verification_receipt_id'),'')<>NEW.credential_verification_receipt_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.adoption.binding.credential_verification_receipt_digest'),'')<>NEW.credential_verification_receipt_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.adoption.binding.credential_locator_commitment'),'')<>NEW.credential_locator_commitment
          OR COALESCE(json_extract(NEW.receipt_json,'$.adoption.adopted_by_admin_user_id'),'')<>NEW.adopted_by_admin_user_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.adoption.idempotency_scope'),'')<>NEW.idempotency_scope
          OR COALESCE(json_extract(NEW.receipt_json,'$.adoption.idempotency_key'),'')<>NEW.idempotency_key
        BEGIN SELECT RAISE(ABORT,'Adapter adoption JSON projection mismatch'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_adoption_terminal_exact_root
        BEFORE INSERT ON compute_external_pool_adapter_adoption_terminal_receipts
        WHEN NOT EXISTS(SELECT 1 FROM compute_external_pool_adapter_adoption_receipts adoption
          WHERE adoption.adoption_receipt_id=NEW.adoption_receipt_id
            AND adoption.adoption_receipt_digest=NEW.adoption_receipt_digest)
        BEGIN SELECT RAISE(ABORT,'Adapter adoption terminal requires exact adoption root'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_adoption_terminal_json_projection
        AFTER INSERT ON compute_external_pool_adapter_adoption_terminal_receipts
        WHEN COALESCE(json_extract(NEW.receipt_json,'$.schema'),'')<>'compute_federation.external_pool_adapter_adoption_terminal_receipt.v1'
          OR COALESCE(json_extract(NEW.receipt_json,'$.terminal_receipt_id'),'')<>NEW.terminal_receipt_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.terminal_receipt_digest'),'')<>NEW.terminal_receipt_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.terminal_material_digest'),'')<>NEW.terminal_material_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.terminal.adoption_receipt_id'),'')<>NEW.adoption_receipt_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.terminal.adoption_receipt_digest'),'')<>NEW.adoption_receipt_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.terminal.revoked_by_admin_user_id'),'')<>NEW.revoked_by_admin_user_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.terminal.reason'),'')<>NEW.reason
          OR COALESCE(json_extract(NEW.receipt_json,'$.terminal.idempotency_scope'),'')<>NEW.idempotency_scope
          OR COALESCE(json_extract(NEW.receipt_json,'$.terminal.idempotency_key'),'')<>NEW.idempotency_key
        BEGIN SELECT RAISE(ABORT,'Adapter adoption terminal JSON projection mismatch'); END;
        "#,
    )?;
    Ok(())
}
