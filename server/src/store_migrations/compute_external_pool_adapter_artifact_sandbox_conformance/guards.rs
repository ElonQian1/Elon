use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS sandbox_conformance_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_sandbox_conformance_reports
        BEGIN SELECT RAISE(ABORT,'sandbox conformance reports are immutable'); END;

        CREATE TRIGGER IF NOT EXISTS sandbox_conformance_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_sandbox_conformance_reports
        BEGIN SELECT RAISE(ABORT,'sandbox conformance reports are append-only'); END;

        CREATE TRIGGER IF NOT EXISTS sandbox_conformance_exact_vulnerability_report
        BEFORE INSERT ON compute_external_pool_adapter_sandbox_conformance_reports
        WHEN NOT EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_vulnerability_report_current current
          WHERE current.current_status='verified_current'
            AND current.admission_id=NEW.admission_id
            AND current.vulnerability_report_receipt_id=NEW.vulnerability_report_receipt_id
            AND current.vulnerability_report_receipt_digest=NEW.vulnerability_report_receipt_digest)
        BEGIN SELECT RAISE(ABORT,'sandbox conformance lacks exact current V236 authority'); END;

        CREATE TRIGGER IF NOT EXISTS sandbox_conformance_exact_verifier_key
        BEFORE INSERT ON compute_external_pool_adapter_sandbox_conformance_reports
        WHEN NOT EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_sandbox_verifier_key_current current
          WHERE current.current_status='active'
            AND current.key_record_id=NEW.sandbox_verifier_key_record_id
            AND current.key_record_digest=NEW.sandbox_verifier_key_record_digest
            AND current.key_id=NEW.sandbox_verifier_key_id)
        BEGIN SELECT RAISE(ABORT,'sandbox conformance lacks exact active V237 verifier authority'); END;

        CREATE TRIGGER IF NOT EXISTS sandbox_conformance_fresh_report
        BEFORE INSERT ON compute_external_pool_adapter_sandbox_conformance_reports
        WHEN julianday(NEW.report_expires_at)<=julianday('now')
          OR julianday(json_extract(NEW.receipt_json,'$.conformance.binding.run_started_at'))>julianday('now','+5 minutes')
          OR julianday(json_extract(NEW.receipt_json,'$.conformance.binding.run_started_at'))<julianday(json_extract(NEW.receipt_json,'$.conformance.binding.vulnerability_report_verified_at'))
          OR julianday(json_extract(NEW.receipt_json,'$.conformance.binding.run_completed_at'))<julianday(json_extract(NEW.receipt_json,'$.conformance.binding.run_started_at'))
          OR julianday(json_extract(NEW.receipt_json,'$.conformance.binding.run_completed_at'))>julianday(json_extract(NEW.receipt_json,'$.conformance.binding.run_started_at'),'+30 minutes')
          OR julianday(json_extract(NEW.receipt_json,'$.conformance.binding.report_generated_at'))<julianday(json_extract(NEW.receipt_json,'$.conformance.binding.run_completed_at'))
          OR julianday(NEW.report_expires_at)>julianday(json_extract(NEW.receipt_json,'$.conformance.binding.report_generated_at'),'+24 hours')
          OR julianday(NEW.report_expires_at)>julianday(json_extract(NEW.receipt_json,'$.conformance.binding.vulnerability_intelligence_expires_at'))
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.external_network_attempt_count'),-1)<>0
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.write_outside_ephemeral_count'),-1)<>0
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.child_process_attempt_count'),-1)<>0
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.peak_memory_bytes'),0) NOT BETWEEN 1 AND 536870912
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.cpu_time_ms'),0) NOT BETWEEN 1 AND 900000
          OR julianday(NEW.verified_at)>julianday('now','+5 minutes')
          OR julianday(NEW.verified_at)<julianday(json_extract(NEW.receipt_json,'$.conformance.binding.report_generated_at'))
          OR julianday(NEW.verified_at)>julianday(NEW.report_expires_at)
        BEGIN SELECT RAISE(ABORT,'sandbox conformance report is outside its runtime validity bound'); END;

        CREATE TRIGGER IF NOT EXISTS sandbox_conformance_json_projection
        AFTER INSERT ON compute_external_pool_adapter_sandbox_conformance_reports
        WHEN COALESCE(json_extract(NEW.receipt_json,'$.schema'),'')<>'compute_federation.external_pool_adapter_sandbox_conformance_receipt.v1'
          OR COALESCE(json_extract(NEW.receipt_json,'$.canonicalization'),'')<>'rfc8785_jcs'
          OR COALESCE(json_extract(NEW.receipt_json,'$.digest_algorithm'),'')<>'sha256'
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.schema'),'')<>'compute_federation.external_pool_adapter_sandbox_conformance_binding.v1'
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.signature_algorithm'),'')<>'rsa-pkcs1v15-sha256'
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.sandbox_policy_id'),'')<>'external_pool_adapter_six_capability_offline_sandbox_v1'
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.isolation_profile_id'),'')<>'offline_readonly_ephemeral_no_child_process_v1'
          OR COALESCE(json_extract(NEW.receipt_json,'$.sandbox_conformance_receipt_id'),'')<>NEW.sandbox_conformance_receipt_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.sandbox_conformance_receipt_digest'),'')<>NEW.sandbox_conformance_receipt_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance_material_digest'),'')<>NEW.conformance_material_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.admission_id'),'')<>NEW.admission_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.admission_digest'),'')<>NEW.admission_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.adapter_id'),'')<>NEW.adapter_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.release_version'),'')<>NEW.release_version
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.vulnerability_report_receipt_id'),'')<>NEW.vulnerability_report_receipt_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.vulnerability_report_receipt_digest'),'')<>NEW.vulnerability_report_receipt_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.sandbox_verifier_key_record_id'),'')<>NEW.sandbox_verifier_key_record_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.sandbox_verifier_key_record_digest'),'')<>NEW.sandbox_verifier_key_record_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.sandbox_verifier_key_id'),'')<>NEW.sandbox_verifier_key_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.verifier_report_id'),'')<>NEW.verifier_report_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.sandbox_runtime_id'),'')<>NEW.sandbox_runtime_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.runtime_image_digest'),'')<>NEW.runtime_image_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.report_expires_at'),'')<>NEW.report_expires_at
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.capability_set_digest'),'')<>NEW.capability_set_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.test_plan_digest'),'')<>NEW.test_plan_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.observation_inventory_digest'),'')<>NEW.observation_inventory_digest
          OR COALESCE(json_array_length(json_extract(NEW.receipt_json,'$.conformance.binding.supported_capabilities')),-1)<>NEW.capability_count
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.passed_capability_count'),-1)<>NEW.passed_capability_count
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.binding.policy_violation_count'),-1)<>NEW.policy_violation_count
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.signature_message_digest'),'')<>NEW.signature_message_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.signature_base64'),'')<>NEW.signature_base64
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.signature_digest'),'')<>NEW.signature_digest
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.verified_by_admin_user_id'),'')<>NEW.verified_by_admin_user_id
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.confirmation'),'')<>NEW.confirmation
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.idempotency_scope'),'')<>NEW.idempotency_scope
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.idempotency_key'),'')<>NEW.idempotency_key
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.verified_at'),'')<>NEW.verified_at
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.recorded_at'),'')<>NEW.recorded_at
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.evidence_scope'),'')<>NEW.evidence_scope
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.conformance_effect'),'')<>NEW.conformance_effect
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.credential_effect'),'')<>NEW.credential_effect
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.adapter_effect'),'')<>NEW.adapter_effect
          OR COALESCE(json_extract(NEW.receipt_json,'$.conformance.route_effect'),'')<>NEW.route_effect
        BEGIN SELECT RAISE(ABORT,'sandbox conformance JSON projection mismatch'); END;
        "#,
    )?;
    Ok(())
}
