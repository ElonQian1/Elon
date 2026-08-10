use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_planning_events_no_replace
        BEFORE INSERT ON node_compute_plugin_endpoint_planning_chain_events_v1
        WHEN EXISTS (
          SELECT 1 FROM node_compute_plugin_endpoint_planning_chain_events_v1 stored
           WHERE stored.event_id=NEW.event_id
              OR stored.message_digest=NEW.message_digest
              OR (stored.bootstrap_id=NEW.bootstrap_id
                  AND stored.message_sequence=NEW.message_sequence)
              OR (NEW.previous_event_id IS NOT NULL
                  AND stored.bootstrap_id=NEW.bootstrap_id
                  AND stored.previous_event_id=NEW.previous_event_id)
              OR (NEW.message_sequence=1 AND stored.message_sequence=1
                  AND stored.authentication_receipt_id=NEW.authentication_receipt_id)
        ) BEGIN
          SELECT RAISE(ABORT, 'endpoint Planning events cannot be replaced');
        END;
        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_planning_events_no_update
        BEFORE UPDATE ON node_compute_plugin_endpoint_planning_chain_events_v1 BEGIN
          SELECT RAISE(ABORT, 'endpoint Planning events are append-only');
        END;
        CREATE TRIGGER IF NOT EXISTS trg_node_endpoint_planning_events_no_delete
        BEFORE DELETE ON node_compute_plugin_endpoint_planning_chain_events_v1 BEGIN
          SELECT RAISE(ABORT, 'endpoint Planning events are append-only');
        END;
        "#,
    )?;
    Ok(())
}
