use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_route_credentials (
            credential_id TEXT PRIMARY KEY CHECK(
                length(trim(credential_id)) BETWEEN 1 AND 160
            ),
            current_credential_revision INTEGER NOT NULL CHECK(
                current_credential_revision BETWEEN 1 AND 9007199254740991
            ),
            current_credential_digest TEXT NOT NULL CHECK(
                length(current_credential_digest)=64
                AND current_credential_digest NOT GLOB '*[^0-9a-f]*'
            ),
            status TEXT NOT NULL CHECK(status IN ('active','retired','revoked')),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            CHECK(created_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(created_at)=30 AND substr(created_at,20,1)='.'
                AND substr(created_at,30,1)='Z' AND julianday(created_at) IS NOT NULL),
            CHECK(updated_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(updated_at)=30 AND substr(updated_at,20,1)='.'
                AND substr(updated_at,30,1)='Z' AND julianday(updated_at) IS NOT NULL),
            CHECK(created_at<=updated_at),
            FOREIGN KEY(credential_id, current_credential_revision)
                REFERENCES compute_route_credential_versions(credential_id, credential_revision)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
        );

        CREATE TABLE IF NOT EXISTS compute_route_credential_versions (
            credential_id TEXT NOT NULL,
            credential_revision INTEGER NOT NULL CHECK(
                credential_revision BETWEEN 1 AND 9007199254740991
            ),
            credential_schema TEXT NOT NULL CHECK(
                credential_schema='compute_federation.route_credential.v1'
            ),
            credential_digest TEXT NOT NULL UNIQUE CHECK(
                length(credential_digest)=64
                AND credential_digest NOT GLOB '*[^0-9a-f]*'
            ),
            credential_json TEXT NOT NULL CHECK(
                json_valid(credential_json)
                AND length(CAST(credential_json AS BLOB))<=524288
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
            route_kind TEXT NOT NULL CHECK(
                route_kind IN ('provider_endpoint','server_adapter')
            ),
            route_binding_digest TEXT NOT NULL CHECK(length(route_binding_digest)=64),
            adapter_binding_digest TEXT NOT NULL CHECK(length(adapter_binding_digest)=64),
            endpoint_id TEXT,
            endpoint_transport TEXT,
            adapter_id TEXT NOT NULL,
            adapter_revision INTEGER NOT NULL CHECK(
                adapter_revision BETWEEN 1 AND 9007199254740991
            ),
            adapter_registry_digest TEXT NOT NULL CHECK(length(adapter_registry_digest)=64),
            adapter_release_version TEXT NOT NULL CHECK(
                length(trim(adapter_release_version)) BETWEEN 1 AND 80
            ),
            implementation_digest TEXT NOT NULL CHECK(length(implementation_digest)=64),
            adapter_config_revision INTEGER NOT NULL CHECK(
                adapter_config_revision BETWEEN 1 AND 9007199254740991
            ),
            adapter_config_digest TEXT NOT NULL CHECK(
                length(trim(adapter_config_digest)) BETWEEN 1 AND 512
                AND adapter_config_digest=trim(adapter_config_digest)
            ),
            non_bearer_credential_ref TEXT NOT NULL CHECK(
                length(trim(non_bearer_credential_ref)) BETWEEN 1 AND 512
            ),
            credential_hint TEXT NOT NULL CHECK(
                length(trim(credential_hint)) BETWEEN 1 AND 160
            ),
            verification_kind TEXT NOT NULL CHECK(
                length(trim(verification_kind)) BETWEEN 1 AND 80
            ),
            verifier_id TEXT NOT NULL CHECK(length(trim(verifier_id)) BETWEEN 1 AND 160),
            verifier_revision INTEGER NOT NULL CHECK(
                verifier_revision BETWEEN 1 AND 9007199254740991
            ),
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
            expires_at TEXT NOT NULL,
            cleanup_expires_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            PRIMARY KEY(credential_id, credential_revision),
            CHECK(route_binding_digest NOT GLOB '*[^0-9a-f]*'
                AND route_binding_digest=adapter_binding_digest
                AND adapter_binding_digest NOT GLOB '*[^0-9a-f]*'
                AND adapter_registry_digest NOT GLOB '*[^0-9a-f]*'
                AND implementation_digest NOT GLOB '*[^0-9a-f]*'
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
            CHECK(authenticated_at<=recorded_at AND recorded_at<expires_at
                AND expires_at<=cleanup_expires_at),
            FOREIGN KEY(credential_id) REFERENCES compute_route_credentials(credential_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
            FOREIGN KEY(adapter_id, adapter_revision)
                REFERENCES compute_route_adapter_versions(adapter_id, adapter_revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(actor_authorization_id)
                REFERENCES compute_service_actor_authorizations(actor_authorization_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
        );

        CREATE TABLE IF NOT EXISTS compute_route_credential_revocations (
            revocation_id TEXT PRIMARY KEY CHECK(
                length(trim(revocation_id)) BETWEEN 1 AND 160
            ),
            revocation_schema TEXT NOT NULL CHECK(
                revocation_schema='compute_federation.route_credential_revocation.v1'
            ),
            revocation_digest TEXT NOT NULL UNIQUE CHECK(length(revocation_digest)=64),
            revocation_json TEXT NOT NULL CHECK(
                json_valid(revocation_json)
                AND length(CAST(revocation_json AS BLOB))<=262144
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            credential_id TEXT NOT NULL,
            credential_revision INTEGER NOT NULL CHECK(credential_revision>0),
            credential_digest TEXT NOT NULL CHECK(length(credential_digest)=64),
            provider_id TEXT NOT NULL,
            reason_code TEXT NOT NULL CHECK(length(trim(reason_code)) BETWEEN 1 AND 160),
            revoked_by_service_actor_id TEXT NOT NULL CHECK(
                length(trim(revoked_by_service_actor_id)) BETWEEN 1 AND 160
            ),
            actor_authorization_id TEXT NOT NULL CHECK(
                length(trim(actor_authorization_id)) BETWEEN 1 AND 160
            ),
            actor_authorization_digest TEXT NOT NULL CHECK(length(actor_authorization_digest)=64),
            revoked_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            UNIQUE(credential_id, credential_revision),
            CHECK(revocation_digest NOT GLOB '*[^0-9a-f]*'
                AND credential_digest NOT GLOB '*[^0-9a-f]*'
                AND actor_authorization_digest NOT GLOB '*[^0-9a-f]*'),
            CHECK(revoked_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(revoked_at)=30 AND substr(revoked_at,20,1)='.'
                AND substr(revoked_at,30,1)='Z' AND julianday(revoked_at) IS NOT NULL),
            CHECK(recorded_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(recorded_at)=30 AND substr(recorded_at,20,1)='.'
                AND substr(recorded_at,30,1)='Z' AND julianday(recorded_at) IS NOT NULL),
            CHECK(revoked_at<=recorded_at),
            FOREIGN KEY(credential_id, credential_revision)
                REFERENCES compute_route_credential_versions(credential_id, credential_revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(actor_authorization_id)
                REFERENCES compute_service_actor_authorizations(actor_authorization_id)
                ON DELETE RESTRICT
        );

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_credentials_current_transition
        BEFORE UPDATE ON compute_route_credentials
        WHEN OLD.credential_id IS NOT NEW.credential_id
          OR NEW.current_credential_revision<OLD.current_credential_revision
          OR NEW.updated_at<=OLD.updated_at
          OR NOT (
                OLD.status=NEW.status
                OR (OLD.status='active' AND NEW.status IN ('retired','revoked'))
                OR (OLD.status='retired' AND NEW.status='revoked')
          )
        BEGIN
            SELECT RAISE(ABORT, 'compute route credential transition is invalid');
        END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_credentials_current_exact_insert
        AFTER INSERT ON compute_route_credentials
        WHEN NOT EXISTS (
            SELECT 1 FROM compute_route_credential_versions version
             WHERE version.credential_id=NEW.credential_id
               AND version.credential_revision=NEW.current_credential_revision
               AND version.credential_digest=NEW.current_credential_digest
        ) OR NEW.status!='active'
        BEGIN SELECT RAISE(ABORT, 'compute route credential current version mismatch'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_credentials_current_exact_update
        AFTER UPDATE ON compute_route_credentials
        WHEN NOT EXISTS (
            SELECT 1 FROM compute_route_credential_versions version
             WHERE version.credential_id=NEW.credential_id
               AND version.credential_revision=NEW.current_credential_revision
               AND version.credential_digest=NEW.current_credential_digest
        ) OR (NEW.status='revoked' AND NOT EXISTS (
            SELECT 1 FROM compute_route_credential_revocations revoked
             WHERE revoked.credential_id=NEW.credential_id
               AND revoked.credential_revision=NEW.current_credential_revision
        ))
        BEGIN SELECT RAISE(ABORT, 'compute route credential current version mismatch'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_credentials_no_delete
        BEFORE DELETE ON compute_route_credentials
        BEGIN SELECT RAISE(ABORT, 'compute route credentials cannot be deleted'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_credential_versions_no_update
        BEFORE UPDATE ON compute_route_credential_versions
        BEGIN SELECT RAISE(ABORT, 'compute route credential versions are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_credential_versions_no_delete
        BEFORE DELETE ON compute_route_credential_versions
        BEGIN SELECT RAISE(ABORT, 'compute route credential versions are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_credential_revocations_no_update
        BEFORE UPDATE ON compute_route_credential_revocations
        BEGIN SELECT RAISE(ABORT, 'compute route credential revocations are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_credential_revocations_no_delete
        BEFORE DELETE ON compute_route_credential_revocations
        BEGIN SELECT RAISE(ABORT, 'compute route credential revocations are append-only'); END;
        "#,
    )?;
    Ok(())
}
