use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    for (prefix, table, identity) in [
        (
            "external_pool_adapter_runtime_launch_profile",
            "compute_external_pool_adapter_runtime_launch_profiles",
            "old.profile_id=NEW.profile_id OR old.profile_digest=NEW.profile_digest OR (old.provider_binding_id=NEW.provider_binding_id AND old.sequence=NEW.sequence) OR old.predecessor_profile_id=NEW.predecessor_profile_id OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key)",
        ),
        (
            "external_pool_adapter_runtime_launch_profile_revocation",
            "compute_external_pool_adapter_runtime_launch_profile_revocations",
            "old.revocation_id=NEW.revocation_id OR old.revocation_digest=NEW.revocation_digest OR old.profile_id=NEW.profile_id OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key)",
        ),
    ] {
        conn.execute_batch(&format!(
            "CREATE TRIGGER IF NOT EXISTS {prefix}_no_update BEFORE UPDATE ON {table}
             BEGIN SELECT RAISE(ABORT,'V255 runtime launch profile authority is immutable'); END;
             CREATE TRIGGER IF NOT EXISTS {prefix}_no_delete BEFORE DELETE ON {table}
             BEGIN SELECT RAISE(ABORT,'V255 runtime launch profile authority is append-only'); END;
             CREATE TRIGGER IF NOT EXISTS {prefix}_no_replace BEFORE INSERT ON {table}
             WHEN EXISTS(SELECT 1 FROM {table} old WHERE {identity})
             BEGIN SELECT RAISE(ABORT,'V255 runtime launch profile authority cannot replace immutable history'); END;"
        ))?;
    }
    Ok(())
}
