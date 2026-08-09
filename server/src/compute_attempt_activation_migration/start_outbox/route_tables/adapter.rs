use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_route_adapters (
            adapter_id TEXT PRIMARY KEY CHECK(length(trim(adapter_id)) BETWEEN 1 AND 160),
            current_adapter_revision INTEGER NOT NULL CHECK(
                current_adapter_revision BETWEEN 1 AND 9007199254740991
            ),
            current_adapter_digest TEXT NOT NULL CHECK(
                length(current_adapter_digest)=64
                AND current_adapter_digest NOT GLOB '*[^0-9a-f]*'
            ),
            status TEXT NOT NULL CHECK(status IN ('active','draining','revoked')),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            CHECK(created_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(created_at)=30 AND substr(created_at,20,1)='.'
                AND substr(created_at,30,1)='Z' AND julianday(created_at) IS NOT NULL),
            CHECK(updated_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(updated_at)=30 AND substr(updated_at,20,1)='.'
                AND substr(updated_at,30,1)='Z' AND julianday(updated_at) IS NOT NULL),
            CHECK(created_at<=updated_at),
            FOREIGN KEY(adapter_id, current_adapter_revision)
                REFERENCES compute_route_adapter_versions(adapter_id, adapter_revision)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
        );

        CREATE TABLE IF NOT EXISTS compute_route_adapter_versions (
            adapter_id TEXT NOT NULL,
            adapter_revision INTEGER NOT NULL CHECK(
                adapter_revision BETWEEN 1 AND 9007199254740991
            ),
            adapter_schema TEXT NOT NULL CHECK(
                adapter_schema='compute_federation.route_adapter.v1'
            ),
            adapter_digest TEXT NOT NULL UNIQUE CHECK(
                length(adapter_digest)=64 AND adapter_digest NOT GLOB '*[^0-9a-f]*'
            ),
            adapter_json TEXT NOT NULL CHECK(
                json_valid(adapter_json)
                AND length(CAST(adapter_json AS BLOB))<=524288
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            release_version TEXT NOT NULL CHECK(length(trim(release_version)) BETWEEN 1 AND 80),
            implementation_digest TEXT NOT NULL CHECK(length(implementation_digest)=64),
            route_kind TEXT NOT NULL CHECK(route_kind IN ('provider_endpoint','server_adapter')),
            supported_provider_kinds_json TEXT NOT NULL CHECK(
                json_valid(supported_provider_kinds_json)
                AND json_type(supported_provider_kinds_json)='array'
                AND json_array_length(supported_provider_kinds_json)>0
                AND length(CAST(supported_provider_kinds_json AS BLOB))<=65536
            ),
            credential_verification_kind TEXT NOT NULL CHECK(
                length(trim(credential_verification_kind)) BETWEEN 1 AND 80
            ),
            credential_verifier_id TEXT NOT NULL CHECK(
                length(trim(credential_verifier_id)) BETWEEN 1 AND 160
            ),
            credential_verifier_revision INTEGER NOT NULL CHECK(
                credential_verifier_revision BETWEEN 1 AND 9007199254740991
            ),
            credential_verifier_digest TEXT NOT NULL CHECK(
                length(credential_verifier_digest)=64
            ),
            supported_capabilities_json TEXT NOT NULL CHECK(
                json_valid(supported_capabilities_json)
                AND json_type(supported_capabilities_json)='array'
                AND json_array_length(supported_capabilities_json)>=6
                AND length(CAST(supported_capabilities_json AS BLOB))<=65536
            ),
            status TEXT NOT NULL CHECK(status IN ('active','draining','revoked')),
            registered_by_service_actor_id TEXT NOT NULL CHECK(
                length(trim(registered_by_service_actor_id)) BETWEEN 1 AND 160
            ),
            actor_authorization_id TEXT NOT NULL CHECK(
                length(trim(actor_authorization_id)) BETWEEN 1 AND 160
            ),
            actor_authorization_digest TEXT NOT NULL CHECK(length(actor_authorization_digest)=64),
            registered_at TEXT NOT NULL,
            PRIMARY KEY(adapter_id, adapter_revision),
            CHECK(implementation_digest NOT GLOB '*[^0-9a-f]*'
                AND credential_verifier_digest NOT GLOB '*[^0-9a-f]*'
                AND actor_authorization_digest NOT GLOB '*[^0-9a-f]*'),
            CHECK(registered_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(registered_at)=30 AND substr(registered_at,20,1)='.'
                AND substr(registered_at,30,1)='Z' AND julianday(registered_at) IS NOT NULL),
            FOREIGN KEY(adapter_id) REFERENCES compute_route_adapters(adapter_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
            FOREIGN KEY(actor_authorization_id)
                REFERENCES compute_service_actor_authorizations(actor_authorization_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
        );

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_adapters_current_transition
        BEFORE UPDATE ON compute_route_adapters
        WHEN OLD.adapter_id IS NOT NEW.adapter_id
          OR NEW.current_adapter_revision<OLD.current_adapter_revision
          OR NEW.updated_at<=OLD.updated_at
          OR NOT (
                (OLD.status=NEW.status)
                OR (OLD.status='active' AND NEW.status IN ('draining','revoked'))
                OR (OLD.status='draining' AND NEW.status='revoked')
          )
        BEGIN
            SELECT RAISE(ABORT, 'compute route adapter transition is invalid');
        END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_adapters_current_exact_insert
        AFTER INSERT ON compute_route_adapters
        WHEN NOT EXISTS (
            SELECT 1 FROM compute_route_adapter_versions version
             WHERE version.adapter_id=NEW.adapter_id
               AND version.adapter_revision=NEW.current_adapter_revision
               AND version.adapter_digest=NEW.current_adapter_digest
               AND version.status=NEW.status
        )
        BEGIN SELECT RAISE(ABORT, 'compute route adapter current version mismatch'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_adapters_current_exact_update
        AFTER UPDATE ON compute_route_adapters
        WHEN NOT EXISTS (
            SELECT 1 FROM compute_route_adapter_versions version
             WHERE version.adapter_id=NEW.adapter_id
               AND version.adapter_revision=NEW.current_adapter_revision
               AND version.adapter_digest=NEW.current_adapter_digest
               AND version.status=NEW.status
        )
        BEGIN SELECT RAISE(ABORT, 'compute route adapter current version mismatch'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_adapters_no_delete
        BEFORE DELETE ON compute_route_adapters
        BEGIN SELECT RAISE(ABORT, 'compute route adapters cannot be deleted'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_adapter_versions_no_update
        BEFORE UPDATE ON compute_route_adapter_versions
        BEGIN SELECT RAISE(ABORT, 'compute route adapter versions are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_adapter_versions_no_delete
        BEFORE DELETE ON compute_route_adapter_versions
        BEGIN SELECT RAISE(ABORT, 'compute route adapter versions are append-only'); END;
        "#,
    )?;
    Ok(())
}
