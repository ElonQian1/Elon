use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS node_endpoint_credentials (
            credential_id TEXT PRIMARY KEY CHECK(
                credential_id=trim(credential_id)
                AND length(CAST(credential_id AS BLOB)) BETWEEN 1 AND 160
            ),
            agent_id TEXT NOT NULL UNIQUE CHECK(
                agent_id=trim(agent_id)
                AND length(CAST(agent_id AS BLOB)) BETWEEN 1 AND 160
            ),
            owner_user_id TEXT NOT NULL CHECK(
                owner_user_id=trim(owner_user_id)
                AND length(CAST(owner_user_id AS BLOB)) BETWEEN 1 AND 160
            ),
            install_id TEXT NOT NULL CHECK(
                install_id=trim(install_id)
                AND length(CAST(install_id AS BLOB)) BETWEEN 1 AND 512
            ),
            installation_binding_digest TEXT NOT NULL CHECK(
                length(installation_binding_digest)=64
                AND installation_binding_digest NOT GLOB '*[^0-9a-f]*'
            ),
            current_credential_revision INTEGER NOT NULL CHECK(
                current_credential_revision BETWEEN 1 AND 9007199254740991
            ),
            current_credential_digest TEXT NOT NULL CHECK(
                length(current_credential_digest)=64
                AND current_credential_digest NOT GLOB '*[^0-9a-f]*'
            ),
            status TEXT NOT NULL CHECK(status IN ('active','revoked')),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(owner_user_id, install_id),
            CHECK(created_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(created_at)=30 AND substr(created_at,20,1)='.'
                AND substr(created_at,30,1)='Z' AND julianday(created_at) IS NOT NULL),
            CHECK(updated_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(updated_at)=30 AND substr(updated_at,20,1)='.'
                AND substr(updated_at,30,1)='Z' AND julianday(updated_at) IS NOT NULL),
            CHECK(created_at<=updated_at),
            FOREIGN KEY(agent_id) REFERENCES node_credentials(agent_id) ON DELETE RESTRICT,
            FOREIGN KEY(owner_user_id) REFERENCES users(id) ON DELETE RESTRICT,
            FOREIGN KEY(
                credential_id, current_credential_revision, current_credential_digest
            ) REFERENCES node_endpoint_credential_versions(
                credential_id, credential_revision, credential_digest
            ) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
        ) WITHOUT ROWID;

        CREATE TABLE IF NOT EXISTS node_endpoint_credential_versions (
            credential_id TEXT NOT NULL CHECK(
                credential_id=trim(credential_id)
                AND length(CAST(credential_id AS BLOB)) BETWEEN 1 AND 160
            ),
            credential_revision INTEGER NOT NULL CHECK(
                credential_revision BETWEEN 1 AND 9007199254740991
            ),
            credential_schema TEXT NOT NULL CHECK(
                credential_schema='elon.node_endpoint.credential.v1'
            ),
            credential_digest TEXT NOT NULL CHECK(
                length(credential_digest)=64
                AND credential_digest NOT GLOB '*[^0-9a-f]*'
            ),
            credential_json TEXT NOT NULL CHECK(
                json_valid(credential_json)
                AND json_type(credential_json)='object'
                AND length(CAST(credential_json AS BLOB))<=524288
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            agent_id TEXT NOT NULL CHECK(
                agent_id=trim(agent_id)
                AND length(CAST(agent_id AS BLOB)) BETWEEN 1 AND 160
            ),
            owner_user_id TEXT NOT NULL CHECK(
                owner_user_id=trim(owner_user_id)
                AND length(CAST(owner_user_id AS BLOB)) BETWEEN 1 AND 160
            ),
            install_id TEXT NOT NULL CHECK(
                install_id=trim(install_id)
                AND length(CAST(install_id AS BLOB)) BETWEEN 1 AND 512
            ),
            installation_binding_digest TEXT NOT NULL CHECK(
                length(installation_binding_digest)=64
                AND installation_binding_digest NOT GLOB '*[^0-9a-f]*'
            ),
            secret_hash TEXT NOT NULL CHECK(
                length(secret_hash)=64 AND secret_hash NOT GLOB '*[^0-9a-f]*'
            ),
            secret_verifier_digest TEXT NOT NULL CHECK(
                length(secret_verifier_digest)=64
                AND secret_verifier_digest NOT GLOB '*[^0-9a-f]*'
            ),
            secret_hash_algorithm TEXT NOT NULL CHECK(secret_hash_algorithm='sha256'),
            issuance_kind TEXT NOT NULL CHECK(issuance_kind IN (
                'initial_registration','credential_rotation','account_recovery'
            )),
            issuance_request_id TEXT NOT NULL CHECK(
                issuance_request_id=trim(issuance_request_id)
                AND length(CAST(issuance_request_id AS BLOB)) BETWEEN 1 AND 160
            ),
            issued_by_user_id TEXT NOT NULL CHECK(
                issued_by_user_id=trim(issued_by_user_id)
                AND length(CAST(issued_by_user_id AS BLOB)) BETWEEN 1 AND 160
            ),
            owner_authorization_basis_kind TEXT NOT NULL CHECK(
                owner_authorization_basis_kind IN (
                    'future_owner_session','recent_reauthentication','security_operator'
                )
            ),
            owner_authorization_basis_id TEXT NOT NULL CHECK(
                owner_authorization_basis_id=trim(owner_authorization_basis_id)
                AND length(CAST(owner_authorization_basis_id AS BLOB)) BETWEEN 1 AND 160
            ),
            owner_authorization_basis_digest TEXT NOT NULL CHECK(
                length(owner_authorization_basis_digest)=64
                AND owner_authorization_basis_digest NOT GLOB '*[^0-9a-f]*'
            ),
            previous_credential_revision INTEGER,
            previous_credential_digest TEXT,
            issued_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            PRIMARY KEY(credential_id, credential_revision),
            UNIQUE(credential_digest),
            UNIQUE(secret_hash),
            UNIQUE(secret_verifier_digest),
            UNIQUE(credential_id, issuance_request_id),
            UNIQUE(credential_id, credential_revision, credential_digest),
            CHECK(
                (credential_revision=1
                    AND issuance_kind='initial_registration'
                    AND previous_credential_revision IS NULL
                    AND previous_credential_digest IS NULL)
                OR (credential_revision>1
                    AND issuance_kind IN ('credential_rotation','account_recovery')
                    AND previous_credential_revision=credential_revision-1
                    AND length(previous_credential_digest)=64
                    AND previous_credential_digest NOT GLOB '*[^0-9a-f]*')
            ),
            CHECK(issued_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(issued_at)=30 AND substr(issued_at,20,1)='.'
                AND substr(issued_at,30,1)='Z' AND julianday(issued_at) IS NOT NULL),
            CHECK(recorded_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(recorded_at)=30 AND substr(recorded_at,20,1)='.'
                AND substr(recorded_at,30,1)='Z' AND julianday(recorded_at) IS NOT NULL),
            CHECK(issued_at<=recorded_at),
            FOREIGN KEY(credential_id)
                REFERENCES node_endpoint_credentials(credential_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
            FOREIGN KEY(agent_id) REFERENCES node_credentials(agent_id) ON DELETE RESTRICT,
            FOREIGN KEY(owner_user_id) REFERENCES users(id) ON DELETE RESTRICT,
            FOREIGN KEY(issued_by_user_id) REFERENCES users(id) ON DELETE RESTRICT,
            FOREIGN KEY(
                credential_id, previous_credential_revision, previous_credential_digest
            ) REFERENCES node_endpoint_credential_versions(
                credential_id, credential_revision, credential_digest
            ) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
        ) WITHOUT ROWID;

        CREATE TABLE IF NOT EXISTS node_endpoint_credential_revocations (
            revocation_id TEXT PRIMARY KEY CHECK(
                revocation_id=trim(revocation_id)
                AND length(CAST(revocation_id AS BLOB)) BETWEEN 1 AND 160
            ),
            revocation_schema TEXT NOT NULL CHECK(
                revocation_schema='elon.node_endpoint.credential_revocation.v1'
            ),
            revocation_digest TEXT NOT NULL UNIQUE CHECK(
                length(revocation_digest)=64
                AND revocation_digest NOT GLOB '*[^0-9a-f]*'
            ),
            revocation_json TEXT NOT NULL CHECK(
                json_valid(revocation_json)
                AND json_type(revocation_json)='object'
                AND length(CAST(revocation_json AS BLOB))<=262144
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            credential_id TEXT NOT NULL CHECK(
                credential_id=trim(credential_id)
                AND length(CAST(credential_id AS BLOB)) BETWEEN 1 AND 160
            ),
            credential_revision INTEGER NOT NULL CHECK(
                credential_revision BETWEEN 1 AND 9007199254740991
            ),
            credential_digest TEXT NOT NULL CHECK(
                length(credential_digest)=64
                AND credential_digest NOT GLOB '*[^0-9a-f]*'
            ),
            agent_id TEXT NOT NULL CHECK(
                agent_id=trim(agent_id)
                AND length(CAST(agent_id AS BLOB)) BETWEEN 1 AND 160
            ),
            owner_user_id TEXT NOT NULL CHECK(
                owner_user_id=trim(owner_user_id)
                AND length(CAST(owner_user_id AS BLOB)) BETWEEN 1 AND 160
            ),
            revocation_kind TEXT NOT NULL CHECK(revocation_kind IN (
                'rotated','recovered','owner_revoked','security_revoked'
            )),
            reason_code TEXT NOT NULL CHECK(
                reason_code=trim(reason_code)
                AND length(CAST(reason_code AS BLOB)) BETWEEN 1 AND 160
            ),
            mutation_request_id TEXT NOT NULL CHECK(
                mutation_request_id=trim(mutation_request_id)
                AND length(CAST(mutation_request_id AS BLOB)) BETWEEN 1 AND 160
            ),
            revoked_by_user_id TEXT NOT NULL CHECK(
                revoked_by_user_id=trim(revoked_by_user_id)
                AND length(CAST(revoked_by_user_id AS BLOB)) BETWEEN 1 AND 160
            ),
            owner_authorization_basis_kind TEXT NOT NULL CHECK(
                owner_authorization_basis_kind IN (
                    'future_owner_session','recent_reauthentication','security_operator'
                )
            ),
            owner_authorization_basis_id TEXT NOT NULL CHECK(
                owner_authorization_basis_id=trim(owner_authorization_basis_id)
                AND length(CAST(owner_authorization_basis_id AS BLOB)) BETWEEN 1 AND 160
            ),
            owner_authorization_basis_digest TEXT NOT NULL CHECK(
                length(owner_authorization_basis_digest)=64
                AND owner_authorization_basis_digest NOT GLOB '*[^0-9a-f]*'
            ),
            revoked_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            UNIQUE(credential_id, credential_revision),
            UNIQUE(credential_id, mutation_request_id),
            CHECK(revoked_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(revoked_at)=30 AND substr(revoked_at,20,1)='.'
                AND substr(revoked_at,30,1)='Z' AND julianday(revoked_at) IS NOT NULL),
            CHECK(recorded_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(recorded_at)=30 AND substr(recorded_at,20,1)='.'
                AND substr(recorded_at,30,1)='Z' AND julianday(recorded_at) IS NOT NULL),
            CHECK(revoked_at<=recorded_at),
            FOREIGN KEY(credential_id, credential_revision, credential_digest)
                REFERENCES node_endpoint_credential_versions(
                    credential_id, credential_revision, credential_digest
                ) ON DELETE RESTRICT,
            FOREIGN KEY(agent_id) REFERENCES node_credentials(agent_id) ON DELETE RESTRICT,
            FOREIGN KEY(owner_user_id) REFERENCES users(id) ON DELETE RESTRICT,
            FOREIGN KEY(revoked_by_user_id) REFERENCES users(id) ON DELETE RESTRICT
        ) WITHOUT ROWID;

        CREATE INDEX IF NOT EXISTS idx_node_endpoint_credentials_owner_status
            ON node_endpoint_credentials(owner_user_id, status, agent_id);
        CREATE INDEX IF NOT EXISTS idx_node_endpoint_credential_versions_agent
            ON node_endpoint_credential_versions(agent_id, credential_revision DESC);
        CREATE INDEX IF NOT EXISTS idx_node_endpoint_credential_revocations_agent
            ON node_endpoint_credential_revocations(agent_id, recorded_at DESC);
        "#,
    )?;
    Ok(())
}
