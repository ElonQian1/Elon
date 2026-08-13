use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_runtime_launch_profile_lineage
        BEFORE INSERT ON compute_external_pool_adapter_runtime_launch_profiles
        WHEN NOT (
          (NEW.sequence=1
           AND NEW.predecessor_profile_id IS NULL
           AND NEW.predecessor_profile_digest IS NULL
           AND NOT EXISTS (
             SELECT 1 FROM compute_external_pool_adapter_runtime_launch_profiles existing
              WHERE existing.provider_binding_id=NEW.provider_binding_id))
          OR
          (NEW.sequence>1
           AND EXISTS (
             SELECT 1 FROM compute_external_pool_adapter_runtime_launch_profiles predecessor
              WHERE predecessor.profile_id=NEW.predecessor_profile_id
                AND predecessor.profile_digest=NEW.predecessor_profile_digest
                AND predecessor.provider_binding_id=NEW.provider_binding_id
                AND predecessor.provider_binding_digest=NEW.provider_binding_digest
                AND predecessor.sequence=NEW.sequence-1
                AND predecessor.recorded_at<=NEW.recorded_at
                AND NOT EXISTS (
                  SELECT 1 FROM compute_external_pool_adapter_runtime_launch_profiles successor
                   WHERE successor.predecessor_profile_id=predecessor.profile_id)))
        )
        BEGIN SELECT RAISE(ABORT,'V255 profile requires exact structural predecessor head'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_runtime_launch_profile_revocation_lineage
        BEFORE INSERT ON compute_external_pool_adapter_runtime_launch_profile_revocations
        WHEN NOT EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_runtime_launch_profiles target
           WHERE target.profile_id=NEW.profile_id
             AND target.profile_digest=NEW.profile_digest
             AND target.provider_binding_id=NEW.provider_binding_id
             AND target.provider_binding_digest=NEW.provider_binding_digest
             AND target.candidate_id=NEW.candidate_id
             AND target.candidate_digest=NEW.candidate_digest
             AND target.recorded_at<=NEW.revoked_at
             AND NOT EXISTS (
               SELECT 1 FROM compute_external_pool_adapter_runtime_launch_profiles successor
                WHERE successor.predecessor_profile_id=target.profile_id)
             AND NOT EXISTS (
               SELECT 1 FROM compute_external_pool_adapter_runtime_launch_profile_revocations prior
                WHERE prior.profile_id=target.profile_id))
        BEGIN SELECT RAISE(ABORT,'V255 revocation requires exact current unrevoked profile head'); END;
        "#,
    )?;
    Ok(())
}
