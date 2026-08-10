use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_credential_version_recent_reauth_basis
        BEFORE INSERT ON node_endpoint_credential_versions
        WHEN NEW.owner_authorization_basis_kind='recent_reauthentication'
          AND NOT EXISTS (
            SELECT 1
              FROM node_endpoint_owner_reauthentication_consumptions consumed
              JOIN node_endpoint_owner_reauthentication_receipts source
                ON source.reauthentication_receipt_id=
                    consumed.reauthentication_receipt_id
               AND source.reauthentication_digest=consumed.reauthentication_digest
             WHERE consumed.reauthentication_receipt_id=
                    NEW.owner_authorization_basis_id
               AND consumed.reauthentication_digest=
                    NEW.owner_authorization_basis_digest
               AND consumed.owner_user_id=NEW.owner_user_id
               AND consumed.credential_mutation_request_id=NEW.issuance_request_id
               AND consumed.current_credential_id=NEW.credential_id
               AND consumed.current_credential_revision=NEW.credential_revision
               AND consumed.current_credential_digest=NEW.credential_digest
               AND consumed.current_credential_status='active'
               AND consumed.issued_credential_id=NEW.credential_id
               AND consumed.issued_credential_revision=NEW.credential_revision
               AND consumed.issued_credential_digest=NEW.credential_digest
               AND consumed.consumed_at=NEW.issued_at
               AND consumed.recorded_at=NEW.recorded_at
               AND source.owner_user_id=NEW.owner_user_id
               AND source.agent_id=NEW.agent_id
               AND source.install_id=NEW.install_id
               AND source.credential_mutation_request_id=NEW.issuance_request_id
               AND source.credential_mutation_request_digest=
                    consumed.credential_mutation_request_digest
               AND source.authorization_target_digest=
                    consumed.authorization_target_digest
               AND NEW.issued_by_user_id=NEW.owner_user_id
               AND (
                    (NEW.issuance_kind='initial_registration'
                        AND consumed.authorization_action='initial_registration'
                        AND source.expected_credential_id IS NULL
                        AND NEW.credential_revision=1
                        AND NEW.previous_credential_revision IS NULL
                        AND NEW.previous_credential_digest IS NULL)
                    OR (NEW.issuance_kind='credential_rotation'
                        AND consumed.authorization_action='credential_rotation'
                        AND NEW.previous_credential_revision=
                            source.expected_credential_revision
                        AND NEW.previous_credential_digest=
                            source.expected_credential_digest
                        AND NEW.credential_id=source.expected_credential_id)
                    OR (NEW.issuance_kind='account_recovery'
                        AND consumed.authorization_action='account_recovery'
                        AND NEW.previous_credential_revision=
                            source.expected_credential_revision
                        AND NEW.previous_credential_digest=
                            source.expected_credential_digest
                        AND NEW.credential_id=source.expected_credential_id)
               )
        )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint credential lacks consumed recent reauthentication basis');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_credential_revocation_recent_reauth_basis
        BEFORE INSERT ON node_endpoint_credential_revocations
        WHEN NEW.owner_authorization_basis_kind='recent_reauthentication'
          AND NOT EXISTS (
            SELECT 1
              FROM node_endpoint_owner_reauthentication_consumptions consumed
              JOIN node_endpoint_owner_reauthentication_receipts source
                ON source.reauthentication_receipt_id=
                    consumed.reauthentication_receipt_id
               AND source.reauthentication_digest=consumed.reauthentication_digest
             WHERE consumed.reauthentication_receipt_id=
                    NEW.owner_authorization_basis_id
               AND consumed.reauthentication_digest=
                    NEW.owner_authorization_basis_digest
               AND consumed.owner_user_id=NEW.owner_user_id
               AND consumed.credential_mutation_request_id=NEW.mutation_request_id
               AND consumed.revocation_id=NEW.revocation_id
               AND consumed.revocation_digest=NEW.revocation_digest
               AND consumed.consumed_at=NEW.revoked_at
               AND consumed.recorded_at=NEW.recorded_at
               AND source.owner_user_id=NEW.owner_user_id
               AND source.agent_id=NEW.agent_id
               AND source.expected_credential_id=NEW.credential_id
               AND source.expected_credential_revision=NEW.credential_revision
               AND source.expected_credential_digest=NEW.credential_digest
               AND source.credential_mutation_request_id=NEW.mutation_request_id
               AND source.credential_mutation_request_digest=
                    consumed.credential_mutation_request_digest
               AND source.authorization_target_digest=
                    consumed.authorization_target_digest
               AND NEW.revoked_by_user_id=NEW.owner_user_id
               AND (
                    (NEW.revocation_kind='rotated'
                        AND consumed.authorization_action='credential_rotation'
                        AND consumed.current_credential_id=NEW.credential_id
                        AND consumed.current_credential_revision=
                            NEW.credential_revision+1
                        AND consumed.current_credential_status='active'
                        AND consumed.issued_credential_id=
                            consumed.current_credential_id
                        AND consumed.issued_credential_revision=
                            consumed.current_credential_revision
                        AND consumed.issued_credential_digest=
                            consumed.current_credential_digest)
                    OR (NEW.revocation_kind='recovered'
                        AND consumed.authorization_action='account_recovery'
                        AND consumed.current_credential_id=NEW.credential_id
                        AND consumed.current_credential_revision=
                            NEW.credential_revision+1
                        AND consumed.current_credential_status='active'
                        AND consumed.issued_credential_id=
                            consumed.current_credential_id
                        AND consumed.issued_credential_revision=
                            consumed.current_credential_revision
                        AND consumed.issued_credential_digest=
                            consumed.current_credential_digest)
                    OR (NEW.revocation_kind='owner_revoked'
                        AND consumed.authorization_action='owner_revocation'
                        AND consumed.current_credential_id=NEW.credential_id
                        AND consumed.current_credential_revision=NEW.credential_revision
                        AND consumed.current_credential_digest=NEW.credential_digest
                        AND consumed.current_credential_status='revoked'
                        AND consumed.issued_credential_id IS NULL)
               )
        )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint revocation lacks consumed recent reauthentication basis');
        END;
        "#,
    )?;
    Ok(())
}
