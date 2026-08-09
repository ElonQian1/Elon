use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_credential_version_projection
        BEFORE INSERT ON node_endpoint_credential_versions
        WHEN json_type(NEW.credential_json)!='object'
          OR (SELECT COUNT(*) FROM json_each(NEW.credential_json))!=16
          OR json_type(NEW.credential_json,'$.owner_authorization_basis')!='object'
          OR (
                SELECT COUNT(*)
                  FROM json_each(NEW.credential_json,'$.owner_authorization_basis')
             )!=3
          OR json_extract(NEW.credential_json,'$.schema') IS NOT NEW.credential_schema
          OR json_extract(NEW.credential_json,'$.credential_id') IS NOT NEW.credential_id
          OR json_extract(NEW.credential_json,'$.credential_revision')
                IS NOT NEW.credential_revision
          OR json_extract(NEW.credential_json,'$.agent_id') IS NOT NEW.agent_id
          OR json_extract(NEW.credential_json,'$.owner_user_id') IS NOT NEW.owner_user_id
          OR json_extract(NEW.credential_json,'$.install_id') IS NOT NEW.install_id
          OR json_extract(NEW.credential_json,'$.installation_binding_digest')
                IS NOT NEW.installation_binding_digest
          OR json_extract(NEW.credential_json,'$.secret_verifier_digest')
                IS NOT NEW.secret_verifier_digest
          OR json_extract(NEW.credential_json,'$.issuance_kind') IS NOT NEW.issuance_kind
          OR json_extract(NEW.credential_json,'$.issuance_request_id')
                IS NOT NEW.issuance_request_id
          OR json_extract(NEW.credential_json,'$.issued_by_user_id')
                IS NOT NEW.issued_by_user_id
          OR json_extract(NEW.credential_json,'$.owner_authorization_basis.kind')
                IS NOT NEW.owner_authorization_basis_kind
          OR json_extract(NEW.credential_json,'$.owner_authorization_basis.basis_id')
                IS NOT NEW.owner_authorization_basis_id
          OR json_extract(NEW.credential_json,'$.owner_authorization_basis.basis_digest')
                IS NOT NEW.owner_authorization_basis_digest
          OR (
                NEW.previous_credential_revision IS NULL
                AND json_type(
                    NEW.credential_json,
                    '$.previous_credential_revision'
                ) IS NOT 'null'
             )
          OR (
                NEW.previous_credential_revision IS NOT NULL
                AND json_extract(
                    NEW.credential_json,
                    '$.previous_credential_revision'
                ) IS NOT NEW.previous_credential_revision
             )
          OR (
                NEW.previous_credential_digest IS NULL
                AND json_type(
                    NEW.credential_json,
                    '$.previous_credential_digest'
                ) IS NOT 'null'
             )
          OR (
                NEW.previous_credential_digest IS NOT NULL
                AND json_extract(
                    NEW.credential_json,
                    '$.previous_credential_digest'
                ) IS NOT NEW.previous_credential_digest
             )
          OR json_extract(NEW.credential_json,'$.issued_at') IS NOT NEW.issued_at
          OR json_extract(NEW.credential_json,'$.recorded_at') IS NOT NEW.recorded_at
          OR json_type(NEW.credential_json,'$.credential_digest') IS NOT NULL
          OR json_type(NEW.credential_json,'$.canonicalization') IS NOT NULL
          OR json_type(NEW.credential_json,'$.digest_algorithm') IS NOT NULL
          OR json_type(NEW.credential_json,'$.secret_hash') IS NOT NULL
          OR json_type(NEW.credential_json,'$.secret_hash_algorithm') IS NOT NULL
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint credential projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_credential_version_source
        BEFORE INSERT ON node_endpoint_credential_versions
        WHEN NOT EXISTS (
                SELECT 1 FROM node_credentials legacy
                 WHERE legacy.agent_id=NEW.agent_id
                   AND legacy.owner_user_id=NEW.owner_user_id
                   AND legacy.install_id=trim(legacy.install_id)
                   AND legacy.install_id=NEW.install_id
                   AND length(CAST(legacy.install_id AS BLOB)) BETWEEN 1 AND 512
             )
          OR (
                NEW.owner_authorization_basis_kind!='security_operator'
                AND NEW.issued_by_user_id!=NEW.owner_user_id
             )
          OR (
                NEW.credential_revision=1
                AND EXISTS (
                    SELECT 1 FROM node_endpoint_credentials root
                     WHERE root.credential_id=NEW.credential_id
                        OR root.agent_id=NEW.agent_id
                        OR (root.owner_user_id=NEW.owner_user_id
                            AND root.install_id=NEW.install_id)
                )
             )
          OR (
                NEW.credential_revision>1
                AND NOT EXISTS (
                    SELECT 1
                      FROM node_endpoint_credentials root
                      JOIN node_endpoint_credential_versions previous
                        ON previous.credential_id=root.credential_id
                       AND previous.credential_revision=root.current_credential_revision
                       AND previous.credential_digest=root.current_credential_digest
                     WHERE root.credential_id=NEW.credential_id
                       AND root.agent_id=NEW.agent_id
                       AND root.owner_user_id=NEW.owner_user_id
                       AND root.install_id=NEW.install_id
                       AND root.installation_binding_digest=NEW.installation_binding_digest
                       AND (
                            root.status='active'
                            OR (
                                root.status='revoked'
                                AND NEW.issuance_kind='account_recovery'
                                AND EXISTS (
                                    SELECT 1
                                      FROM node_endpoint_credential_revocations terminal
                                     WHERE terminal.credential_id=root.credential_id
                                       AND terminal.credential_revision=
                                            root.current_credential_revision
                                       AND terminal.credential_digest=
                                            root.current_credential_digest
                                       AND terminal.revocation_kind IN (
                                            'owner_revoked','security_revoked'
                                       )
                                )
                            )
                       )
                       AND root.updated_at<=NEW.issued_at
                       AND root.current_credential_revision=NEW.previous_credential_revision
                       AND root.current_credential_digest=NEW.previous_credential_digest
                       AND previous.agent_id=NEW.agent_id
                       AND previous.owner_user_id=NEW.owner_user_id
                       AND previous.install_id=NEW.install_id
                       AND previous.installation_binding_digest=
                            NEW.installation_binding_digest
                       AND previous.recorded_at<=NEW.issued_at
                       AND NOT EXISTS (
                            SELECT 1 FROM node_endpoint_session_heads session
                             WHERE session.agent_id=NEW.agent_id
                               AND session.credential_id=NEW.credential_id
                               AND session.state='active'
                       )
                )
             )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint credential lacks exact issuance source');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_credential_revocation_projection
        BEFORE INSERT ON node_endpoint_credential_revocations
        WHEN json_type(NEW.revocation_json)!='object'
          OR (SELECT COUNT(*) FROM json_each(NEW.revocation_json))!=14
          OR json_type(NEW.revocation_json,'$.owner_authorization_basis')!='object'
          OR (
                SELECT COUNT(*)
                  FROM json_each(NEW.revocation_json,'$.owner_authorization_basis')
             )!=3
          OR json_extract(NEW.revocation_json,'$.schema') IS NOT NEW.revocation_schema
          OR json_extract(NEW.revocation_json,'$.revocation_id') IS NOT NEW.revocation_id
          OR json_extract(NEW.revocation_json,'$.credential_id') IS NOT NEW.credential_id
          OR json_extract(NEW.revocation_json,'$.credential_revision')
                IS NOT NEW.credential_revision
          OR json_extract(NEW.revocation_json,'$.credential_digest')
                IS NOT NEW.credential_digest
          OR json_extract(NEW.revocation_json,'$.agent_id') IS NOT NEW.agent_id
          OR json_extract(NEW.revocation_json,'$.owner_user_id') IS NOT NEW.owner_user_id
          OR json_extract(NEW.revocation_json,'$.revocation_kind')
                IS NOT NEW.revocation_kind
          OR json_extract(NEW.revocation_json,'$.reason_code') IS NOT NEW.reason_code
          OR json_extract(NEW.revocation_json,'$.mutation_request_id')
                IS NOT NEW.mutation_request_id
          OR json_extract(NEW.revocation_json,'$.revoked_by_user_id')
                IS NOT NEW.revoked_by_user_id
          OR json_extract(NEW.revocation_json,'$.owner_authorization_basis.kind')
                IS NOT NEW.owner_authorization_basis_kind
          OR json_extract(NEW.revocation_json,'$.owner_authorization_basis.basis_id')
                IS NOT NEW.owner_authorization_basis_id
          OR json_extract(NEW.revocation_json,'$.owner_authorization_basis.basis_digest')
                IS NOT NEW.owner_authorization_basis_digest
          OR json_extract(NEW.revocation_json,'$.revoked_at') IS NOT NEW.revoked_at
          OR json_extract(NEW.revocation_json,'$.recorded_at') IS NOT NEW.recorded_at
          OR json_type(NEW.revocation_json,'$.revocation_digest') IS NOT NULL
          OR json_type(NEW.revocation_json,'$.canonicalization') IS NOT NULL
          OR json_type(NEW.revocation_json,'$.digest_algorithm') IS NOT NULL
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint credential revocation projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_credential_revocation_source
        BEFORE INSERT ON node_endpoint_credential_revocations
        WHEN NOT EXISTS (
                SELECT 1
                  FROM node_endpoint_credentials root
                  JOIN node_endpoint_credential_versions version
                    ON version.credential_id=root.credential_id
                   AND version.credential_revision=root.current_credential_revision
                   AND version.credential_digest=root.current_credential_digest
                 WHERE root.credential_id=NEW.credential_id
                   AND root.current_credential_revision=NEW.credential_revision
                   AND root.current_credential_digest=NEW.credential_digest
                   AND root.agent_id=NEW.agent_id
                   AND root.owner_user_id=NEW.owner_user_id
                   AND root.status='active'
                   AND version.agent_id=NEW.agent_id
                   AND version.owner_user_id=NEW.owner_user_id
                   AND version.recorded_at<=NEW.revoked_at
                   AND NOT EXISTS (
                        SELECT 1 FROM node_endpoint_session_heads session
                         WHERE session.agent_id=NEW.agent_id
                           AND session.credential_id=NEW.credential_id
                           AND session.state='active'
                   )
             )
          OR (
                NEW.revocation_kind IN ('rotated','recovered')
                AND NOT EXISTS (
                    SELECT 1 FROM node_endpoint_credential_versions successor
                     WHERE successor.credential_id=NEW.credential_id
                       AND successor.credential_revision=NEW.credential_revision+1
                       AND successor.previous_credential_revision=NEW.credential_revision
                       AND successor.previous_credential_digest=NEW.credential_digest
                       AND successor.agent_id=NEW.agent_id
                       AND successor.owner_user_id=NEW.owner_user_id
                       AND successor.issuance_request_id=NEW.mutation_request_id
                       AND successor.issued_by_user_id=NEW.revoked_by_user_id
                       AND successor.owner_authorization_basis_kind=
                            NEW.owner_authorization_basis_kind
                       AND successor.owner_authorization_basis_id=
                            NEW.owner_authorization_basis_id
                       AND successor.owner_authorization_basis_digest=
                            NEW.owner_authorization_basis_digest
                       AND successor.issued_at=NEW.revoked_at
                       AND successor.recorded_at=NEW.recorded_at
                       AND (
                            (NEW.revocation_kind='rotated'
                                AND successor.issuance_kind='credential_rotation')
                            OR (NEW.revocation_kind='recovered'
                                AND successor.issuance_kind='account_recovery')
                       )
                )
             )
          OR (
                NEW.revocation_kind='owner_revoked'
                AND NEW.owner_authorization_basis_kind='security_operator'
             )
          OR (
                NEW.revocation_kind='security_revoked'
                AND NEW.owner_authorization_basis_kind!='security_operator'
             )
          OR (
                NEW.owner_authorization_basis_kind!='security_operator'
                AND NEW.revoked_by_user_id!=(
                    SELECT root.owner_user_id
                      FROM node_endpoint_credentials root
                     WHERE root.credential_id=NEW.credential_id
                )
             )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint credential revocation lacks current source');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_credentials_initial_exact
        BEFORE INSERT ON node_endpoint_credentials
        WHEN NEW.status!='active'
          OR NEW.current_credential_revision!=1
          OR NOT EXISTS (
                SELECT 1 FROM node_endpoint_credential_versions version
                 WHERE version.credential_id=NEW.credential_id
                   AND version.credential_revision=1
                   AND version.credential_digest=NEW.current_credential_digest
                   AND version.agent_id=NEW.agent_id
                   AND version.owner_user_id=NEW.owner_user_id
                   AND version.install_id=NEW.install_id
                   AND version.installation_binding_digest=NEW.installation_binding_digest
                   AND version.previous_credential_revision IS NULL
                   AND version.previous_credential_digest IS NULL
                   AND NEW.created_at=version.recorded_at
                   AND NEW.updated_at=version.recorded_at
             )
          OR EXISTS (
                SELECT 1 FROM node_endpoint_credential_revocations revoked
                 WHERE revoked.credential_id=NEW.credential_id
                   AND revoked.credential_revision=1
             )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint credential root lacks exact initial version');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_credentials_linear_transition
        BEFORE UPDATE ON node_endpoint_credentials
        WHEN NEW.credential_id!=OLD.credential_id
          OR NEW.agent_id!=OLD.agent_id
          OR NEW.owner_user_id!=OLD.owner_user_id
          OR NEW.install_id!=OLD.install_id
          OR NEW.installation_binding_digest!=OLD.installation_binding_digest
          OR NEW.created_at!=OLD.created_at
          OR NEW.updated_at<=OLD.updated_at
          OR NOT (
                (
                    OLD.status='active' AND NEW.status='active'
                    AND NEW.current_credential_revision=OLD.current_credential_revision+1
                    AND NEW.current_credential_digest!=OLD.current_credential_digest
                    AND EXISTS (
                        SELECT 1
                          FROM node_endpoint_credential_versions version
                          JOIN node_endpoint_credential_revocations revoked
                            ON revoked.credential_id=OLD.credential_id
                           AND revoked.credential_revision=OLD.current_credential_revision
                           AND revoked.credential_digest=OLD.current_credential_digest
                         WHERE version.credential_id=NEW.credential_id
                           AND version.credential_revision=NEW.current_credential_revision
                           AND version.credential_digest=NEW.current_credential_digest
                           AND version.agent_id=NEW.agent_id
                           AND version.owner_user_id=NEW.owner_user_id
                           AND version.install_id=NEW.install_id
                           AND version.installation_binding_digest=
                                NEW.installation_binding_digest
                           AND version.previous_credential_revision=
                                OLD.current_credential_revision
                           AND version.previous_credential_digest=
                                OLD.current_credential_digest
                           AND version.issuance_request_id=revoked.mutation_request_id
                           AND version.issued_by_user_id=revoked.revoked_by_user_id
                           AND version.owner_authorization_basis_kind=
                                revoked.owner_authorization_basis_kind
                           AND version.owner_authorization_basis_id=
                                revoked.owner_authorization_basis_id
                           AND version.owner_authorization_basis_digest=
                                revoked.owner_authorization_basis_digest
                           AND version.issued_at=revoked.revoked_at
                           AND version.recorded_at=revoked.recorded_at
                           AND NEW.updated_at=version.recorded_at
                           AND (
                                (version.issuance_kind='credential_rotation'
                                    AND revoked.revocation_kind='rotated')
                                OR (version.issuance_kind='account_recovery'
                                    AND revoked.revocation_kind='recovered')
                           )
                    )
                    AND NOT EXISTS (
                        SELECT 1 FROM node_endpoint_session_heads session
                         WHERE session.agent_id=OLD.agent_id
                           AND session.credential_id=OLD.credential_id
                           AND session.state='active'
                    )
                )
                OR (
                    OLD.status='active' AND NEW.status='revoked'
                    AND NEW.current_credential_revision=OLD.current_credential_revision
                    AND NEW.current_credential_digest=OLD.current_credential_digest
                    AND EXISTS (
                        SELECT 1 FROM node_endpoint_credential_revocations revoked
                         WHERE revoked.credential_id=OLD.credential_id
                           AND revoked.credential_revision=OLD.current_credential_revision
                           AND revoked.credential_digest=OLD.current_credential_digest
                           AND revoked.revocation_kind IN (
                                'owner_revoked','security_revoked'
                           )
                           AND NEW.updated_at=revoked.recorded_at
                    )
                    AND NOT EXISTS (
                        SELECT 1 FROM node_endpoint_session_heads session
                         WHERE session.agent_id=OLD.agent_id
                           AND session.credential_id=OLD.credential_id
                           AND session.state='active'
                    )
                )
                OR (
                    OLD.status='revoked' AND NEW.status='active'
                    AND NEW.current_credential_revision=OLD.current_credential_revision+1
                    AND NEW.current_credential_digest!=OLD.current_credential_digest
                    AND EXISTS (
                        SELECT 1
                          FROM node_endpoint_credential_versions version
                          JOIN node_endpoint_credential_revocations terminal
                            ON terminal.credential_id=OLD.credential_id
                           AND terminal.credential_revision=
                                OLD.current_credential_revision
                           AND terminal.credential_digest=OLD.current_credential_digest
                           AND terminal.revocation_kind IN (
                                'owner_revoked','security_revoked'
                           )
                         WHERE version.credential_id=NEW.credential_id
                           AND version.credential_revision=NEW.current_credential_revision
                           AND version.credential_digest=NEW.current_credential_digest
                           AND version.agent_id=NEW.agent_id
                           AND version.owner_user_id=NEW.owner_user_id
                           AND version.install_id=NEW.install_id
                           AND version.installation_binding_digest=
                                NEW.installation_binding_digest
                           AND version.issuance_kind='account_recovery'
                           AND version.previous_credential_revision=
                                OLD.current_credential_revision
                           AND version.previous_credential_digest=
                                OLD.current_credential_digest
                           AND NEW.updated_at=version.recorded_at
                    )
                    AND NOT EXISTS (
                        SELECT 1 FROM node_endpoint_session_heads session
                         WHERE session.agent_id=OLD.agent_id
                           AND session.credential_id=OLD.credential_id
                           AND session.state='active'
                    )
                )
          )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint credential transition is not exact');
        END;
        "#,
    )?;
    Ok(())
}
