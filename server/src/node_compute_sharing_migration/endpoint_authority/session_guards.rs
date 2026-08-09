use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_session_receipt_projection
        BEFORE INSERT ON node_endpoint_session_authentication_receipts
        WHEN json_type(NEW.authentication_json)!='object'
          OR (SELECT COUNT(*) FROM json_each(NEW.authentication_json))!=23
          OR json_type(NEW.authentication_json,'$.transport')!='object'
          OR (
                SELECT COUNT(*)
                  FROM json_each(NEW.authentication_json,'$.transport')
             )!=8
          OR json_extract(NEW.authentication_json,'$.schema')
                IS NOT NEW.authentication_schema
          OR json_extract(NEW.authentication_json,'$.authentication_receipt_id')
                IS NOT NEW.authentication_receipt_id
          OR json_extract(NEW.authentication_json,'$.credential_id')
                IS NOT NEW.credential_id
          OR json_extract(NEW.authentication_json,'$.credential_revision')
                IS NOT NEW.credential_revision
          OR json_extract(NEW.authentication_json,'$.credential_digest')
                IS NOT NEW.credential_digest
          OR json_extract(NEW.authentication_json,'$.agent_id') IS NOT NEW.agent_id
          OR json_extract(NEW.authentication_json,'$.owner_user_id')
                IS NOT NEW.owner_user_id
          OR json_extract(NEW.authentication_json,'$.install_id') IS NOT NEW.install_id
          OR json_extract(NEW.authentication_json,'$.installation_binding_digest')
                IS NOT NEW.installation_binding_digest
          OR json_extract(NEW.authentication_json,'$.session_id') IS NOT NEW.session_id
          OR json_extract(NEW.authentication_json,'$.session_generation')
                IS NOT NEW.session_generation
          OR (
                NEW.previous_authentication_receipt_id IS NULL
                AND json_type(
                    NEW.authentication_json,
                    '$.previous_authentication_receipt_id'
                ) IS NOT 'null'
             )
          OR (
                NEW.previous_authentication_receipt_id IS NOT NULL
                AND json_extract(
                    NEW.authentication_json,
                    '$.previous_authentication_receipt_id'
                ) IS NOT NEW.previous_authentication_receipt_id
             )
          OR (
                NEW.previous_authentication_digest IS NULL
                AND json_type(
                    NEW.authentication_json,
                    '$.previous_authentication_digest'
                ) IS NOT 'null'
             )
          OR (
                NEW.previous_authentication_digest IS NOT NULL
                AND json_extract(
                    NEW.authentication_json,
                    '$.previous_authentication_digest'
                ) IS NOT NEW.previous_authentication_digest
             )
          OR json_extract(NEW.authentication_json,'$.server_instance_id')
                IS NOT NEW.server_instance_id
          OR json_extract(NEW.authentication_json,'$.authentication_method')
                IS NOT NEW.authentication_method
          OR json_extract(NEW.authentication_json,'$.protocol_version')
                IS NOT NEW.protocol_version
          OR json_extract(NEW.authentication_json,'$.agent_version')
                IS NOT NEW.agent_version
          OR json_extract(NEW.authentication_json,'$.capability_count')
                IS NOT NEW.capability_count
          OR json_extract(NEW.authentication_json,'$.capability_set_digest')
                IS NOT NEW.capability_set_digest
          OR json_extract(NEW.authentication_json,'$.transport.transport_scheme')
                IS NOT NEW.transport_scheme
          OR json_extract(
                NEW.authentication_json,
                '$.transport.transport_security_source'
             ) IS NOT NEW.transport_security_source
          OR json_extract(
                NEW.authentication_json,
                '$.transport.transport_security_evidence_schema'
             ) IS NOT NEW.transport_security_evidence_schema
          OR json_extract(
                NEW.authentication_json,
                '$.transport.transport_security_evidence_id'
             ) IS NOT NEW.transport_security_evidence_id
          OR json_extract(
                NEW.authentication_json,
                '$.transport.transport_security_evidence_digest'
             ) IS NOT NEW.transport_security_evidence_digest
          OR json_extract(
                NEW.authentication_json,
                '$.transport.transport_verifier_revision'
             ) IS NOT NEW.transport_verifier_revision
          OR json_extract(
                NEW.authentication_json,
                '$.transport.transport_verifier_digest'
             ) IS NOT NEW.transport_verifier_digest
          OR json_extract(
                NEW.authentication_json,
                '$.transport.transport_verified_at'
             ) IS NOT NEW.transport_verified_at
          OR json_extract(NEW.authentication_json,'$.authenticated_at')
                IS NOT NEW.authenticated_at
          OR json_extract(NEW.authentication_json,'$.expires_at') IS NOT NEW.expires_at
          OR json_extract(NEW.authentication_json,'$.recorded_at') IS NOT NEW.recorded_at
          OR json_type(NEW.authentication_json,'$.authentication_digest') IS NOT NULL
          OR json_type(NEW.authentication_json,'$.authentication_json') IS NOT NULL
          OR json_type(NEW.authentication_json,'$.canonicalization') IS NOT NULL
          OR json_type(NEW.authentication_json,'$.digest_algorithm') IS NOT NULL
          OR json_type(NEW.authentication_json,'$.capability_set_json') IS NOT NULL
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint session receipt projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_session_capability_projection
        BEFORE INSERT ON node_endpoint_session_authentication_receipts
        WHEN json_array_length(NEW.capability_set_json)!=NEW.capability_count
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.capability_set_json) capability
                 WHERE capability.type!='text'
                    OR capability.value!=trim(capability.value)
                    OR length(CAST(capability.value AS BLOB)) NOT BETWEEN 1 AND 160
             )
          OR EXISTS (
                SELECT 1
                  FROM json_each(NEW.capability_set_json) earlier
                  JOIN json_each(NEW.capability_set_json) later
                    ON CAST(earlier.key AS INTEGER)<CAST(later.key AS INTEGER)
                 WHERE CAST(earlier.value AS TEXT)>=CAST(later.value AS TEXT)
             )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint capability set is not canonical');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_session_receipt_source
        BEFORE INSERT ON node_endpoint_session_authentication_receipts
        WHEN NEW.transport_verified_at>NEW.authenticated_at
          OR NEW.authenticated_at>=NEW.recorded_at
          OR NEW.recorded_at>=NEW.expires_at
          OR unixepoch(NEW.expires_at)-unixepoch(NEW.authenticated_at)!=900
          OR substr(NEW.expires_at,20,10)!=substr(NEW.authenticated_at,20,10)
          OR NOT EXISTS (
                SELECT 1
                  FROM node_endpoint_credentials root
                  JOIN node_endpoint_credential_versions version
                    ON version.credential_id=root.credential_id
                   AND version.credential_revision=root.current_credential_revision
                   AND version.credential_digest=root.current_credential_digest
                  JOIN node_credentials legacy ON legacy.agent_id=root.agent_id
                 WHERE root.credential_id=NEW.credential_id
                   AND root.current_credential_revision=NEW.credential_revision
                   AND root.current_credential_digest=NEW.credential_digest
                   AND root.agent_id=NEW.agent_id
                   AND root.owner_user_id=NEW.owner_user_id
                   AND root.install_id=NEW.install_id
                   AND root.installation_binding_digest=NEW.installation_binding_digest
                   AND root.status='active'
                   AND root.updated_at<=NEW.authenticated_at
                   AND version.agent_id=NEW.agent_id
                   AND version.owner_user_id=NEW.owner_user_id
                   AND version.install_id=NEW.install_id
                   AND version.installation_binding_digest=
                        NEW.installation_binding_digest
                   AND version.recorded_at<=NEW.authenticated_at
                   AND legacy.owner_user_id=NEW.owner_user_id
                   AND legacy.install_id=NEW.install_id
                   AND legacy.install_id=trim(legacy.install_id)
                   AND NOT EXISTS (
                        SELECT 1 FROM node_endpoint_credential_revocations revoked
                         WHERE revoked.credential_id=NEW.credential_id
                           AND revoked.credential_revision=NEW.credential_revision
                           AND revoked.credential_digest=NEW.credential_digest
                   )
             )
          OR (
                NEW.session_generation=1
                AND EXISTS (
                    SELECT 1 FROM node_endpoint_session_heads head
                     WHERE head.agent_id=NEW.agent_id
                )
             )
          OR (
                NEW.session_generation>1
                AND NOT EXISTS (
                    SELECT 1
                      FROM node_endpoint_session_heads head
                      JOIN node_endpoint_session_authentication_receipts previous
                        ON previous.authentication_receipt_id=
                            head.authentication_receipt_id
                       AND previous.authentication_digest=head.authentication_digest
                     WHERE head.agent_id=NEW.agent_id
                       AND head.session_generation=NEW.session_generation-1
                       AND head.authentication_receipt_id=
                            NEW.previous_authentication_receipt_id
                       AND head.authentication_digest=NEW.previous_authentication_digest
                       AND previous.agent_id=NEW.agent_id
                       AND previous.session_generation=NEW.session_generation-1
                       AND head.updated_at<=NEW.authenticated_at
                )
             )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint session receipt lacks current authority');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_session_head_initial_exact
        BEFORE INSERT ON node_endpoint_session_heads
        WHEN NEW.state!='active'
          OR NEW.session_generation!=1
          OR NOT EXISTS (
                SELECT 1
                  FROM node_endpoint_session_authentication_receipts receipt
                  JOIN node_endpoint_credentials root
                    ON root.credential_id=receipt.credential_id
                   AND root.current_credential_revision=receipt.credential_revision
                   AND root.current_credential_digest=receipt.credential_digest
                 WHERE receipt.authentication_receipt_id=
                        NEW.authentication_receipt_id
                   AND receipt.authentication_digest=NEW.authentication_digest
                   AND receipt.agent_id=NEW.agent_id
                   AND receipt.credential_id=NEW.credential_id
                   AND receipt.credential_revision=NEW.credential_revision
                   AND receipt.credential_digest=NEW.credential_digest
                   AND receipt.session_id=NEW.session_id
                   AND receipt.session_generation=NEW.session_generation
                   AND receipt.server_instance_id=NEW.server_instance_id
                   AND receipt.authenticated_at=NEW.authenticated_at
                   AND receipt.expires_at=NEW.expires_at
                   AND receipt.recorded_at=NEW.created_at
                   AND receipt.recorded_at=NEW.updated_at
                   AND root.agent_id=NEW.agent_id
                   AND root.status='active'
                   AND NOT EXISTS (
                        SELECT 1 FROM node_endpoint_credential_revocations revoked
                         WHERE revoked.credential_id=NEW.credential_id
                           AND revoked.credential_revision=NEW.credential_revision
                           AND revoked.credential_digest=NEW.credential_digest
                   )
             )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint session head lacks exact initial receipt');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_session_head_linear_transition
        BEFORE UPDATE ON node_endpoint_session_heads
        WHEN NEW.agent_id!=OLD.agent_id
          OR NEW.updated_at<=OLD.updated_at
          OR NOT (
                (
                    OLD.state='active'
                    AND NEW.state IN (
                        'closed','stale','credential_rotated','credential_revoked'
                    )
                    AND NEW.credential_id=OLD.credential_id
                    AND NEW.credential_revision=OLD.credential_revision
                    AND NEW.credential_digest=OLD.credential_digest
                    AND NEW.authentication_receipt_id=OLD.authentication_receipt_id
                    AND NEW.authentication_digest=OLD.authentication_digest
                    AND NEW.session_id=OLD.session_id
                    AND NEW.session_generation=OLD.session_generation
                    AND NEW.server_instance_id=OLD.server_instance_id
                    AND NEW.authenticated_at=OLD.authenticated_at
                    AND NEW.expires_at=OLD.expires_at
                    AND NEW.created_at=OLD.created_at
                    AND NEW.closed_at=NEW.updated_at
                )
                OR (
                    OLD.state!='active'
                    AND NEW.state='active'
                    AND NEW.session_generation=OLD.session_generation+1
                    AND NEW.closed_at IS NULL
                    AND NEW.close_reason_code IS NULL
                    AND EXISTS (
                        SELECT 1
                          FROM node_endpoint_session_authentication_receipts receipt
                          JOIN node_endpoint_credentials root
                            ON root.credential_id=receipt.credential_id
                           AND root.current_credential_revision=
                                receipt.credential_revision
                           AND root.current_credential_digest=receipt.credential_digest
                         WHERE receipt.authentication_receipt_id=
                                NEW.authentication_receipt_id
                           AND receipt.authentication_digest=NEW.authentication_digest
                           AND receipt.agent_id=NEW.agent_id
                           AND receipt.credential_id=NEW.credential_id
                           AND receipt.credential_revision=NEW.credential_revision
                           AND receipt.credential_digest=NEW.credential_digest
                           AND receipt.session_id=NEW.session_id
                           AND receipt.session_generation=NEW.session_generation
                           AND receipt.previous_authentication_receipt_id=
                                OLD.authentication_receipt_id
                           AND receipt.previous_authentication_digest=
                                OLD.authentication_digest
                           AND receipt.server_instance_id=NEW.server_instance_id
                           AND receipt.authenticated_at=NEW.authenticated_at
                           AND receipt.expires_at=NEW.expires_at
                           AND receipt.recorded_at=NEW.created_at
                           AND receipt.recorded_at=NEW.updated_at
                           AND root.agent_id=NEW.agent_id
                           AND root.status='active'
                           AND NOT EXISTS (
                                SELECT 1
                                  FROM node_endpoint_credential_revocations revoked
                                 WHERE revoked.credential_id=NEW.credential_id
                                   AND revoked.credential_revision=
                                        NEW.credential_revision
                                   AND revoked.credential_digest=NEW.credential_digest
                           )
                    )
                )
             )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint session head transition is not exact');
        END;
        "#,
    )?;
    Ok(())
}
