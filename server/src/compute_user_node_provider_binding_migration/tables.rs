use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_user_node_provider_bindings (
            binding_id TEXT PRIMARY KEY CHECK(
                length(binding_id)=64 AND binding_id NOT GLOB '*[^0-9a-f]*'
            ),
            binding_schema TEXT NOT NULL CHECK(
                binding_schema='compute_federation.user_node_provider_binding.v1'
            ),
            binding_digest TEXT NOT NULL UNIQUE CHECK(
                length(binding_digest)=64 AND binding_digest NOT GLOB '*[^0-9a-f]*'
            ),
            binding_json TEXT NOT NULL CHECK(
                json_valid(binding_json)
                AND json_type(binding_json)='object'
                AND length(CAST(binding_json AS BLOB))<=131072
            ),
            binding_material_digest TEXT NOT NULL UNIQUE CHECK(
                length(binding_material_digest)=64
                AND binding_material_digest NOT GLOB '*[^0-9a-f]*'
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            provider_id TEXT NOT NULL CHECK(
                provider_id=trim(provider_id)
                AND length(provider_id) BETWEEN 1 AND 160
            ),
            provider_genesis_policy_revision INTEGER NOT NULL CHECK(
                provider_genesis_policy_revision=1
            ),
            provider_genesis_digest TEXT NOT NULL CHECK(
                length(provider_genesis_digest)=64
                AND provider_genesis_digest NOT GLOB '*[^0-9a-f]*'
            ),
            node_id TEXT NOT NULL CHECK(
                node_id=trim(node_id)
                AND length(node_id) BETWEEN 1 AND 160
            ),
            owner_user_id TEXT NOT NULL CHECK(
                owner_user_id=trim(owner_user_id)
                AND length(owner_user_id) BETWEEN 1 AND 160
            ),
            installation_identity_digest TEXT NOT NULL CHECK(
                length(installation_identity_digest)=64
                AND installation_identity_digest NOT GLOB '*[^0-9a-f]*'
            ),
            endpoint_installation_binding_digest TEXT NOT NULL CHECK(
                length(endpoint_installation_binding_digest)=64
                AND endpoint_installation_binding_digest NOT GLOB '*[^0-9a-f]*'
            ),
            source_endpoint_credential_id TEXT NOT NULL CHECK(
                source_endpoint_credential_id=trim(source_endpoint_credential_id)
                AND length(source_endpoint_credential_id) BETWEEN 1 AND 160
            ),
            source_endpoint_credential_revision INTEGER NOT NULL CHECK(
                source_endpoint_credential_revision BETWEEN 1 AND 9007199254740991
            ),
            source_endpoint_credential_digest TEXT NOT NULL CHECK(
                length(source_endpoint_credential_digest)=64
                AND source_endpoint_credential_digest NOT GLOB '*[^0-9a-f]*'
            ),
            source_consent_receipt_id TEXT NOT NULL CHECK(
                source_consent_receipt_id=trim(source_consent_receipt_id)
                AND length(source_consent_receipt_id) BETWEEN 1 AND 160
            ),
            source_consent_policy_revision INTEGER NOT NULL CHECK(
                source_consent_policy_revision BETWEEN 1 AND 9007199254740991
            ),
            source_consent_policy_digest TEXT NOT NULL CHECK(
                length(source_consent_policy_digest)=64
                AND source_consent_policy_digest NOT GLOB '*[^0-9a-f]*'
            ),
            source_authorization_ref TEXT NOT NULL CHECK(
                source_authorization_ref=trim(source_authorization_ref)
                AND length(source_authorization_ref) BETWEEN 1 AND 160
            ),
            source_authorization_revision INTEGER NOT NULL CHECK(
                source_authorization_revision=source_consent_policy_revision
            ),
            source_authorization_digest TEXT NOT NULL CHECK(
                length(source_authorization_digest)=64
                AND source_authorization_digest NOT GLOB '*[^0-9a-f]*'
            ),
            confirmation TEXT NOT NULL CHECK(
                confirmation='confirm_user_node_provider_binding'
            ),
            idempotency_scope TEXT NOT NULL CHECK(
                idempotency_scope=trim(idempotency_scope)
                AND length(idempotency_scope) BETWEEN 1 AND 200
            ),
            idempotency_key TEXT NOT NULL CHECK(
                idempotency_key=trim(idempotency_key)
                AND length(idempotency_key) BETWEEN 1 AND 160
            ),
            request_digest TEXT NOT NULL CHECK(
                length(request_digest)=64 AND request_digest NOT GLOB '*[^0-9a-f]*'
            ),
            bound_at TEXT NOT NULL CHECK(
                bound_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(bound_at)=30
                AND substr(bound_at,20,1)='.'
                AND substr(bound_at,30,1)='Z'
                AND julianday(bound_at) IS NOT NULL
            ),
            recorded_at TEXT NOT NULL CHECK(
                recorded_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(recorded_at)=30
                AND substr(recorded_at,20,1)='.'
                AND substr(recorded_at,30,1)='Z'
                AND julianday(recorded_at) IS NOT NULL
                AND recorded_at=bound_at
            ),
            binding_effect TEXT NOT NULL CHECK(binding_effect='identity_binding_recorded'),
            provider_effect TEXT NOT NULL CHECK(provider_effect='none'),
            capacity_effect TEXT NOT NULL CHECK(capacity_effect='none'),
            offer_effect TEXT NOT NULL CHECK(offer_effect='none'),
            readiness_effect TEXT NOT NULL CHECK(readiness_effect='none'),
            route_effect TEXT NOT NULL CHECK(route_effect='none'),
            execution_effect TEXT NOT NULL CHECK(execution_effect='none'),
            settlement_effect TEXT NOT NULL CHECK(settlement_effect='none'),
            UNIQUE(provider_id),
            UNIQUE(node_id),
            UNIQUE(idempotency_scope,idempotency_key),
            FOREIGN KEY(provider_id,provider_genesis_policy_revision)
                REFERENCES compute_provider_versions(provider_id,policy_revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(node_id) REFERENCES node_credentials(agent_id) ON DELETE RESTRICT,
            FOREIGN KEY(owner_user_id) REFERENCES users(id) ON DELETE RESTRICT,
            FOREIGN KEY(
                source_endpoint_credential_id,
                source_endpoint_credential_revision,
                source_endpoint_credential_digest
            ) REFERENCES node_endpoint_credential_versions(
                credential_id,credential_revision,credential_digest
            ) ON DELETE RESTRICT,
            FOREIGN KEY(source_consent_receipt_id)
                REFERENCES node_compute_plugin_sharing_consents(receipt_id)
                ON DELETE RESTRICT
        ) WITHOUT ROWID;

        CREATE INDEX IF NOT EXISTS idx_compute_user_node_provider_bindings_owner
            ON compute_user_node_provider_bindings(owner_user_id,provider_id);
        "#,
    )?;
    Ok(())
}
