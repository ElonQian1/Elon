use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS node_endpoint_session_authentication_receipts (
            authentication_receipt_id TEXT PRIMARY KEY CHECK(
                authentication_receipt_id=trim(authentication_receipt_id)
                AND length(CAST(authentication_receipt_id AS BLOB)) BETWEEN 1 AND 160
            ),
            authentication_schema TEXT NOT NULL CHECK(
                authentication_schema='elon.node_endpoint.session_authentication_receipt.v1'
            ),
            authentication_digest TEXT NOT NULL UNIQUE CHECK(
                length(authentication_digest)=64
                AND authentication_digest NOT GLOB '*[^0-9a-f]*'
            ),
            authentication_json TEXT NOT NULL CHECK(
                json_valid(authentication_json)
                AND json_type(authentication_json)='object'
                AND length(CAST(authentication_json AS BLOB))<=524288
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
            install_id TEXT NOT NULL CHECK(
                install_id=trim(install_id)
                AND length(CAST(install_id AS BLOB)) BETWEEN 1 AND 512
            ),
            installation_binding_digest TEXT NOT NULL CHECK(
                length(installation_binding_digest)=64
                AND installation_binding_digest NOT GLOB '*[^0-9a-f]*'
            ),
            session_id TEXT NOT NULL UNIQUE CHECK(
                session_id=trim(session_id)
                AND length(CAST(session_id AS BLOB)) BETWEEN 1 AND 160
            ),
            session_generation INTEGER NOT NULL CHECK(
                session_generation BETWEEN 1 AND 9007199254740991
            ),
            previous_authentication_receipt_id TEXT,
            previous_authentication_digest TEXT,
            server_instance_id TEXT NOT NULL CHECK(
                server_instance_id=trim(server_instance_id)
                AND length(CAST(server_instance_id AS BLOB)) BETWEEN 1 AND 160
            ),
            authentication_method TEXT NOT NULL CHECK(
                authentication_method='bearer_sha256'
            ),
            agent_version TEXT NOT NULL CHECK(
                agent_version=trim(agent_version)
                AND length(CAST(agent_version AS BLOB)) BETWEEN 1 AND 160
            ),
            protocol_version INTEGER NOT NULL CHECK(
                protocol_version BETWEEN 1 AND 9007199254740991
            ),
            capability_count INTEGER NOT NULL CHECK(
                capability_count BETWEEN 0 AND 256
            ),
            capability_set_json TEXT NOT NULL CHECK(
                json_valid(capability_set_json)
                AND json_type(capability_set_json)='array'
                AND length(CAST(capability_set_json AS BLOB))<=262144
            ),
            capability_set_digest TEXT NOT NULL CHECK(
                length(capability_set_digest)=64
                AND capability_set_digest NOT GLOB '*[^0-9a-f]*'
            ),
            transport_scheme TEXT NOT NULL CHECK(transport_scheme='wss'),
            transport_security_source TEXT NOT NULL CHECK(
                transport_security_source IN ('direct_tls','trusted_reverse_proxy_tls')
            ),
            transport_security_evidence_schema TEXT NOT NULL CHECK(
                transport_security_evidence_schema=trim(transport_security_evidence_schema)
                AND length(CAST(transport_security_evidence_schema AS BLOB))
                    BETWEEN 1 AND 160
            ),
            transport_security_evidence_id TEXT NOT NULL UNIQUE CHECK(
                transport_security_evidence_id=trim(transport_security_evidence_id)
                AND length(CAST(transport_security_evidence_id AS BLOB)) BETWEEN 1 AND 160
            ),
            transport_security_evidence_digest TEXT NOT NULL UNIQUE CHECK(
                length(transport_security_evidence_digest)=64
                AND transport_security_evidence_digest NOT GLOB '*[^0-9a-f]*'
            ),
            transport_verifier_revision INTEGER NOT NULL CHECK(
                transport_verifier_revision BETWEEN 1 AND 9007199254740991
            ),
            transport_verifier_digest TEXT NOT NULL CHECK(
                length(transport_verifier_digest)=64
                AND transport_verifier_digest NOT GLOB '*[^0-9a-f]*'
            ),
            transport_verified_at TEXT NOT NULL,
            authenticated_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            UNIQUE(agent_id, session_generation),
            UNIQUE(authentication_receipt_id, authentication_digest),
            CHECK(
                (session_generation=1
                    AND previous_authentication_receipt_id IS NULL
                    AND previous_authentication_digest IS NULL)
                OR (session_generation>1
                    AND previous_authentication_receipt_id=
                        trim(previous_authentication_receipt_id)
                    AND length(CAST(previous_authentication_receipt_id AS BLOB))
                        BETWEEN 1 AND 160
                    AND length(previous_authentication_digest)=64
                    AND previous_authentication_digest NOT GLOB '*[^0-9a-f]*')
            ),
            CHECK(authenticated_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(authenticated_at)=30 AND substr(authenticated_at,20,1)='.'
                AND substr(authenticated_at,30,1)='Z'
                AND julianday(authenticated_at) IS NOT NULL),
            CHECK(transport_verified_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(transport_verified_at)=30
                AND substr(transport_verified_at,20,1)='.'
                AND substr(transport_verified_at,30,1)='Z'
                AND julianday(transport_verified_at) IS NOT NULL),
            CHECK(expires_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(expires_at)=30 AND substr(expires_at,20,1)='.'
                AND substr(expires_at,30,1)='Z' AND julianday(expires_at) IS NOT NULL),
            CHECK(recorded_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(recorded_at)=30 AND substr(recorded_at,20,1)='.'
                AND substr(recorded_at,30,1)='Z' AND julianday(recorded_at) IS NOT NULL),
            CHECK(transport_verified_at<=authenticated_at
                AND authenticated_at<recorded_at AND recorded_at<expires_at),
            CHECK(
                unixepoch(expires_at)-unixepoch(authenticated_at)=900
                AND substr(expires_at,20,10)=substr(authenticated_at,20,10)
            ),
            FOREIGN KEY(credential_id, credential_revision, credential_digest)
                REFERENCES node_endpoint_credential_versions(
                    credential_id, credential_revision, credential_digest
                ) ON DELETE RESTRICT,
            FOREIGN KEY(agent_id) REFERENCES node_credentials(agent_id) ON DELETE RESTRICT,
            FOREIGN KEY(owner_user_id) REFERENCES users(id) ON DELETE RESTRICT,
            FOREIGN KEY(previous_authentication_receipt_id, previous_authentication_digest)
                REFERENCES node_endpoint_session_authentication_receipts(
                    authentication_receipt_id, authentication_digest
                ) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
        ) WITHOUT ROWID;

        CREATE TABLE IF NOT EXISTS node_endpoint_session_heads (
            agent_id TEXT PRIMARY KEY CHECK(
                agent_id=trim(agent_id)
                AND length(CAST(agent_id AS BLOB)) BETWEEN 1 AND 160
            ),
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
            authentication_receipt_id TEXT NOT NULL UNIQUE CHECK(
                authentication_receipt_id=trim(authentication_receipt_id)
                AND length(CAST(authentication_receipt_id AS BLOB)) BETWEEN 1 AND 160
            ),
            authentication_digest TEXT NOT NULL UNIQUE CHECK(
                length(authentication_digest)=64
                AND authentication_digest NOT GLOB '*[^0-9a-f]*'
            ),
            session_id TEXT NOT NULL UNIQUE CHECK(
                session_id=trim(session_id)
                AND length(CAST(session_id AS BLOB)) BETWEEN 1 AND 160
            ),
            session_generation INTEGER NOT NULL CHECK(
                session_generation BETWEEN 1 AND 9007199254740991
            ),
            server_instance_id TEXT NOT NULL CHECK(
                server_instance_id=trim(server_instance_id)
                AND length(CAST(server_instance_id AS BLOB)) BETWEEN 1 AND 160
            ),
            state TEXT NOT NULL CHECK(state IN (
                'active','closed','stale','credential_rotated','credential_revoked'
            )),
            authenticated_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            closed_at TEXT,
            close_reason_code TEXT,
            CHECK(
                (state='active' AND closed_at IS NULL AND close_reason_code IS NULL)
                OR (state!='active' AND closed_at IS NOT NULL
                    AND close_reason_code=trim(close_reason_code)
                    AND length(CAST(close_reason_code AS BLOB)) BETWEEN 1 AND 160)
            ),
            CHECK(authenticated_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(authenticated_at)=30 AND substr(authenticated_at,20,1)='.'
                AND substr(authenticated_at,30,1)='Z'
                AND julianday(authenticated_at) IS NOT NULL),
            CHECK(expires_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(expires_at)=30 AND substr(expires_at,20,1)='.'
                AND substr(expires_at,30,1)='Z' AND julianday(expires_at) IS NOT NULL),
            CHECK(created_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(created_at)=30 AND substr(created_at,20,1)='.'
                AND substr(created_at,30,1)='Z' AND julianday(created_at) IS NOT NULL),
            CHECK(updated_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(updated_at)=30 AND substr(updated_at,20,1)='.'
                AND substr(updated_at,30,1)='Z' AND julianday(updated_at) IS NOT NULL),
            CHECK(closed_at IS NULL OR (
                closed_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(closed_at)=30 AND substr(closed_at,20,1)='.'
                AND substr(closed_at,30,1)='Z' AND julianday(closed_at) IS NOT NULL
            )),
            CHECK(authenticated_at<=created_at AND created_at<=updated_at
                AND authenticated_at<expires_at
                AND (closed_at IS NULL OR authenticated_at<=closed_at)),
            FOREIGN KEY(credential_id, credential_revision, credential_digest)
                REFERENCES node_endpoint_credential_versions(
                    credential_id, credential_revision, credential_digest
                ) ON DELETE RESTRICT,
            FOREIGN KEY(authentication_receipt_id, authentication_digest)
                REFERENCES node_endpoint_session_authentication_receipts(
                    authentication_receipt_id, authentication_digest
                ) ON DELETE RESTRICT,
            FOREIGN KEY(agent_id) REFERENCES node_credentials(agent_id) ON DELETE RESTRICT
        ) WITHOUT ROWID;

        CREATE INDEX IF NOT EXISTS idx_node_endpoint_session_receipts_agent_time
            ON node_endpoint_session_authentication_receipts(
                agent_id, authenticated_at DESC, session_generation DESC
            );
        CREATE INDEX IF NOT EXISTS idx_node_endpoint_session_heads_state_expiry
            ON node_endpoint_session_heads(state, expires_at, agent_id);
        CREATE INDEX IF NOT EXISTS idx_node_endpoint_session_heads_server_instance
            ON node_endpoint_session_heads(server_instance_id, state, agent_id);
        "#,
    )?;
    Ok(())
}
