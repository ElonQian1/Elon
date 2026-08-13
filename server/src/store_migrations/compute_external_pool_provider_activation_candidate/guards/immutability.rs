use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    for (name, table, identity) in TABLES {
        conn.execute_batch(&format!(
            "CREATE TRIGGER IF NOT EXISTS {name}_no_update
             BEFORE UPDATE ON {table}
             BEGIN SELECT RAISE(ABORT,'V254 activation authority is immutable'); END;
             CREATE TRIGGER IF NOT EXISTS {name}_no_delete
             BEFORE DELETE ON {table}
             BEGIN SELECT RAISE(ABORT,'V254 activation authority is append-only'); END;
             CREATE TRIGGER IF NOT EXISTS {name}_no_replace
             BEFORE INSERT ON {table}
             WHEN EXISTS(SELECT 1 FROM {table} old WHERE {identity})
             BEGIN SELECT RAISE(ABORT,'V254 activation authority cannot replace immutable history'); END;"
        ))?;
    }
    Ok(())
}

const TABLES: &[(&str, &str, &str)] = &[
    (
        "external_pool_provider_activation_delegation",
        "compute_external_pool_provider_activation_delegations",
        "old.delegation_id=NEW.delegation_id OR old.delegation_digest=NEW.delegation_digest OR (old.provider_binding_id=NEW.provider_binding_id AND old.sequence=NEW.sequence) OR old.predecessor_delegation_id=NEW.predecessor_delegation_id OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key)",
    ),
    (
        "external_pool_provider_activation_candidate",
        "compute_external_pool_provider_activation_candidates",
        "old.candidate_id=NEW.candidate_id OR old.candidate_digest=NEW.candidate_digest OR old.delegation_id=NEW.delegation_id OR (old.provider_binding_id=NEW.provider_binding_id AND old.sequence=NEW.sequence) OR old.predecessor_candidate_id=NEW.predecessor_candidate_id",
    ),
    (
        "external_pool_provider_activation_delegation_revocation",
        "compute_external_pool_provider_activation_delegation_revocations",
        "old.revocation_id=NEW.revocation_id OR old.revocation_digest=NEW.revocation_digest OR old.delegation_id=NEW.delegation_id OR old.candidate_id=NEW.candidate_id OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key)",
    ),
];
