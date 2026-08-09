use anyhow::Result;
use rusqlite::Connection;

/// SQLite may implement `OR REPLACE` by deleting the conflicting row without firing its DELETE
/// trigger. Reject every primary-key and unique-key collision before replacement can occur.
pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_credentials_no_replace
        BEFORE INSERT ON node_endpoint_credentials
        WHEN EXISTS (
            SELECT 1 FROM node_endpoint_credentials stored
             WHERE stored.credential_id=NEW.credential_id
                OR stored.agent_id=NEW.agent_id
                OR (stored.owner_user_id=NEW.owner_user_id
                    AND stored.install_id=NEW.install_id)
        )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint credential replacement is forbidden');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_credential_versions_no_replace
        BEFORE INSERT ON node_endpoint_credential_versions
        WHEN EXISTS (
            SELECT 1 FROM node_endpoint_credential_versions stored
             WHERE (stored.credential_id=NEW.credential_id
                    AND stored.credential_revision=NEW.credential_revision)
                OR stored.credential_digest=NEW.credential_digest
                OR stored.secret_hash=NEW.secret_hash
                OR stored.secret_verifier_digest=NEW.secret_verifier_digest
                OR (stored.credential_id=NEW.credential_id
                    AND stored.issuance_request_id=NEW.issuance_request_id)
                OR (stored.credential_id=NEW.credential_id
                    AND stored.credential_revision=NEW.credential_revision
                    AND stored.credential_digest=NEW.credential_digest)
        )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint credential version replacement is forbidden');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_credential_revocations_no_replace
        BEFORE INSERT ON node_endpoint_credential_revocations
        WHEN EXISTS (
            SELECT 1 FROM node_endpoint_credential_revocations stored
             WHERE stored.revocation_id=NEW.revocation_id
                OR stored.revocation_digest=NEW.revocation_digest
                OR (stored.credential_id=NEW.credential_id
                    AND stored.credential_revision=NEW.credential_revision)
                OR (stored.credential_id=NEW.credential_id
                    AND stored.mutation_request_id=NEW.mutation_request_id)
        )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint credential revocation replacement is forbidden');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_session_receipts_no_replace
        BEFORE INSERT ON node_endpoint_session_authentication_receipts
        WHEN EXISTS (
            SELECT 1 FROM node_endpoint_session_authentication_receipts stored
             WHERE stored.authentication_receipt_id=NEW.authentication_receipt_id
                OR stored.authentication_digest=NEW.authentication_digest
                OR stored.session_id=NEW.session_id
                OR (stored.agent_id=NEW.agent_id
                    AND stored.session_generation=NEW.session_generation)
                OR (stored.authentication_receipt_id=NEW.authentication_receipt_id
                    AND stored.authentication_digest=NEW.authentication_digest)
                OR stored.transport_security_evidence_id=
                    NEW.transport_security_evidence_id
                OR stored.transport_security_evidence_digest=
                    NEW.transport_security_evidence_digest
        )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint session receipt replacement is forbidden');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_session_heads_no_replace
        BEFORE INSERT ON node_endpoint_session_heads
        WHEN EXISTS (
            SELECT 1 FROM node_endpoint_session_heads stored
             WHERE stored.agent_id=NEW.agent_id
                OR stored.authentication_receipt_id=NEW.authentication_receipt_id
                OR stored.authentication_digest=NEW.authentication_digest
                OR stored.session_id=NEW.session_id
        )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint session head replacement is forbidden');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_session_heads_update_no_replace
        BEFORE UPDATE ON node_endpoint_session_heads
        WHEN EXISTS (
            SELECT 1 FROM node_endpoint_session_heads peer
             WHERE peer.agent_id!=OLD.agent_id
               AND (
                    peer.authentication_receipt_id=NEW.authentication_receipt_id
                    OR peer.authentication_digest=NEW.authentication_digest
                    OR peer.session_id=NEW.session_id
               )
        )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint session head update replacement is forbidden');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_credential_versions_immutable
        BEFORE UPDATE ON node_endpoint_credential_versions
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint credential versions are immutable');
        END;
        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_credential_versions_append_only
        BEFORE DELETE ON node_endpoint_credential_versions
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint credential versions are append-only');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_credential_revocations_immutable
        BEFORE UPDATE ON node_endpoint_credential_revocations
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint credential revocations are immutable');
        END;
        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_credential_revocations_append_only
        BEFORE DELETE ON node_endpoint_credential_revocations
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint credential revocations are append-only');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_session_receipts_immutable
        BEFORE UPDATE ON node_endpoint_session_authentication_receipts
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint session receipts are immutable');
        END;
        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_session_receipts_append_only
        BEFORE DELETE ON node_endpoint_session_authentication_receipts
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint session receipts are append-only');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_credentials_delete_forbidden
        BEFORE DELETE ON node_endpoint_credentials
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint credential roots cannot be deleted');
        END;
        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_session_heads_delete_forbidden
        BEFORE DELETE ON node_endpoint_session_heads
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint session heads cannot be deleted');
        END;
        "#,
    )?;
    Ok(())
}
