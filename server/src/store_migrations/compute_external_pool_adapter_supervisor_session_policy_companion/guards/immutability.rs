use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    for (prefix, table, identity) in [
        (
            "external_pool_adapter_supervisor_session_policy_companion",
            "compute_external_pool_adapter_supervisor_session_policy_companions",
            "old.companion_id=NEW.companion_id OR old.companion_digest=NEW.companion_digest OR (old.provider_binding_id=NEW.provider_binding_id AND old.sequence=NEW.sequence) OR old.predecessor_companion_id=NEW.predecessor_companion_id OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key)",
        ),
        (
            "external_pool_adapter_supervisor_session_policy_companion_revocation",
            "compute_external_pool_adapter_supervisor_session_policy_companion_revocations",
            "old.revocation_id=NEW.revocation_id OR old.revocation_digest=NEW.revocation_digest OR old.companion_id=NEW.companion_id OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key)",
        ),
    ] {
        conn.execute_batch(&format!(
            "CREATE TRIGGER IF NOT EXISTS {prefix}_no_update BEFORE UPDATE ON {table}
             BEGIN SELECT RAISE(ABORT,'V259 supervisor/session companion authority is immutable'); END;
             CREATE TRIGGER IF NOT EXISTS {prefix}_no_delete BEFORE DELETE ON {table}
             BEGIN SELECT RAISE(ABORT,'V259 supervisor/session companion authority is append-only'); END;
             CREATE TRIGGER IF NOT EXISTS {prefix}_no_replace BEFORE INSERT ON {table}
             WHEN EXISTS(SELECT 1 FROM {table} old WHERE {identity})
             BEGIN SELECT RAISE(ABORT,'V259 supervisor/session companion authority cannot replace immutable history'); END;"
        ))?;
    }
    Ok(())
}
