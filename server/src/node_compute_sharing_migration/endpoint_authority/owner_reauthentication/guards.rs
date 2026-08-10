use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_owner_reauthentication_projection
        BEFORE INSERT ON node_endpoint_owner_reauthentication_receipts
        WHEN json_type(NEW.reauthentication_json)!='object'
          OR (SELECT COUNT(*) FROM json_each(NEW.reauthentication_json))!=33
          OR json_extract(NEW.reauthentication_json,'$.schema')
                IS NOT NEW.reauthentication_schema
          OR json_extract(NEW.reauthentication_json,'$.reauthentication_receipt_id')
                IS NOT NEW.reauthentication_receipt_id
          OR json_extract(NEW.reauthentication_json,'$.owner_user_id')
                IS NOT NEW.owner_user_id
          OR json_extract(NEW.reauthentication_json,'$.account_session_id')
                IS NOT NEW.account_session_id
          OR json_extract(NEW.reauthentication_json,'$.session_binding_digest')
                IS NOT NEW.session_binding_digest
          OR json_extract(NEW.reauthentication_json,'$.account_auth_state_digest')
                IS NOT NEW.account_auth_state_digest
          OR json_extract(NEW.reauthentication_json,'$.authentication_method')
                IS NOT NEW.authentication_method
          OR json_extract(NEW.reauthentication_json,'$.authentication_factor_id')
                IS NOT NEW.authentication_factor_id
          OR json_extract(
                NEW.reauthentication_json,
                '$.authentication_factor_binding_digest'
             ) IS NOT NEW.authentication_factor_binding_digest
          OR json_extract(NEW.reauthentication_json,'$.authentication_evidence_id')
                IS NOT NEW.authentication_evidence_id
          OR json_extract(
                NEW.reauthentication_json,
                '$.authentication_evidence_digest'
             ) IS NOT NEW.authentication_evidence_digest
          OR json_extract(
                NEW.reauthentication_json,
                '$.authorization_issuance_request_id'
             ) IS NOT NEW.authorization_issuance_request_id
          OR json_extract(NEW.reauthentication_json,'$.authorization_action')
                IS NOT NEW.authorization_action
          OR json_extract(
                NEW.reauthentication_json,
                '$.credential_mutation_request_id'
             ) IS NOT NEW.credential_mutation_request_id
          OR json_extract(
                NEW.reauthentication_json,
                '$.credential_mutation_request_digest'
             ) IS NOT NEW.credential_mutation_request_digest
          OR json_extract(
                NEW.reauthentication_json,
                '$.authorization_target_digest'
             ) IS NOT NEW.authorization_target_digest
          OR json_extract(NEW.reauthentication_json,'$.agent_id') IS NOT NEW.agent_id
          OR json_extract(NEW.reauthentication_json,'$.install_id') IS NOT NEW.install_id
          OR (
                NEW.expected_credential_id IS NULL
                AND json_type(NEW.reauthentication_json,'$.expected_credential_id')
                    IS NOT 'null'
             )
          OR (
                NEW.expected_credential_id IS NOT NULL
                AND json_extract(NEW.reauthentication_json,'$.expected_credential_id')
                    IS NOT NEW.expected_credential_id
             )
          OR (
                NEW.expected_credential_revision IS NULL
                AND json_type(
                    NEW.reauthentication_json,
                    '$.expected_credential_revision'
                ) IS NOT 'null'
             )
          OR (
                NEW.expected_credential_revision IS NOT NULL
                AND json_extract(
                    NEW.reauthentication_json,
                    '$.expected_credential_revision'
                ) IS NOT NEW.expected_credential_revision
             )
          OR (
                NEW.expected_credential_digest IS NULL
                AND json_type(NEW.reauthentication_json,'$.expected_credential_digest')
                    IS NOT 'null'
             )
          OR (
                NEW.expected_credential_digest IS NOT NULL
                AND json_extract(NEW.reauthentication_json,'$.expected_credential_digest')
                    IS NOT NEW.expected_credential_digest
             )
          OR json_extract(NEW.reauthentication_json,'$.secure_transport_source')
                IS NOT NEW.secure_transport_source
          OR json_extract(
                NEW.reauthentication_json,
                '$.secure_transport_evidence_schema'
             ) IS NOT NEW.secure_transport_evidence_schema
          OR json_extract(
                NEW.reauthentication_json,
                '$.secure_transport_evidence_id'
             ) IS NOT NEW.secure_transport_evidence_id
          OR json_extract(
                NEW.reauthentication_json,
                '$.secure_transport_evidence_digest'
             ) IS NOT NEW.secure_transport_evidence_digest
          OR json_extract(
                NEW.reauthentication_json,
                '$.secure_transport_verifier_revision'
             ) IS NOT NEW.secure_transport_verifier_revision
          OR json_extract(
                NEW.reauthentication_json,
                '$.secure_transport_verifier_digest'
             ) IS NOT NEW.secure_transport_verifier_digest
          OR json_extract(
                NEW.reauthentication_json,
                '$.secure_transport_server_instance_id'
             ) IS NOT NEW.secure_transport_server_instance_id
          OR json_extract(
                NEW.reauthentication_json,
                '$.secure_transport_request_binding_digest'
             ) IS NOT NEW.secure_transport_request_binding_digest
          OR json_extract(
                NEW.reauthentication_json,
                '$.secure_transport_verified_at'
             ) IS NOT NEW.secure_transport_verified_at
          OR json_extract(NEW.reauthentication_json,'$.reauthenticated_at')
                IS NOT NEW.reauthenticated_at
          OR json_extract(NEW.reauthentication_json,'$.expires_at') IS NOT NEW.expires_at
          OR json_extract(NEW.reauthentication_json,'$.recorded_at') IS NOT NEW.recorded_at
          OR json_type(NEW.reauthentication_json,'$.reauthentication_digest') IS NOT NULL
          OR json_type(NEW.reauthentication_json,'$.reauthentication_json') IS NOT NULL
          OR json_type(NEW.reauthentication_json,'$.canonicalization') IS NOT NULL
          OR json_type(NEW.reauthentication_json,'$.digest_algorithm') IS NOT NULL
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint owner reauthentication projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_owner_reauthentication_session_owner
        BEFORE INSERT ON node_endpoint_owner_reauthentication_receipts
        WHEN NOT EXISTS (
            SELECT 1 FROM sessions session
            JOIN users owner ON owner.id=session.user_id
             WHERE session.id=NEW.account_session_id
               AND session.user_id=NEW.owner_user_id
               AND session.revoked_at IS NULL
               AND julianday(session.expires_at)>julianday(NEW.recorded_at)
               AND owner.status='active'
               AND (
                    (NEW.authentication_method='password'
                        AND owner.password_login_enabled=1)
                    OR (NEW.authentication_method='google_oidc'
                        AND EXISTS (
                            SELECT 1 FROM user_identities identity
                             WHERE identity.id=NEW.authentication_factor_id
                               AND identity.user_id=NEW.owner_user_id
                               AND identity.provider='google'
                        ))
               )
        )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint owner reauthentication session owner mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_owner_reauthentication_target_current
        BEFORE INSERT ON node_endpoint_owner_reauthentication_receipts
        WHEN NOT EXISTS (
            SELECT 1 FROM node_credentials legacy
             WHERE legacy.agent_id=NEW.agent_id
               AND legacy.owner_user_id=NEW.owner_user_id
               AND legacy.install_id=NEW.install_id
        )
          OR (
            NEW.authorization_action='initial_registration'
            AND EXISTS (
                SELECT 1 FROM node_endpoint_credentials root
                 WHERE root.agent_id=NEW.agent_id
                    OR (root.owner_user_id=NEW.owner_user_id
                        AND root.install_id=NEW.install_id)
            )
          )
          OR (
            NEW.authorization_action!='initial_registration'
            AND NOT EXISTS (
                SELECT 1 FROM node_endpoint_credentials root
                 WHERE root.agent_id=NEW.agent_id
                   AND root.owner_user_id=NEW.owner_user_id
                   AND root.install_id=NEW.install_id
                   AND root.credential_id=NEW.expected_credential_id
                   AND root.current_credential_revision=NEW.expected_credential_revision
                   AND root.current_credential_digest=NEW.expected_credential_digest
                   AND (
                        (NEW.authorization_action='account_recovery'
                            AND root.status IN ('active','revoked'))
                        OR (NEW.authorization_action IN (
                                'credential_rotation','owner_revocation'
                            ) AND root.status='active')
                   )
            )
          )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint owner reauthentication target not current');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_owner_reauthentication_no_replace
        BEFORE INSERT ON node_endpoint_owner_reauthentication_receipts
        WHEN EXISTS (
            SELECT 1 FROM node_endpoint_owner_reauthentication_receipts stored
             WHERE stored.reauthentication_receipt_id=NEW.reauthentication_receipt_id
                OR stored.reauthentication_digest=NEW.reauthentication_digest
                OR (stored.owner_user_id=NEW.owner_user_id
                    AND stored.authorization_issuance_request_id=
                        NEW.authorization_issuance_request_id)
                OR stored.authentication_evidence_id=NEW.authentication_evidence_id
                OR stored.authentication_evidence_digest=NEW.authentication_evidence_digest
                OR stored.secure_transport_evidence_id=NEW.secure_transport_evidence_id
                OR stored.secure_transport_evidence_digest=
                    NEW.secure_transport_evidence_digest
        )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint owner reauthentication replacement is forbidden');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_owner_reauthentication_immutable
        BEFORE UPDATE ON node_endpoint_owner_reauthentication_receipts
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint owner reauthentication receipts are immutable');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_owner_reauthentication_append_only
        BEFORE DELETE ON node_endpoint_owner_reauthentication_receipts
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint owner reauthentication receipts are append-only');
        END;
        "#,
    )?;
    Ok(())
}
