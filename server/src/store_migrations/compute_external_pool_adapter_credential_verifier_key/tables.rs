use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_external_pool_adapter_credential_verifier_keys (
          key_record_id TEXT PRIMARY KEY NOT NULL,
          key_record_schema TEXT NOT NULL CHECK(key_record_schema='compute_federation.external_pool_adapter_credential_verifier_key.v1'),
          key_record_digest TEXT NOT NULL UNIQUE CHECK(length(key_record_digest)=64 AND key_record_digest NOT GLOB '*[^0-9a-f]*'),
          key_record_json TEXT NOT NULL CHECK(json_valid(key_record_json) AND json_type(key_record_json)='object' AND length(CAST(key_record_json AS BLOB))<=131072),
          registration_material_digest TEXT NOT NULL CHECK(length(registration_material_digest)=64 AND registration_material_digest NOT GLOB '*[^0-9a-f]*'),
          canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
          digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
          verifier_record_id TEXT NOT NULL,
          verifier_record_digest TEXT NOT NULL,
          verifier_operator TEXT NOT NULL CHECK(length(trim(verifier_operator)) BETWEEN 1 AND 160),
          verifier_product TEXT NOT NULL CHECK(length(trim(verifier_product)) BETWEEN 1 AND 160),
          verification_kind TEXT NOT NULL CHECK(length(trim(verification_kind)) BETWEEN 1 AND 80),
          verifier_id TEXT NOT NULL CHECK(length(trim(verifier_id)) BETWEEN 1 AND 160),
          verifier_revision INTEGER NOT NULL CHECK(verifier_revision BETWEEN 1 AND 9007199254740991),
          verifier_digest TEXT NOT NULL,
          key_id TEXT NOT NULL UNIQUE CHECK(length(key_id)=64 AND key_id NOT GLOB '*[^0-9a-f]*'),
          algorithm TEXT NOT NULL CHECK(algorithm='rsa-pkcs1v15-sha256'),
          public_key_pem TEXT NOT NULL CHECK(length(public_key_pem) BETWEEN 1 AND 16384),
          actor_kind TEXT NOT NULL CHECK(actor_kind='platform_admin'),
          created_by_admin_user_id TEXT NOT NULL CHECK(length(trim(created_by_admin_user_id)) BETWEEN 1 AND 160),
          confirmation TEXT NOT NULL CHECK(confirmation='confirm_external_pool_adapter_credential_verifier_key_registration'),
          idempotency_scope TEXT NOT NULL CHECK(length(trim(idempotency_scope)) BETWEEN 1 AND 200),
          idempotency_key TEXT NOT NULL CHECK(length(trim(idempotency_key)) BETWEEN 1 AND 160),
          created_at TEXT NOT NULL CHECK(created_at GLOB '????-??-??T??:??:??.?????????Z' AND length(created_at)=30 AND julianday(created_at) IS NOT NULL),
          recorded_at TEXT NOT NULL CHECK(recorded_at=created_at),
          currentness_effect TEXT NOT NULL CHECK(currentness_effect='active'),
          credential_receipt_effect TEXT NOT NULL CHECK(credential_receipt_effect='none'),
          adapter_effect TEXT NOT NULL CHECK(adapter_effect='none'),
          route_effect TEXT NOT NULL CHECK(route_effect='none'),
          UNIQUE(idempotency_scope,idempotency_key),
          UNIQUE(key_record_id,key_record_digest,verifier_record_id,verifier_record_digest,key_id),
          FOREIGN KEY(verifier_record_id,verifier_record_digest,verification_kind,verifier_id,verifier_revision,verifier_digest)
            REFERENCES compute_external_pool_adapter_credential_verifiers(
              verifier_record_id,verifier_record_digest,verification_kind,verifier_id,verifier_revision,verifier_digest)
            ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_external_pool_adapter_credential_verifier_key_revocations (
          revocation_receipt_id TEXT PRIMARY KEY NOT NULL,
          revocation_receipt_schema TEXT NOT NULL CHECK(revocation_receipt_schema='compute_federation.external_pool_adapter_credential_verifier_key_revocation_receipt.v1'),
          revocation_receipt_digest TEXT NOT NULL UNIQUE CHECK(length(revocation_receipt_digest)=64 AND revocation_receipt_digest NOT GLOB '*[^0-9a-f]*'),
          revocation_receipt_json TEXT NOT NULL CHECK(json_valid(revocation_receipt_json) AND json_type(revocation_receipt_json)='object' AND length(CAST(revocation_receipt_json AS BLOB))<=131072),
          revocation_material_digest TEXT NOT NULL CHECK(length(revocation_material_digest)=64 AND revocation_material_digest NOT GLOB '*[^0-9a-f]*'),
          canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
          digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
          key_record_id TEXT NOT NULL,
          key_record_digest TEXT NOT NULL,
          verifier_record_id TEXT NOT NULL,
          verifier_record_digest TEXT NOT NULL,
          key_id TEXT NOT NULL,
          actor_kind TEXT NOT NULL CHECK(actor_kind='platform_admin'),
          revoked_by_admin_user_id TEXT NOT NULL CHECK(length(trim(revoked_by_admin_user_id)) BETWEEN 1 AND 160),
          reason TEXT NOT NULL CHECK(length(reason) BETWEEN 8 AND 2000 AND reason=trim(reason)),
          confirmation TEXT NOT NULL CHECK(confirmation='confirm_external_pool_adapter_credential_verifier_key_revocation'),
          idempotency_scope TEXT NOT NULL CHECK(length(trim(idempotency_scope)) BETWEEN 1 AND 200),
          idempotency_key TEXT NOT NULL CHECK(length(trim(idempotency_key)) BETWEEN 1 AND 160),
          revoked_at TEXT NOT NULL CHECK(revoked_at GLOB '????-??-??T??:??:??.?????????Z' AND length(revoked_at)=30 AND julianday(revoked_at) IS NOT NULL),
          recorded_at TEXT NOT NULL CHECK(recorded_at=revoked_at),
          currentness_effect TEXT NOT NULL CHECK(currentness_effect='revoked'),
          credential_receipt_effect TEXT NOT NULL CHECK(credential_receipt_effect='none'),
          adapter_effect TEXT NOT NULL CHECK(adapter_effect='none'),
          route_effect TEXT NOT NULL CHECK(route_effect='none'),
          UNIQUE(key_record_id),
          UNIQUE(idempotency_scope,idempotency_key),
          FOREIGN KEY(key_record_id,key_record_digest,verifier_record_id,verifier_record_digest,key_id)
            REFERENCES compute_external_pool_adapter_credential_verifier_keys(
              key_record_id,key_record_digest,verifier_record_id,verifier_record_digest,key_id)
            ON DELETE RESTRICT
        );
        "#,
    )?;
    Ok(())
}
