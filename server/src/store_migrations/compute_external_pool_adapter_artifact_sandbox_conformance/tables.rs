use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_external_pool_adapter_sandbox_conformance_reports (
          sandbox_conformance_receipt_id TEXT PRIMARY KEY CHECK(length(trim(sandbox_conformance_receipt_id)) BETWEEN 1 AND 200),
          sandbox_conformance_receipt_digest TEXT NOT NULL UNIQUE CHECK(length(sandbox_conformance_receipt_digest)=64 AND sandbox_conformance_receipt_digest NOT GLOB '*[^0-9a-f]*'),
          receipt_json TEXT NOT NULL CHECK(json_valid(receipt_json) AND json_type(receipt_json)='object' AND length(CAST(receipt_json AS BLOB))<=1048576),
          conformance_material_digest TEXT NOT NULL CHECK(length(conformance_material_digest)=64 AND conformance_material_digest NOT GLOB '*[^0-9a-f]*'),
          admission_id TEXT NOT NULL UNIQUE,
          admission_digest TEXT NOT NULL CHECK(length(admission_digest)=64 AND admission_digest NOT GLOB '*[^0-9a-f]*'),
          adapter_id TEXT NOT NULL CHECK(length(trim(adapter_id)) BETWEEN 1 AND 200),
          release_version TEXT NOT NULL CHECK(length(trim(release_version)) BETWEEN 1 AND 200),
          vulnerability_report_receipt_id TEXT NOT NULL UNIQUE,
          vulnerability_report_receipt_digest TEXT NOT NULL UNIQUE CHECK(length(vulnerability_report_receipt_digest)=64 AND vulnerability_report_receipt_digest NOT GLOB '*[^0-9a-f]*'),
          sandbox_verifier_key_record_id TEXT NOT NULL,
          sandbox_verifier_key_record_digest TEXT NOT NULL CHECK(length(sandbox_verifier_key_record_digest)=64 AND sandbox_verifier_key_record_digest NOT GLOB '*[^0-9a-f]*'),
          sandbox_verifier_key_id TEXT NOT NULL,
          verifier_report_id TEXT NOT NULL UNIQUE CHECK(length(trim(verifier_report_id)) BETWEEN 1 AND 200),
          sandbox_runtime_id TEXT NOT NULL CHECK(length(trim(sandbox_runtime_id)) BETWEEN 1 AND 200),
          runtime_image_digest TEXT NOT NULL CHECK(length(runtime_image_digest)=64 AND runtime_image_digest NOT GLOB '*[^0-9a-f]*'),
          report_expires_at TEXT NOT NULL CHECK(report_expires_at GLOB '????-??-??T??:??:??.?????????Z' AND length(report_expires_at)=30 AND julianday(report_expires_at) IS NOT NULL),
          capability_set_digest TEXT NOT NULL CHECK(length(capability_set_digest)=64 AND capability_set_digest NOT GLOB '*[^0-9a-f]*'),
          test_plan_digest TEXT NOT NULL CHECK(length(test_plan_digest)=64 AND test_plan_digest NOT GLOB '*[^0-9a-f]*'),
          observation_inventory_digest TEXT NOT NULL CHECK(length(observation_inventory_digest)=64 AND observation_inventory_digest NOT GLOB '*[^0-9a-f]*'),
          capability_count INTEGER NOT NULL CHECK(capability_count=6),
          passed_capability_count INTEGER NOT NULL CHECK(passed_capability_count=6),
          policy_violation_count INTEGER NOT NULL CHECK(policy_violation_count=0),
          signature_message_digest TEXT NOT NULL CHECK(length(signature_message_digest)=64 AND signature_message_digest NOT GLOB '*[^0-9a-f]*'),
          signature_base64 TEXT NOT NULL CHECK(length(signature_base64) BETWEEN 1 AND 2048),
          signature_digest TEXT NOT NULL CHECK(length(signature_digest)=64 AND signature_digest NOT GLOB '*[^0-9a-f]*'),
          verified_by_admin_user_id TEXT NOT NULL CHECK(length(trim(verified_by_admin_user_id)) BETWEEN 1 AND 200),
          confirmation TEXT NOT NULL CHECK(confirmation='confirm_external_pool_adapter_sandbox_conformance'),
          idempotency_scope TEXT NOT NULL CHECK(length(trim(idempotency_scope)) BETWEEN 1 AND 240),
          idempotency_key TEXT NOT NULL CHECK(length(trim(idempotency_key)) BETWEEN 1 AND 240),
          verified_at TEXT NOT NULL CHECK(verified_at GLOB '????-??-??T??:??:??.?????????Z' AND length(verified_at)=30 AND julianday(verified_at) IS NOT NULL),
          recorded_at TEXT NOT NULL CHECK(recorded_at=verified_at),
          evidence_scope TEXT NOT NULL CHECK(evidence_scope='verifier_signature_over_exact_v236_artifact_server_derived_test_plan_and_asserted_observations'),
          conformance_effect TEXT NOT NULL CHECK(conformance_effect='signed_sandbox_report_verified_current'),
          credential_effect TEXT NOT NULL CHECK(credential_effect='none'),
          adapter_effect TEXT NOT NULL CHECK(adapter_effect='none'),
          route_effect TEXT NOT NULL CHECK(route_effect='none'),
          UNIQUE(idempotency_scope,idempotency_key),
          FOREIGN KEY(vulnerability_report_receipt_id) REFERENCES compute_external_pool_adapter_vulnerability_reports(vulnerability_report_receipt_id) ON DELETE RESTRICT,
          FOREIGN KEY(sandbox_verifier_key_record_id,sandbox_verifier_key_record_digest,sandbox_verifier_key_id)
            REFERENCES compute_external_pool_adapter_sandbox_verifier_keys(key_record_id,key_record_digest,key_id) ON DELETE RESTRICT
        );
        "#,
    )?;
    Ok(())
}
