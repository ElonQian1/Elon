use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_external_pool_adapter_adoption_receipts (
          adoption_receipt_id TEXT PRIMARY KEY NOT NULL CHECK(length(trim(adoption_receipt_id)) BETWEEN 1 AND 200),
          adoption_receipt_digest TEXT NOT NULL UNIQUE CHECK(length(adoption_receipt_digest)=64 AND adoption_receipt_digest NOT GLOB '*[^0-9a-f]*'),
          receipt_json TEXT NOT NULL CHECK(json_valid(receipt_json) AND json_type(receipt_json)='object' AND length(CAST(receipt_json AS BLOB))<=524288),
          adoption_material_digest TEXT NOT NULL CHECK(length(adoption_material_digest)=64 AND adoption_material_digest NOT GLOB '*[^0-9a-f]*'),
          application_id TEXT NOT NULL,
          application_digest TEXT NOT NULL CHECK(length(application_digest)=64 AND application_digest NOT GLOB '*[^0-9a-f]*'),
          provider_id TEXT NOT NULL,
          provider_owner_account_id TEXT NOT NULL,
          provider_policy_revision INTEGER NOT NULL CHECK(provider_policy_revision>0),
          provider_digest TEXT NOT NULL CHECK(length(provider_digest)=64 AND provider_digest NOT GLOB '*[^0-9a-f]*'),
          admission_id TEXT NOT NULL,
          admission_digest TEXT NOT NULL CHECK(length(admission_digest)=64 AND admission_digest NOT GLOB '*[^0-9a-f]*'),
          adapter_id TEXT NOT NULL CHECK(length(trim(adapter_id)) BETWEEN 1 AND 200),
          adapter_release_version TEXT NOT NULL CHECK(length(trim(adapter_release_version)) BETWEEN 1 AND 200),
          adapter_config_revision INTEGER NOT NULL CHECK(adapter_config_revision>0),
          adapter_config_digest TEXT NOT NULL CHECK(length(trim(adapter_config_digest)) BETWEEN 1 AND 512),
          declared_implementation_sha256 TEXT NOT NULL CHECK(length(declared_implementation_sha256)=64 AND declared_implementation_sha256 NOT GLOB '*[^0-9a-f]*'),
          capability_set_digest TEXT NOT NULL CHECK(length(capability_set_digest)=64 AND capability_set_digest NOT GLOB '*[^0-9a-f]*'),
          sandbox_conformance_receipt_id TEXT NOT NULL,
          sandbox_conformance_receipt_digest TEXT NOT NULL CHECK(length(sandbox_conformance_receipt_digest)=64 AND sandbox_conformance_receipt_digest NOT GLOB '*[^0-9a-f]*'),
          sandbox_report_expires_at TEXT NOT NULL CHECK(sandbox_report_expires_at GLOB '????-??-??T??:??:??.?????????Z' AND length(sandbox_report_expires_at)=30 AND julianday(sandbox_report_expires_at) IS NOT NULL),
          credential_verification_receipt_id TEXT NOT NULL,
          credential_verification_receipt_digest TEXT NOT NULL CHECK(length(credential_verification_receipt_digest)=64 AND credential_verification_receipt_digest NOT GLOB '*[^0-9a-f]*'),
          credential_locator_commitment TEXT NOT NULL CHECK(length(credential_locator_commitment)=64 AND credential_locator_commitment NOT GLOB '*[^0-9a-f]*'),
          credential_report_expires_at TEXT NOT NULL CHECK(credential_report_expires_at GLOB '????-??-??T??:??:??.?????????Z' AND length(credential_report_expires_at)=30 AND julianday(credential_report_expires_at) IS NOT NULL),
          adopted_by_admin_user_id TEXT NOT NULL CHECK(length(trim(adopted_by_admin_user_id)) BETWEEN 1 AND 200),
          confirmation TEXT NOT NULL CHECK(confirmation='confirm_external_pool_adapter_adoption'),
          idempotency_scope TEXT NOT NULL CHECK(length(trim(idempotency_scope)) BETWEEN 1 AND 240),
          idempotency_key TEXT NOT NULL CHECK(length(trim(idempotency_key)) BETWEEN 1 AND 240),
          adopted_at TEXT NOT NULL CHECK(adopted_at GLOB '????-??-??T??:??:??.?????????Z' AND length(adopted_at)=30 AND julianday(adopted_at) IS NOT NULL),
          recorded_at TEXT NOT NULL CHECK(recorded_at=adopted_at),
          adoption_effect TEXT NOT NULL CHECK(adoption_effect='adoption_authority_current'),
          install_effect TEXT NOT NULL CHECK(install_effect='authorization_only'),
          provider_effect TEXT NOT NULL CHECK(provider_effect='none'),
          route_effect TEXT NOT NULL CHECK(route_effect='none'),
          execution_effect TEXT NOT NULL CHECK(execution_effect='none'),
          settlement_effect TEXT NOT NULL CHECK(settlement_effect='none'),
          UNIQUE(application_id,admission_id),
          UNIQUE(idempotency_scope,idempotency_key),
          FOREIGN KEY(application_id) REFERENCES compute_external_pool_onboarding_applications(application_id) ON DELETE RESTRICT,
          FOREIGN KEY(admission_id) REFERENCES compute_external_pool_adapter_release_admissions(admission_id) ON DELETE RESTRICT,
          FOREIGN KEY(sandbox_conformance_receipt_id) REFERENCES compute_external_pool_adapter_sandbox_conformance_reports(sandbox_conformance_receipt_id) ON DELETE RESTRICT,
          FOREIGN KEY(credential_verification_receipt_id) REFERENCES compute_external_pool_adapter_credential_verification_receipts(credential_verification_receipt_id) ON DELETE RESTRICT
        );
        CREATE INDEX IF NOT EXISTS idx_external_pool_adapter_adoption_provider
          ON compute_external_pool_adapter_adoption_receipts(provider_id,adopted_at DESC);

        CREATE TABLE IF NOT EXISTS compute_external_pool_adapter_adoption_terminal_receipts (
          terminal_receipt_id TEXT PRIMARY KEY NOT NULL CHECK(length(trim(terminal_receipt_id)) BETWEEN 1 AND 200),
          terminal_receipt_digest TEXT NOT NULL UNIQUE CHECK(length(terminal_receipt_digest)=64 AND terminal_receipt_digest NOT GLOB '*[^0-9a-f]*'),
          receipt_json TEXT NOT NULL CHECK(json_valid(receipt_json) AND json_type(receipt_json)='object' AND length(CAST(receipt_json AS BLOB))<=262144),
          terminal_material_digest TEXT NOT NULL CHECK(length(terminal_material_digest)=64 AND terminal_material_digest NOT GLOB '*[^0-9a-f]*'),
          adoption_receipt_id TEXT NOT NULL UNIQUE,
          adoption_receipt_digest TEXT NOT NULL CHECK(length(adoption_receipt_digest)=64 AND adoption_receipt_digest NOT GLOB '*[^0-9a-f]*'),
          revoked_by_admin_user_id TEXT NOT NULL CHECK(length(trim(revoked_by_admin_user_id)) BETWEEN 1 AND 200),
          reason TEXT NOT NULL CHECK(length(trim(reason)) BETWEEN 1 AND 1000),
          confirmation TEXT NOT NULL CHECK(confirmation='confirm_external_pool_adapter_adoption_revocation'),
          idempotency_scope TEXT NOT NULL CHECK(length(trim(idempotency_scope)) BETWEEN 1 AND 240),
          idempotency_key TEXT NOT NULL CHECK(length(trim(idempotency_key)) BETWEEN 1 AND 240),
          revoked_at TEXT NOT NULL CHECK(revoked_at GLOB '????-??-??T??:??:??.?????????Z' AND length(revoked_at)=30 AND julianday(revoked_at) IS NOT NULL),
          recorded_at TEXT NOT NULL CHECK(recorded_at=revoked_at),
          adoption_effect TEXT NOT NULL CHECK(adoption_effect='adoption_authority_revoked'),
          provider_effect TEXT NOT NULL CHECK(provider_effect='none'),
          route_effect TEXT NOT NULL CHECK(route_effect='none'),
          execution_effect TEXT NOT NULL CHECK(execution_effect='none'),
          settlement_effect TEXT NOT NULL CHECK(settlement_effect='none'),
          UNIQUE(idempotency_scope,idempotency_key),
          FOREIGN KEY(adoption_receipt_id) REFERENCES compute_external_pool_adapter_adoption_receipts(adoption_receipt_id) ON DELETE RESTRICT
        );
        "#,
    )?;
    Ok(())
}
