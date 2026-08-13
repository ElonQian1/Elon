use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_upstream_transport_target_exact_roots
        BEFORE INSERT ON compute_external_pool_adapter_upstream_transport_targets
        WHEN NOT EXISTS (
          SELECT 1
            FROM compute_external_pool_adapter_runtime_launch_profiles profile
            JOIN compute_external_pool_adapter_runtime_launch_profile_current current_profile
              ON current_profile.profile_id=profile.profile_id
             AND current_profile.profile_digest=profile.profile_digest
           WHERE profile.profile_id=NEW.profile_id
             AND profile.profile_digest=NEW.profile_digest
             AND current_profile.current_status='launch_profile_current_inert'
             AND current_profile.head_status='head'
             AND current_profile.revocation_status='none'
             AND current_profile.runtime_launch_ready=0
             AND profile.recorded_at<=NEW.recorded_at
             AND profile.candidate_id=NEW.candidate_id
             AND profile.candidate_digest=NEW.candidate_digest
             AND profile.delegation_id=NEW.delegation_id
             AND profile.delegation_digest=NEW.delegation_digest
             AND profile.provider_binding_id=NEW.provider_binding_id
             AND profile.provider_binding_digest=NEW.provider_binding_digest
             AND profile.registry_release_id=NEW.registry_release_id
             AND profile.registry_release_digest=NEW.registry_release_digest
             AND profile.installation_receipt_id=NEW.installation_receipt_id
             AND profile.installation_receipt_digest=NEW.installation_receipt_digest
             AND profile.installation_content_digest=NEW.installation_content_digest
             AND profile.route_adapter_projection_id=NEW.route_adapter_projection_id
             AND profile.provider_id=NEW.provider_id
             AND profile.provider_owner_account_id=NEW.provider_owner_account_id
             AND profile.provider_policy_revision=NEW.provider_policy_revision
             AND profile.provider_digest=NEW.provider_digest
             AND profile.provider_status=NEW.provider_status
             AND profile.logical_adapter_id=NEW.logical_adapter_id
             AND profile.release_version=NEW.release_version
             AND profile.adapter_config_revision=NEW.adapter_config_revision
             AND profile.adapter_config_digest=NEW.adapter_config_digest
             AND profile.implementation_digest=NEW.implementation_digest
             AND profile.capability_set_digest=NEW.capability_set_digest
             AND profile.credential_verifier_digest=NEW.credential_verifier_digest
             AND profile.launch_policy_digest=NEW.launch_policy_digest
             AND json_type(profile.launch_policy_json,'$.network_egress_policy_id')='text'
             AND json_extract(profile.launch_policy_json,'$.network_egress_policy_id')=NEW.network_egress_policy_id
             AND json_type(profile.launch_policy_json,'$.network_egress_policy_revision')='integer'
             AND json_extract(profile.launch_policy_json,'$.network_egress_policy_revision')=NEW.network_egress_policy_revision
             AND json_type(profile.launch_policy_json,'$.network_egress_policy_digest')='text'
             AND json_extract(profile.launch_policy_json,'$.network_egress_policy_digest')=NEW.network_egress_policy_digest
             AND profile.service_actor_id=NEW.service_actor_id
             AND (
               (NEW.recorded_by_actor_kind='provider_owner'
                AND NEW.recorded_by_actor_user_id=profile.provider_owner_account_id)
               OR
               (NEW.recorded_by_actor_kind='platform_admin'
                AND EXISTS (SELECT 1 FROM users actor
                             WHERE actor.id=NEW.recorded_by_actor_user_id
                               AND actor.role IN ('admin','owner')
                               AND actor.status='active'))))
        BEGIN SELECT RAISE(ABORT,'V258 target lacks exact current unrevoked V255 roots'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_upstream_transport_target_revocation_exact_roots
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
             AND (
               (NEW.revoked_by_actor_kind='provider_owner'
                AND NEW.revoked_by_actor_user_id=target.provider_owner_account_id)
               OR
               (NEW.revoked_by_actor_kind='platform_admin'
                AND EXISTS (SELECT 1 FROM users actor
                             WHERE actor.id=NEW.revoked_by_actor_user_id
                               AND actor.role IN ('admin','owner')
                               AND actor.status='active'))))
        BEGIN SELECT RAISE(ABORT,'V258 revocation lacks exact target and authorized actor roots'); END;
        "#,
    )?;
    Ok(())
}
