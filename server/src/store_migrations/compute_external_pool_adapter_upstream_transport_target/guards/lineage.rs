use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_upstream_transport_target_lineage
        BEFORE INSERT ON compute_external_pool_adapter_upstream_transport_targets
        WHEN NOT (
          (NEW.sequence=1
           AND NEW.predecessor_target_id IS NULL
           AND NEW.predecessor_target_digest IS NULL
           AND NOT EXISTS (
             SELECT 1 FROM compute_external_pool_adapter_upstream_transport_targets existing
              WHERE existing.provider_binding_id=NEW.provider_binding_id))
          OR
          (NEW.sequence>1
           AND EXISTS (
             SELECT 1 FROM compute_external_pool_adapter_upstream_transport_targets predecessor
              WHERE predecessor.target_id=NEW.predecessor_target_id
                AND predecessor.target_digest=NEW.predecessor_target_digest
                AND predecessor.provider_binding_id=NEW.provider_binding_id
                AND predecessor.provider_binding_digest=NEW.provider_binding_digest
                AND predecessor.sequence=NEW.sequence-1
                AND predecessor.recorded_at<=NEW.recorded_at
                AND NOT EXISTS (
                  SELECT 1 FROM compute_external_pool_adapter_upstream_transport_targets successor
                   WHERE successor.predecessor_target_id=predecessor.target_id)))
        )
        BEGIN SELECT RAISE(ABORT,'V258 target requires exact structural predecessor head'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_upstream_transport_target_revocation_lineage
        BEFORE INSERT ON compute_external_pool_adapter_upstream_transport_target_revocations
        WHEN NOT EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_upstream_transport_targets target
           WHERE target.target_id=NEW.target_id
             AND target.target_digest=NEW.target_digest
             AND target.profile_id=NEW.profile_id
             AND target.profile_digest=NEW.profile_digest
             AND target.provider_binding_id=NEW.provider_binding_id
             AND target.provider_binding_digest=NEW.provider_binding_digest
             AND target.provider_id=NEW.provider_id
             AND target.recorded_at<=NEW.revoked_at
             AND NOT EXISTS (
               SELECT 1 FROM compute_external_pool_adapter_upstream_transport_targets successor
                WHERE successor.predecessor_target_id=target.target_id)
             AND NOT EXISTS (
               SELECT 1 FROM compute_external_pool_adapter_upstream_transport_target_revocations prior
                WHERE prior.target_id=target.target_id))
        BEGIN SELECT RAISE(ABORT,'V258 revocation requires exact current unrevoked target head'); END;
        "#,
    )?;
    Ok(())
}
