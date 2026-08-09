use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_route_authorization_receipts (
            route_authorization_id TEXT PRIMARY KEY CHECK(
                length(trim(route_authorization_id)) BETWEEN 1 AND 160
            ),
            route_authorization_revision INTEGER NOT NULL CHECK(
                route_authorization_revision BETWEEN 1 AND 9007199254740991
            ),
            route_authorization_schema TEXT NOT NULL CHECK(
                route_authorization_schema='compute_federation.route_authorization.v1'
            ),
            route_authorization_digest TEXT NOT NULL UNIQUE CHECK(
                length(route_authorization_digest)=64
                AND route_authorization_digest NOT GLOB '*[^0-9a-f]*'
            ),
            route_authorization_json TEXT NOT NULL CHECK(
                json_valid(route_authorization_json)
                AND length(CAST(route_authorization_json AS BLOB))<=524288
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            provider_id TEXT NOT NULL,
            provider_kind TEXT NOT NULL CHECK(
                provider_kind IN ('user_node','managed_cluster','external_pool')
            ),
            provider_owner_account_id TEXT NOT NULL CHECK(
                length(trim(provider_owner_account_id)) BETWEEN 1 AND 160
            ),
            executor_id TEXT NOT NULL CHECK(length(trim(executor_id)) BETWEEN 1 AND 160),
            route_kind TEXT NOT NULL CHECK(
                route_kind IN ('provider_endpoint','server_adapter')
            ),
            route_binding_digest TEXT NOT NULL CHECK(length(route_binding_digest)=64),
            adapter_binding_digest TEXT NOT NULL CHECK(length(adapter_binding_digest)=64),
            endpoint_id TEXT,
            endpoint_transport TEXT,
            adapter_id TEXT NOT NULL,
            adapter_revision INTEGER NOT NULL CHECK(adapter_revision>0),
            adapter_registry_digest TEXT NOT NULL CHECK(length(adapter_registry_digest)=64),
            adapter_release_version TEXT NOT NULL CHECK(
                length(trim(adapter_release_version)) BETWEEN 1 AND 80
            ),
            implementation_digest TEXT NOT NULL CHECK(length(implementation_digest)=64),
            adapter_config_revision INTEGER NOT NULL CHECK(adapter_config_revision>0),
            adapter_config_digest TEXT NOT NULL CHECK(
                length(trim(adapter_config_digest)) BETWEEN 1 AND 512
                AND adapter_config_digest=trim(adapter_config_digest)
            ),
            credential_id TEXT NOT NULL,
            credential_revision INTEGER NOT NULL CHECK(credential_revision>0),
            credential_digest TEXT NOT NULL CHECK(length(credential_digest)=64),
            credential_expires_at TEXT NOT NULL,
            credential_cleanup_expires_at TEXT NOT NULL,
            capability_count INTEGER NOT NULL CHECK(capability_count=6),
            capability_set_digest TEXT NOT NULL CHECK(length(capability_set_digest)=64),
            source_kind TEXT NOT NULL CHECK(source_kind IN (
                'provider_activation_application','provider_recovery_application',
                'external_pool_onboarding'
            )),
            source_id TEXT NOT NULL CHECK(length(trim(source_id)) BETWEEN 1 AND 160),
            source_digest TEXT NOT NULL CHECK(length(source_digest)=64),
            approved_by_user_id TEXT NOT NULL CHECK(
                length(trim(approved_by_user_id)) BETWEEN 1 AND 160
            ),
            verification_kind TEXT NOT NULL CHECK(
                length(trim(verification_kind)) BETWEEN 1 AND 80
            ),
            verifier_id TEXT NOT NULL CHECK(length(trim(verifier_id)) BETWEEN 1 AND 160),
            verifier_revision INTEGER NOT NULL CHECK(verifier_revision>0),
            verifier_digest TEXT NOT NULL CHECK(length(verifier_digest)=64),
            verification_receipt_id TEXT NOT NULL CHECK(
                length(trim(verification_receipt_id)) BETWEEN 1 AND 160
            ),
            verification_receipt_digest TEXT NOT NULL CHECK(
                length(verification_receipt_digest)=64
            ),
            verified_by_service_actor_id TEXT NOT NULL CHECK(
                length(trim(verified_by_service_actor_id)) BETWEEN 1 AND 160
            ),
            actor_authorization_id TEXT NOT NULL CHECK(
                length(trim(actor_authorization_id)) BETWEEN 1 AND 160
            ),
            actor_authorization_digest TEXT NOT NULL CHECK(
                length(actor_authorization_digest)=64
            ),
            authenticated_at TEXT NOT NULL,
            authorized_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            cleanup_expires_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            CHECK(route_binding_digest NOT GLOB '*[^0-9a-f]*'
                AND route_binding_digest=adapter_binding_digest
                AND adapter_binding_digest NOT GLOB '*[^0-9a-f]*'
                AND adapter_registry_digest NOT GLOB '*[^0-9a-f]*'
                AND implementation_digest NOT GLOB '*[^0-9a-f]*'
                AND credential_digest NOT GLOB '*[^0-9a-f]*'
                AND capability_set_digest NOT GLOB '*[^0-9a-f]*'
                AND source_digest NOT GLOB '*[^0-9a-f]*'
                AND verifier_digest NOT GLOB '*[^0-9a-f]*'
                AND verification_receipt_digest NOT GLOB '*[^0-9a-f]*'
                AND actor_authorization_digest NOT GLOB '*[^0-9a-f]*'),
            CHECK(
                (route_kind='provider_endpoint'
                    AND endpoint_id IS NOT NULL AND endpoint_transport IS NOT NULL
                    AND length(trim(endpoint_id)) BETWEEN 1 AND 160
                    AND length(trim(endpoint_transport)) BETWEEN 1 AND 80)
                OR (route_kind='server_adapter'
                    AND endpoint_id IS NULL AND endpoint_transport IS NULL)
            ),
            CHECK(provider_kind!='external_pool' OR route_kind='server_adapter'),
            CHECK(authenticated_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(authenticated_at)=30 AND substr(authenticated_at,20,1)='.'
                AND substr(authenticated_at,30,1)='Z' AND julianday(authenticated_at) IS NOT NULL),
            CHECK(credential_expires_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(credential_expires_at)=30
                AND substr(credential_expires_at,20,1)='.'
                AND substr(credential_expires_at,30,1)='Z'
                AND julianday(credential_expires_at) IS NOT NULL),
            CHECK(credential_cleanup_expires_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(credential_cleanup_expires_at)=30
                AND substr(credential_cleanup_expires_at,20,1)='.'
                AND substr(credential_cleanup_expires_at,30,1)='Z'
                AND julianday(credential_cleanup_expires_at) IS NOT NULL),
            CHECK(authorized_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(authorized_at)=30 AND substr(authorized_at,20,1)='.'
                AND substr(authorized_at,30,1)='Z' AND julianday(authorized_at) IS NOT NULL),
            CHECK(expires_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(expires_at)=30 AND substr(expires_at,20,1)='.'
                AND substr(expires_at,30,1)='Z' AND julianday(expires_at) IS NOT NULL),
            CHECK(cleanup_expires_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(cleanup_expires_at)=30 AND substr(cleanup_expires_at,20,1)='.'
                AND substr(cleanup_expires_at,30,1)='Z'
                AND julianday(cleanup_expires_at) IS NOT NULL),
            CHECK(recorded_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(recorded_at)=30 AND substr(recorded_at,20,1)='.'
                AND substr(recorded_at,30,1)='Z' AND julianday(recorded_at) IS NOT NULL),
            CHECK(authenticated_at<=authorized_at AND authorized_at<=recorded_at
                AND recorded_at<expires_at AND expires_at<=cleanup_expires_at),
            CHECK(expires_at<=credential_expires_at
                AND cleanup_expires_at<=credential_cleanup_expires_at),
            FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(adapter_id, adapter_revision)
                REFERENCES compute_route_adapter_versions(adapter_id, adapter_revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(credential_id, credential_revision)
                REFERENCES compute_route_credential_versions(credential_id, credential_revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(actor_authorization_id)
                REFERENCES compute_service_actor_authorizations(actor_authorization_id)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_route_authorization_capabilities (
            route_authorization_id TEXT NOT NULL,
            ordinal INTEGER NOT NULL CHECK(ordinal BETWEEN 0 AND 5),
            capability_id TEXT NOT NULL CHECK(capability_id IN (
                'authenticated_ack','authenticated_events','cancel_no_start',
                'idempotent_commit','prepare','reconcile'
            )),
            capability_revision INTEGER NOT NULL CHECK(capability_revision>0),
            PRIMARY KEY(route_authorization_id, ordinal),
            UNIQUE(route_authorization_id, capability_id),
            CHECK(
                (ordinal=0 AND capability_id='authenticated_ack')
                OR (ordinal=1 AND capability_id='authenticated_events')
                OR (ordinal=2 AND capability_id='cancel_no_start')
                OR (ordinal=3 AND capability_id='idempotent_commit')
                OR (ordinal=4 AND capability_id='prepare')
                OR (ordinal=5 AND capability_id='reconcile')
            ),
            FOREIGN KEY(route_authorization_id)
                REFERENCES compute_route_authorization_receipts(route_authorization_id)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_route_authorization_seals (
            route_authorization_id TEXT PRIMARY KEY,
            route_authorization_revision INTEGER NOT NULL CHECK(
                route_authorization_revision BETWEEN 1 AND 9007199254740991
            ),
            seal_id TEXT NOT NULL UNIQUE CHECK(length(trim(seal_id)) BETWEEN 1 AND 160),
            seal_schema TEXT NOT NULL CHECK(
                seal_schema='compute_federation.route_authorization_seal.v1'
            ),
            seal_digest TEXT NOT NULL UNIQUE CHECK(length(seal_digest)=64),
            seal_json TEXT NOT NULL CHECK(
                json_valid(seal_json) AND length(CAST(seal_json AS BLOB))<=262144
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            route_authorization_digest TEXT NOT NULL CHECK(
                length(route_authorization_digest)=64
            ),
            adapter_id TEXT NOT NULL,
            adapter_revision INTEGER NOT NULL CHECK(adapter_revision>0),
            adapter_registry_digest TEXT NOT NULL CHECK(length(adapter_registry_digest)=64),
            credential_id TEXT NOT NULL,
            credential_revision INTEGER NOT NULL CHECK(credential_revision>0),
            credential_digest TEXT NOT NULL CHECK(length(credential_digest)=64),
            capability_count INTEGER NOT NULL CHECK(capability_count=6),
            capability_set_digest TEXT NOT NULL CHECK(length(capability_set_digest)=64),
            sealed_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            CHECK(seal_digest NOT GLOB '*[^0-9a-f]*'
                AND route_authorization_digest NOT GLOB '*[^0-9a-f]*'
                AND adapter_registry_digest NOT GLOB '*[^0-9a-f]*'
                AND credential_digest NOT GLOB '*[^0-9a-f]*'
                AND capability_set_digest NOT GLOB '*[^0-9a-f]*'),
            CHECK(sealed_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(sealed_at)=30 AND substr(sealed_at,20,1)='.'
                AND substr(sealed_at,30,1)='Z' AND julianday(sealed_at) IS NOT NULL),
            CHECK(recorded_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(recorded_at)=30 AND substr(recorded_at,20,1)='.'
                AND substr(recorded_at,30,1)='Z' AND julianday(recorded_at) IS NOT NULL),
            CHECK(sealed_at<=recorded_at),
            FOREIGN KEY(route_authorization_id)
                REFERENCES compute_route_authorization_receipts(route_authorization_id)
                ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_compute_route_authorizations_provider
            ON compute_route_authorization_receipts(provider_id, expires_at,
                route_authorization_id);
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_authorization_receipts_no_update
        BEFORE UPDATE ON compute_route_authorization_receipts
        BEGIN SELECT RAISE(ABORT, 'compute route authorization receipts are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_authorization_receipts_no_delete
        BEFORE DELETE ON compute_route_authorization_receipts
        BEGIN SELECT RAISE(ABORT, 'compute route authorization receipts are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_authorization_capabilities_no_update
        BEFORE UPDATE ON compute_route_authorization_capabilities
        BEGIN SELECT RAISE(ABORT, 'compute route authorization capabilities are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_authorization_capabilities_no_delete
        BEFORE DELETE ON compute_route_authorization_capabilities
        BEGIN SELECT RAISE(ABORT, 'compute route authorization capabilities are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_authorization_seals_no_update
        BEFORE UPDATE ON compute_route_authorization_seals
        BEGIN SELECT RAISE(ABORT, 'compute route authorization seals are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_authorization_seals_no_delete
        BEFORE DELETE ON compute_route_authorization_seals
        BEGIN SELECT RAISE(ABORT, 'compute route authorization seals are append-only'); END;
        "#,
    )?;
    Ok(())
}
