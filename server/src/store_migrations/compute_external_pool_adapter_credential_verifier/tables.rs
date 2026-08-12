use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_external_pool_adapter_credential_verifiers (
          verifier_record_id TEXT PRIMARY KEY NOT NULL,
          verifier_record_schema TEXT NOT NULL CHECK(verifier_record_schema='compute_federation.external_pool_adapter_credential_verifier.v1'),
          verifier_record_digest TEXT NOT NULL UNIQUE CHECK(length(verifier_record_digest)=64 AND verifier_record_digest NOT GLOB '*[^0-9a-f]*'),
          verifier_record_json TEXT NOT NULL CHECK(json_valid(verifier_record_json) AND json_type(verifier_record_json)='object' AND length(CAST(verifier_record_json AS BLOB))<=131072),
          registration_material_digest TEXT NOT NULL CHECK(length(registration_material_digest)=64 AND registration_material_digest NOT GLOB '*[^0-9a-f]*'),
          canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
          digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
          verifier_operator TEXT NOT NULL CHECK(length(trim(verifier_operator)) BETWEEN 1 AND 160),
          verifier_product TEXT NOT NULL CHECK(length(trim(verifier_product)) BETWEEN 1 AND 160),
          verification_kind TEXT NOT NULL CHECK(length(trim(verification_kind)) BETWEEN 1 AND 80),
          verifier_id TEXT NOT NULL CHECK(length(trim(verifier_id)) BETWEEN 1 AND 160),
          verifier_revision INTEGER NOT NULL CHECK(verifier_revision BETWEEN 1 AND 9007199254740991),
          verifier_digest TEXT NOT NULL CHECK(length(verifier_digest)=64 AND verifier_digest NOT GLOB '*[^0-9a-f]*'),
          actor_kind TEXT NOT NULL CHECK(actor_kind='platform_admin'),
          created_by_admin_user_id TEXT NOT NULL CHECK(length(trim(created_by_admin_user_id)) BETWEEN 1 AND 160),
          confirmation TEXT NOT NULL CHECK(confirmation='confirm_external_pool_adapter_credential_verifier_registration'),
          idempotency_scope TEXT NOT NULL CHECK(length(trim(idempotency_scope)) BETWEEN 1 AND 200),
          idempotency_key TEXT NOT NULL CHECK(length(trim(idempotency_key)) BETWEEN 1 AND 160),
          created_at TEXT NOT NULL CHECK(created_at GLOB '????-??-??T??:??:??.?????????Z' AND length(created_at)=30 AND julianday(created_at) IS NOT NULL),
          recorded_at TEXT NOT NULL CHECK(recorded_at=created_at),
          currentness_effect TEXT NOT NULL CHECK(currentness_effect='pending_activation'),
          credential_receipt_effect TEXT NOT NULL CHECK(credential_receipt_effect='none'),
          adapter_adoption_effect TEXT NOT NULL CHECK(adapter_adoption_effect='none'),
          route_effect TEXT NOT NULL CHECK(route_effect='none'),
          execution_effect TEXT NOT NULL CHECK(execution_effect='none'),
          UNIQUE(idempotency_scope,idempotency_key),
          UNIQUE(verification_kind,verifier_id,verifier_revision),
          UNIQUE(verifier_record_id,verifier_record_digest,verification_kind,verifier_id,verifier_revision,verifier_digest)
        );

        CREATE TABLE IF NOT EXISTS compute_external_pool_adapter_credential_verifier_transitions (
          transition_receipt_id TEXT PRIMARY KEY NOT NULL,
          transition_receipt_schema TEXT NOT NULL CHECK(transition_receipt_schema IN (
            'compute_federation.external_pool_adapter_credential_verifier_activation_receipt.v1',
            'compute_federation.external_pool_adapter_credential_verifier_revocation_receipt.v1')),
          transition_receipt_digest TEXT NOT NULL UNIQUE CHECK(length(transition_receipt_digest)=64 AND transition_receipt_digest NOT GLOB '*[^0-9a-f]*'),
          transition_receipt_json TEXT NOT NULL CHECK(json_valid(transition_receipt_json) AND json_type(transition_receipt_json)='object' AND length(CAST(transition_receipt_json AS BLOB))<=131072),
          transition_material_digest TEXT NOT NULL CHECK(length(transition_material_digest)=64 AND transition_material_digest NOT GLOB '*[^0-9a-f]*'),
          transition_kind TEXT NOT NULL CHECK(transition_kind IN ('activation','revocation')),
          canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
          digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
          verifier_record_id TEXT NOT NULL,
          verifier_record_digest TEXT NOT NULL,
          verification_kind TEXT NOT NULL,
          verifier_id TEXT NOT NULL,
          verifier_revision INTEGER NOT NULL,
          verifier_digest TEXT NOT NULL,
          verifier_operator TEXT NOT NULL,
          verifier_product TEXT NOT NULL,
          actor_kind TEXT NOT NULL CHECK(actor_kind='platform_admin'),
          actor_user_id TEXT NOT NULL CHECK(length(trim(actor_user_id)) BETWEEN 1 AND 160),
          reason TEXT CHECK(reason IS NULL OR (length(reason) BETWEEN 8 AND 2000 AND reason=trim(reason))),
          confirmation TEXT NOT NULL,
          idempotency_scope TEXT NOT NULL,
          idempotency_key TEXT NOT NULL,
          occurred_at TEXT NOT NULL CHECK(occurred_at GLOB '????-??-??T??:??:??.?????????Z' AND length(occurred_at)=30 AND julianday(occurred_at) IS NOT NULL),
          recorded_at TEXT NOT NULL CHECK(recorded_at=occurred_at),
          currentness_effect TEXT NOT NULL CHECK(currentness_effect IN ('active','revoked')),
          credential_receipt_effect TEXT NOT NULL CHECK(credential_receipt_effect='none'),
          adapter_adoption_effect TEXT NOT NULL CHECK(adapter_adoption_effect='none'),
          route_effect TEXT NOT NULL CHECK(route_effect='none'),
          execution_effect TEXT NOT NULL CHECK(execution_effect='none'),
          UNIQUE(verifier_record_id,transition_kind),
          UNIQUE(idempotency_scope,idempotency_key),
          FOREIGN KEY(verifier_record_id,verifier_record_digest,verification_kind,verifier_id,verifier_revision,verifier_digest)
            REFERENCES compute_external_pool_adapter_credential_verifiers(
              verifier_record_id,verifier_record_digest,verification_kind,verifier_id,verifier_revision,verifier_digest)
            ON DELETE RESTRICT,
          CHECK((transition_kind='activation' AND transition_receipt_schema='compute_federation.external_pool_adapter_credential_verifier_activation_receipt.v1' AND reason IS NULL AND confirmation='confirm_external_pool_adapter_credential_verifier_activation' AND currentness_effect='active')
             OR (transition_kind='revocation' AND transition_receipt_schema='compute_federation.external_pool_adapter_credential_verifier_revocation_receipt.v1' AND reason IS NOT NULL AND confirmation='confirm_external_pool_adapter_credential_verifier_revocation' AND currentness_effect='revoked'))
        );
        "#,
    )?;
    Ok(())
}
