use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_owner_reauth_consumption_source
        BEFORE INSERT ON node_endpoint_owner_reauthentication_consumptions
        WHEN NOT EXISTS (
            SELECT 1
              FROM node_endpoint_owner_reauthentication_receipts source
             WHERE source.reauthentication_receipt_id=NEW.reauthentication_receipt_id
               AND source.reauthentication_digest=NEW.reauthentication_digest
               AND source.owner_user_id=NEW.owner_user_id
               AND source.authorization_action=NEW.authorization_action
               AND source.credential_mutation_request_id=
                    NEW.credential_mutation_request_id
               AND source.credential_mutation_request_digest=
                    NEW.credential_mutation_request_digest
               AND source.authorization_target_digest=NEW.authorization_target_digest
               AND source.recorded_at<=NEW.consumed_at
               AND source.reauthenticated_at<=NEW.consumed_at
               AND NEW.consumed_at<=NEW.recorded_at
               AND NEW.recorded_at<source.expires_at
               AND (
                    (source.authorization_action='initial_registration'
                        AND source.expected_credential_id IS NULL
                        AND NOT EXISTS (
                            SELECT 1 FROM node_endpoint_credentials root
                             WHERE root.agent_id=source.agent_id
                                OR (root.owner_user_id=source.owner_user_id
                                    AND root.install_id=source.install_id)
                        )
                        AND NEW.current_credential_revision=1
                        AND NEW.current_credential_status='active'
                        AND NEW.issued_credential_id=NEW.current_credential_id
                        AND NEW.issued_credential_revision=1
                        AND NEW.issued_credential_digest=NEW.current_credential_digest
                        AND NEW.revocation_id IS NULL
                        AND NOT EXISTS (
                            SELECT 1 FROM node_endpoint_credential_versions issued
                             WHERE issued.credential_id=NEW.issued_credential_id
                               AND issued.credential_revision=
                                    NEW.issued_credential_revision
                        ))
                    OR (source.authorization_action='credential_rotation'
                        AND EXISTS (
                            SELECT 1 FROM node_endpoint_credentials root
                             WHERE root.credential_id=source.expected_credential_id
                               AND root.agent_id=source.agent_id
                               AND root.owner_user_id=source.owner_user_id
                               AND root.install_id=source.install_id
                               AND root.current_credential_revision=
                                    source.expected_credential_revision
                               AND root.current_credential_digest=
                                    source.expected_credential_digest
                               AND root.status='active'
                        )
                        AND source.expected_credential_id=NEW.current_credential_id
                        AND source.expected_credential_revision+1=
                            NEW.current_credential_revision
                        AND source.expected_credential_digest!=NEW.current_credential_digest
                        AND NEW.current_credential_status='active'
                        AND NEW.issued_credential_id=NEW.current_credential_id
                        AND NEW.issued_credential_revision=NEW.current_credential_revision
                        AND NEW.issued_credential_digest=NEW.current_credential_digest
                        AND NEW.revocation_id IS NOT NULL
                        AND NOT EXISTS (
                            SELECT 1 FROM node_endpoint_credential_versions issued
                             WHERE issued.credential_id=NEW.issued_credential_id
                               AND issued.credential_revision=
                                    NEW.issued_credential_revision
                        )
                        AND NOT EXISTS (
                            SELECT 1 FROM node_endpoint_credential_revocations revoked
                             WHERE revoked.credential_id=source.expected_credential_id
                               AND revoked.credential_revision=
                                    source.expected_credential_revision
                        ))
                    OR (source.authorization_action='account_recovery'
                        AND source.expected_credential_id=NEW.current_credential_id
                        AND source.expected_credential_revision+1=
                            NEW.current_credential_revision
                        AND source.expected_credential_digest!=NEW.current_credential_digest
                        AND NEW.current_credential_status='active'
                        AND NEW.issued_credential_id=NEW.current_credential_id
                        AND NEW.issued_credential_revision=NEW.current_credential_revision
                        AND NEW.issued_credential_digest=NEW.current_credential_digest
                        AND NEW.revocation_id IS NOT NULL
                        AND NOT EXISTS (
                            SELECT 1 FROM node_endpoint_credential_versions issued
                             WHERE issued.credential_id=NEW.issued_credential_id
                               AND issued.credential_revision=
                                    NEW.issued_credential_revision
                        )
                        AND EXISTS (
                            SELECT 1 FROM node_endpoint_credentials root
                             WHERE root.credential_id=source.expected_credential_id
                               AND root.agent_id=source.agent_id
                               AND root.owner_user_id=source.owner_user_id
                               AND root.install_id=source.install_id
                               AND root.current_credential_revision=
                                    source.expected_credential_revision
                               AND root.current_credential_digest=
                                    source.expected_credential_digest
                               AND (
                                    (root.status='active'
                                        AND NOT EXISTS (
                                            SELECT 1
                                              FROM node_endpoint_credential_revocations revoked
                                             WHERE revoked.credential_id=
                                                    source.expected_credential_id
                                               AND revoked.credential_revision=
                                                    source.expected_credential_revision
                                        ))
                                    OR (root.status='revoked'
                                        AND EXISTS (
                                            SELECT 1
                                              FROM node_endpoint_credential_revocations terminal
                                             WHERE terminal.revocation_id=NEW.revocation_id
                                               AND terminal.revocation_digest=
                                                    NEW.revocation_digest
                                               AND terminal.credential_id=
                                                    source.expected_credential_id
                                               AND terminal.credential_revision=
                                                    source.expected_credential_revision
                                               AND terminal.credential_digest=
                                                    source.expected_credential_digest
                                               AND terminal.revocation_kind IN (
                                                    'owner_revoked','security_revoked'
                                               )
                                        ))
                               )
                        ))
                    OR (source.authorization_action='owner_revocation'
                        AND EXISTS (
                            SELECT 1 FROM node_endpoint_credentials root
                             WHERE root.credential_id=source.expected_credential_id
                               AND root.agent_id=source.agent_id
                               AND root.owner_user_id=source.owner_user_id
                               AND root.install_id=source.install_id
                               AND root.current_credential_revision=
                                    source.expected_credential_revision
                               AND root.current_credential_digest=
                                    source.expected_credential_digest
                               AND root.status='active'
                        )
                        AND source.expected_credential_id=NEW.current_credential_id
                        AND source.expected_credential_revision=
                            NEW.current_credential_revision
                        AND source.expected_credential_digest=NEW.current_credential_digest
                        AND NEW.current_credential_status='revoked'
                        AND NEW.issued_credential_id IS NULL
                        AND NEW.revocation_id IS NOT NULL
                        AND NOT EXISTS (
                            SELECT 1 FROM node_endpoint_credential_revocations revoked
                             WHERE revoked.credential_id=source.expected_credential_id
                               AND revoked.credential_revision=
                                    source.expected_credential_revision
                        ))
               )
        )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint owner reauthentication consumption source mismatch');
        END;
        "#,
    )?;
    Ok(())
}
