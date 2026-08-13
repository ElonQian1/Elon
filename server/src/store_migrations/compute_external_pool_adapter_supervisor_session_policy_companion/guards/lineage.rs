use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_supervisor_session_policy_companion_lineage
        BEFORE INSERT ON compute_external_pool_adapter_supervisor_session_policy_companions
        WHEN NOT (
          (NEW.sequence=1
           AND NEW.predecessor_companion_id IS NULL
           AND NEW.predecessor_companion_digest IS NULL
           AND NOT EXISTS (
             SELECT 1 FROM compute_external_pool_adapter_supervisor_session_policy_companions existing
              WHERE existing.provider_binding_id=NEW.provider_binding_id))
          OR
          (NEW.sequence>1
           AND EXISTS (
             SELECT 1 FROM compute_external_pool_adapter_supervisor_session_policy_companions predecessor
              WHERE predecessor.companion_id=NEW.predecessor_companion_id
                AND predecessor.companion_digest=NEW.predecessor_companion_digest
                AND predecessor.provider_binding_id=NEW.provider_binding_id
                AND predecessor.provider_binding_digest=NEW.provider_binding_digest
                AND predecessor.sequence=NEW.sequence-1
                AND predecessor.recorded_at<=NEW.recorded_at
                AND NOT EXISTS (
                  SELECT 1 FROM compute_external_pool_adapter_supervisor_session_policy_companions successor
                   WHERE successor.predecessor_companion_id=predecessor.companion_id)))
        )
        BEGIN SELECT RAISE(ABORT,'V259 companion requires exact structural predecessor head'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_supervisor_session_policy_companion_revocation_lineage
        BEFORE INSERT ON compute_external_pool_adapter_supervisor_session_policy_companion_revocations
        WHEN NOT EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_supervisor_session_policy_companions companion
           WHERE companion.companion_id=NEW.companion_id
             AND companion.companion_digest=NEW.companion_digest
             AND companion.target_id=NEW.target_id
             AND companion.target_digest=NEW.target_digest
             AND companion.profile_id=NEW.profile_id
             AND companion.profile_digest=NEW.profile_digest
             AND companion.provider_binding_id=NEW.provider_binding_id
             AND companion.provider_binding_digest=NEW.provider_binding_digest
             AND companion.provider_id=NEW.provider_id
             AND companion.recorded_at<=NEW.revoked_at
             AND NOT EXISTS (
               SELECT 1 FROM compute_external_pool_adapter_supervisor_session_policy_companions successor
                WHERE successor.predecessor_companion_id=companion.companion_id)
             AND NOT EXISTS (
               SELECT 1 FROM compute_external_pool_adapter_supervisor_session_policy_companion_revocations prior
                WHERE prior.companion_id=companion.companion_id))
        BEGIN SELECT RAISE(ABORT,'V259 revocation requires exact current unrevoked companion head'); END;
        "#,
    )?;
    Ok(())
}
