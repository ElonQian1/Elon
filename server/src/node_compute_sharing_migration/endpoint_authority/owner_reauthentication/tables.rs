use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS node_endpoint_owner_reauthentication_receipts (
            reauthentication_receipt_id TEXT PRIMARY KEY CHECK(
                reauthentication_receipt_id=trim(reauthentication_receipt_id)
                AND length(CAST(reauthentication_receipt_id AS BLOB)) BETWEEN 1 AND 160
            ),
            reauthentication_schema TEXT NOT NULL CHECK(
                reauthentication_schema='elon.node_endpoint.owner_reauthentication.v1'
            ),
            reauthentication_digest TEXT NOT NULL UNIQUE CHECK(
                length(reauthentication_digest)=64
                AND reauthentication_digest NOT GLOB '*[^0-9a-f]*'
            ),
            reauthentication_json TEXT NOT NULL CHECK(
                json_valid(reauthentication_json)
                AND json_type(reauthentication_json)='object'
                AND length(CAST(reauthentication_json AS BLOB))<=524288
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            owner_user_id TEXT NOT NULL CHECK(
                owner_user_id=trim(owner_user_id)
                AND length(CAST(owner_user_id AS BLOB)) BETWEEN 1 AND 160
            ),
            account_session_id TEXT NOT NULL CHECK(
                account_session_id=trim(account_session_id)
                AND length(CAST(account_session_id AS BLOB)) BETWEEN 1 AND 160
            ),
            session_binding_digest TEXT NOT NULL CHECK(
                length(session_binding_digest)=64
                AND session_binding_digest NOT GLOB '*[^0-9a-f]*'
            ),
            account_auth_state_digest TEXT NOT NULL CHECK(
                length(account_auth_state_digest)=64
                AND account_auth_state_digest NOT GLOB '*[^0-9a-f]*'
            ),
            authentication_method TEXT NOT NULL CHECK(
                authentication_method IN ('password','google_oidc')
            ),
            authentication_factor_id TEXT NOT NULL CHECK(
                authentication_factor_id=trim(authentication_factor_id)
                AND length(CAST(authentication_factor_id AS BLOB)) BETWEEN 1 AND 160
            ),
            authentication_factor_binding_digest TEXT NOT NULL CHECK(
                length(authentication_factor_binding_digest)=64
                AND authentication_factor_binding_digest NOT GLOB '*[^0-9a-f]*'
            ),
            authentication_evidence_id TEXT NOT NULL UNIQUE CHECK(
                authentication_evidence_id=trim(authentication_evidence_id)
                AND length(CAST(authentication_evidence_id AS BLOB)) BETWEEN 1 AND 160
            ),
            authentication_evidence_digest TEXT NOT NULL UNIQUE CHECK(
                length(authentication_evidence_digest)=64
                AND authentication_evidence_digest NOT GLOB '*[^0-9a-f]*'
            ),
            authorization_issuance_request_id TEXT NOT NULL CHECK(
                authorization_issuance_request_id=trim(authorization_issuance_request_id)
                AND length(CAST(authorization_issuance_request_id AS BLOB))
                    BETWEEN 1 AND 160
            ),
            authorization_action TEXT NOT NULL CHECK(authorization_action IN (
                'initial_registration','credential_rotation',
                'account_recovery','owner_revocation'
            )),
            credential_mutation_request_id TEXT NOT NULL CHECK(
                credential_mutation_request_id=trim(credential_mutation_request_id)
                AND length(CAST(credential_mutation_request_id AS BLOB)) BETWEEN 1 AND 160
            ),
            credential_mutation_request_digest TEXT NOT NULL CHECK(
                length(credential_mutation_request_digest)=64
                AND credential_mutation_request_digest NOT GLOB '*[^0-9a-f]*'
            ),
            authorization_target_digest TEXT NOT NULL CHECK(
                length(authorization_target_digest)=64
                AND authorization_target_digest NOT GLOB '*[^0-9a-f]*'
            ),
            agent_id TEXT NOT NULL CHECK(
                agent_id=trim(agent_id)
                AND length(CAST(agent_id AS BLOB)) BETWEEN 1 AND 160
            ),
            install_id TEXT NOT NULL CHECK(
                install_id=trim(install_id)
                AND length(CAST(install_id AS BLOB)) BETWEEN 1 AND 512
            ),
            expected_credential_id TEXT,
            expected_credential_revision INTEGER,
            expected_credential_digest TEXT,
            secure_transport_source TEXT NOT NULL CHECK(
                secure_transport_source IN ('direct_tls','trusted_proxy_mtls')
            ),
            secure_transport_evidence_schema TEXT NOT NULL CHECK(
                secure_transport_evidence_schema=trim(secure_transport_evidence_schema)
                AND length(CAST(secure_transport_evidence_schema AS BLOB))
                    BETWEEN 1 AND 160
            ),
            secure_transport_evidence_id TEXT NOT NULL UNIQUE CHECK(
                secure_transport_evidence_id=trim(secure_transport_evidence_id)
                AND length(CAST(secure_transport_evidence_id AS BLOB)) BETWEEN 1 AND 160
            ),
            secure_transport_evidence_digest TEXT NOT NULL UNIQUE CHECK(
                length(secure_transport_evidence_digest)=64
                AND secure_transport_evidence_digest NOT GLOB '*[^0-9a-f]*'
            ),
            secure_transport_verifier_revision INTEGER NOT NULL CHECK(
                secure_transport_verifier_revision BETWEEN 1 AND 9007199254740991
            ),
            secure_transport_verifier_digest TEXT NOT NULL CHECK(
                length(secure_transport_verifier_digest)=64
                AND secure_transport_verifier_digest NOT GLOB '*[^0-9a-f]*'
            ),
            secure_transport_server_instance_id TEXT NOT NULL CHECK(
                secure_transport_server_instance_id=trim(secure_transport_server_instance_id)
                AND length(CAST(secure_transport_server_instance_id AS BLOB))
                    BETWEEN 1 AND 160
            ),
            secure_transport_request_binding_digest TEXT NOT NULL CHECK(
                length(secure_transport_request_binding_digest)=64
                AND secure_transport_request_binding_digest NOT GLOB '*[^0-9a-f]*'
            ),
            secure_transport_verified_at TEXT NOT NULL,
            reauthenticated_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            UNIQUE(owner_user_id, authorization_issuance_request_id),
            CHECK(
                (authorization_action='initial_registration'
                    AND expected_credential_id IS NULL
                    AND expected_credential_revision IS NULL
                    AND expected_credential_digest IS NULL)
                OR (authorization_action!='initial_registration'
                    AND expected_credential_id IS NOT NULL
                    AND expected_credential_revision IS NOT NULL
                    AND expected_credential_digest IS NOT NULL
                    AND expected_credential_id=trim(expected_credential_id)
                    AND length(CAST(expected_credential_id AS BLOB)) BETWEEN 1 AND 160
                    AND expected_credential_revision BETWEEN 1 AND 9007199254740991
                    AND length(expected_credential_digest)=64
                    AND expected_credential_digest NOT GLOB '*[^0-9a-f]*')
            ),
            CHECK(
                (authentication_method='password'
                    AND authentication_factor_id='password')
                OR (authentication_method='google_oidc'
                    AND authentication_factor_id!='password')
            ),
            CHECK(
                secure_transport_request_binding_digest=
                    credential_mutation_request_digest
            ),
            CHECK(secure_transport_verified_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(secure_transport_verified_at)=30
                AND substr(secure_transport_verified_at,20,1)='.'
                AND substr(secure_transport_verified_at,30,1)='Z'
                AND julianday(secure_transport_verified_at) IS NOT NULL),
            CHECK(reauthenticated_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(reauthenticated_at)=30
                AND substr(reauthenticated_at,20,1)='.'
                AND substr(reauthenticated_at,30,1)='Z'
                AND julianday(reauthenticated_at) IS NOT NULL),
            CHECK(expires_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(expires_at)=30 AND substr(expires_at,20,1)='.'
                AND substr(expires_at,30,1)='Z' AND julianday(expires_at) IS NOT NULL),
            CHECK(recorded_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(recorded_at)=30 AND substr(recorded_at,20,1)='.'
                AND substr(recorded_at,30,1)='Z' AND julianday(recorded_at) IS NOT NULL),
            CHECK(secure_transport_verified_at<=reauthenticated_at
                AND reauthenticated_at<=recorded_at AND recorded_at<expires_at),
            CHECK(
                unixepoch(reauthenticated_at)-unixepoch(secure_transport_verified_at)
                    BETWEEN 0 AND 30
                AND (
                    unixepoch(reauthenticated_at)-
                        unixepoch(secure_transport_verified_at)<30
                    OR substr(reauthenticated_at,20,10)<=
                        substr(secure_transport_verified_at,20,10)
                )
            ),
            CHECK(
                unixepoch(expires_at)-unixepoch(reauthenticated_at)=300
                AND substr(expires_at,20,10)=substr(reauthenticated_at,20,10)
            ),
            FOREIGN KEY(owner_user_id) REFERENCES users(id) ON DELETE RESTRICT,
            FOREIGN KEY(account_session_id) REFERENCES sessions(id) ON DELETE RESTRICT,
            FOREIGN KEY(agent_id) REFERENCES node_credentials(agent_id) ON DELETE RESTRICT
        ) WITHOUT ROWID;
        "#,
    )?;
    Ok(())
}
