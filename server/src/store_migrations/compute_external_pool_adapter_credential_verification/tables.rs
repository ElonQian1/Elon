use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_external_pool_adapter_credential_verification_receipts (
          credential_verification_receipt_id TEXT PRIMARY KEY NOT NULL CHECK(length(trim(credential_verification_receipt_id)) BETWEEN 1 AND 200),
          credential_verification_receipt_digest TEXT NOT NULL UNIQUE CHECK(length(credential_verification_receipt_digest)=64 AND credential_verification_receipt_digest NOT GLOB '*[^0-9a-f]*'),
          receipt_json TEXT NOT NULL CHECK(json_valid(receipt_json) AND json_type(receipt_json)='object' AND length(CAST(receipt_json AS BLOB))<=524288),
          verification_material_digest TEXT NOT NULL CHECK(length(verification_material_digest)=64 AND verification_material_digest NOT GLOB '*[^0-9a-f]*'),
          application_id TEXT NOT NULL,
          application_digest TEXT NOT NULL CHECK(length(application_digest)=64 AND application_digest NOT GLOB '*[^0-9a-f]*'),
          provider_id TEXT NOT NULL,
          provider_policy_revision INTEGER NOT NULL CHECK(provider_policy_revision>0),
          provider_digest TEXT NOT NULL CHECK(length(provider_digest)=64 AND provider_digest NOT GLOB '*[^0-9a-f]*'),
          adapter_id TEXT NOT NULL CHECK(length(trim(adapter_id)) BETWEEN 1 AND 200),
          adapter_release_version TEXT NOT NULL CHECK(length(trim(adapter_release_version)) BETWEEN 1 AND 200),
          adapter_config_revision INTEGER NOT NULL CHECK(adapter_config_revision>0),
          adapter_config_digest TEXT NOT NULL CHECK(length(trim(adapter_config_digest)) BETWEEN 1 AND 512),
          credential_ref_scheme TEXT NOT NULL CHECK(credential_ref_scheme IN ('vault_ref','gateway_ref')),
          credential_locator_commitment TEXT NOT NULL CHECK(length(credential_locator_commitment)=64 AND credential_locator_commitment NOT GLOB '*[^0-9a-f]*'),
          admission_id TEXT NOT NULL,
          admission_digest TEXT NOT NULL CHECK(length(admission_digest)=64 AND admission_digest NOT GLOB '*[^0-9a-f]*'),
          credential_verifier_key_record_id TEXT NOT NULL,
          credential_verifier_key_record_digest TEXT NOT NULL CHECK(length(credential_verifier_key_record_digest)=64 AND credential_verifier_key_record_digest NOT GLOB '*[^0-9a-f]*'),
          credential_verifier_key_id TEXT NOT NULL,
          credential_verifier_record_id TEXT NOT NULL,
          credential_verifier_record_digest TEXT NOT NULL CHECK(length(credential_verifier_record_digest)=64 AND credential_verifier_record_digest NOT GLOB '*[^0-9a-f]*'),
          verifier_report_id TEXT NOT NULL UNIQUE CHECK(length(trim(verifier_report_id)) BETWEEN 1 AND 200),
          report_expires_at TEXT NOT NULL CHECK(report_expires_at GLOB '????-??-??T??:??:??.?????????Z' AND length(report_expires_at)=30 AND julianday(report_expires_at) IS NOT NULL),
          provider_response_evidence_digest TEXT NOT NULL CHECK(length(provider_response_evidence_digest)=64 AND provider_response_evidence_digest NOT GLOB '*[^0-9a-f]*'),
          signature_message_digest TEXT NOT NULL CHECK(length(signature_message_digest)=64 AND signature_message_digest NOT GLOB '*[^0-9a-f]*'),
          signature_base64 TEXT NOT NULL CHECK(length(signature_base64) BETWEEN 1 AND 2048),
          signature_digest TEXT NOT NULL CHECK(length(signature_digest)=64 AND signature_digest NOT GLOB '*[^0-9a-f]*'),
          recorded_by_admin_user_id TEXT NOT NULL CHECK(length(trim(recorded_by_admin_user_id)) BETWEEN 1 AND 200),
          confirmation TEXT NOT NULL CHECK(confirmation='confirm_external_pool_adapter_credential_verification'),
          idempotency_scope TEXT NOT NULL CHECK(length(trim(idempotency_scope)) BETWEEN 1 AND 240),
          idempotency_key TEXT NOT NULL CHECK(length(trim(idempotency_key)) BETWEEN 1 AND 240),
          verified_at TEXT NOT NULL CHECK(verified_at GLOB '????-??-??T??:??:??.?????????Z' AND length(verified_at)=30 AND julianday(verified_at) IS NOT NULL),
          recorded_at TEXT NOT NULL CHECK(recorded_at=verified_at),
          evidence_scope TEXT NOT NULL CHECK(evidence_scope='verifier_signature_over_exact_v221_non_bearer_locator_commitment_v222_admission_and_asserted_authentication'),
          credential_effect TEXT NOT NULL CHECK(credential_effect='signed_credential_verification_current'),
          adapter_effect TEXT NOT NULL CHECK(adapter_effect='none'),
          route_effect TEXT NOT NULL CHECK(route_effect='none'),
          execution_effect TEXT NOT NULL CHECK(execution_effect='none'),
          settlement_effect TEXT NOT NULL CHECK(settlement_effect='none'),
          UNIQUE(idempotency_scope,idempotency_key),
          FOREIGN KEY(application_id) REFERENCES compute_external_pool_onboarding_applications(application_id) ON DELETE RESTRICT,
          FOREIGN KEY(admission_id) REFERENCES compute_external_pool_adapter_release_admissions(admission_id) ON DELETE RESTRICT,
          FOREIGN KEY(credential_verifier_key_record_id,credential_verifier_key_record_digest,credential_verifier_record_id,credential_verifier_record_digest,credential_verifier_key_id)
            REFERENCES compute_external_pool_adapter_credential_verifier_keys(key_record_id,key_record_digest,verifier_record_id,verifier_record_digest,key_id)
            ON DELETE RESTRICT
        );
        CREATE INDEX IF NOT EXISTS idx_external_pool_adapter_credential_verification_application
          ON compute_external_pool_adapter_credential_verification_receipts(
            application_id,verified_at DESC,credential_verification_receipt_id);
        CREATE INDEX IF NOT EXISTS idx_external_pool_adapter_credential_verification_admission
          ON compute_external_pool_adapter_credential_verification_receipts(
            admission_id,verified_at DESC,credential_verification_receipt_id);
        "#,
    )?;
    Ok(())
}
