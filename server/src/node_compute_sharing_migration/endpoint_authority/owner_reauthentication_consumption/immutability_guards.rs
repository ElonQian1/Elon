use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_owner_reauth_consumption_no_replace
        BEFORE INSERT ON node_endpoint_owner_reauthentication_consumptions
        WHEN EXISTS (
            SELECT 1
              FROM node_endpoint_owner_reauthentication_consumptions stored
             WHERE stored.consumption_id=NEW.consumption_id
                OR stored.consumption_digest=NEW.consumption_digest
                OR stored.reauthentication_receipt_id=NEW.reauthentication_receipt_id
                OR (stored.owner_user_id=NEW.owner_user_id
                    AND stored.credential_mutation_request_id=
                        NEW.credential_mutation_request_id)
        )
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint owner reauthentication consumption replacement is forbidden');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_owner_reauth_consumption_immutable
        BEFORE UPDATE ON node_endpoint_owner_reauthentication_consumptions
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint owner reauthentication consumptions are immutable');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_owner_reauth_consumption_append_only
        BEFORE DELETE ON node_endpoint_owner_reauthentication_consumptions
        BEGIN
            SELECT RAISE(ABORT, 'node endpoint owner reauthentication consumptions are append-only');
        END;
        "#,
    )?;
    Ok(())
}
