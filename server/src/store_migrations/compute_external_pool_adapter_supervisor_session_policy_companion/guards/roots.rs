use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_supervisor_session_policy_companion_exact_roots
        BEFORE INSERT ON compute_external_pool_adapter_supervisor_session_policy_companions
        WHEN NOT EXISTS (
          SELECT 1
            FROM compute_external_pool_adapter_upstream_transport_targets target
            JOIN compute_external_pool_adapter_upstream_transport_target_current current_target
              ON current_target.target_id=target.target_id
             AND current_target.target_digest=target.target_digest
            JOIN compute_external_pool_adapter_runtime_launch_profiles profile
              ON profile.profile_id=target.profile_id
             AND profile.profile_digest=target.profile_digest
           WHERE target.target_id=NEW.target_id
             AND target.target_digest=NEW.target_digest
             AND current_target.current_status='upstream_transport_target_current_inert'
             AND current_target.head_status='head'
             AND current_target.revocation_status='unrevoked'
             AND current_target.profile_status='launch_profile_current_inert'
             AND current_target.target_policy_status='server_policy_current'
             AND target.recorded_at<=NEW.recorded_at
             AND target.profile_id=NEW.profile_id
             AND target.profile_digest=NEW.profile_digest
             AND target.candidate_id=NEW.candidate_id
             AND target.candidate_digest=NEW.candidate_digest
             AND target.delegation_id=NEW.delegation_id
             AND target.delegation_digest=NEW.delegation_digest
             AND target.provider_binding_id=NEW.provider_binding_id
             AND target.provider_binding_digest=NEW.provider_binding_digest
             AND target.registry_release_id=NEW.registry_release_id
             AND target.registry_release_digest=NEW.registry_release_digest
             AND target.installation_receipt_id=NEW.installation_receipt_id
             AND target.installation_receipt_digest=NEW.installation_receipt_digest
             AND target.installation_content_digest=NEW.installation_content_digest
             AND target.route_adapter_projection_id=NEW.route_adapter_projection_id
             AND target.provider_id=NEW.provider_id
             AND target.provider_owner_account_id=NEW.provider_owner_account_id
             AND target.provider_policy_revision=NEW.provider_policy_revision
             AND target.provider_digest=NEW.provider_digest
             AND target.provider_status=NEW.provider_status
             AND target.logical_adapter_id=NEW.logical_adapter_id
             AND target.release_version=NEW.release_version
             AND target.adapter_config_revision=NEW.adapter_config_revision
             AND target.adapter_config_digest=NEW.adapter_config_digest
             AND target.implementation_digest=NEW.implementation_digest
             AND target.capability_set_digest=NEW.capability_set_digest
             AND target.credential_verifier_digest=NEW.credential_verifier_digest
             AND target.service_actor_id=NEW.service_actor_id
             AND target.launch_policy_digest=NEW.launch_policy_digest
             AND target.network_egress_policy_id=NEW.network_egress_policy_id
             AND target.network_egress_policy_revision=NEW.network_egress_policy_revision
             AND target.network_egress_policy_digest=NEW.network_egress_policy_digest
             AND target.target_policy_digest=NEW.target_policy_digest
             AND profile.candidate_id=target.candidate_id
             AND profile.candidate_digest=target.candidate_digest
             AND profile.delegation_id=target.delegation_id
             AND profile.delegation_digest=target.delegation_digest
             AND profile.provider_binding_id=target.provider_binding_id
             AND profile.provider_binding_digest=target.provider_binding_digest
             AND profile.registry_release_id=target.registry_release_id
             AND profile.registry_release_digest=target.registry_release_digest
             AND profile.installation_receipt_id=target.installation_receipt_id
             AND profile.installation_receipt_digest=target.installation_receipt_digest
             AND profile.installation_content_digest=target.installation_content_digest
             AND profile.route_adapter_projection_id=target.route_adapter_projection_id
             AND profile.provider_id=target.provider_id
             AND profile.provider_owner_account_id=target.provider_owner_account_id
             AND profile.provider_policy_revision=target.provider_policy_revision
             AND profile.provider_digest=target.provider_digest
             AND profile.provider_status=target.provider_status
             AND profile.logical_adapter_id=target.logical_adapter_id
             AND profile.release_version=target.release_version
             AND profile.adapter_config_revision=target.adapter_config_revision
             AND profile.adapter_config_digest=target.adapter_config_digest
             AND profile.implementation_digest=target.implementation_digest
             AND profile.capability_set_digest=target.capability_set_digest
             AND profile.credential_verifier_digest=target.credential_verifier_digest
             AND profile.service_actor_id=target.service_actor_id
             AND profile.launch_policy_digest=target.launch_policy_digest
             AND json_type(profile.launch_policy_json,'$.process_isolation_policy_id')='text'
             AND json_extract(profile.launch_policy_json,'$.process_isolation_policy_id')=NEW.process_isolation_policy_id
             AND json_type(profile.launch_policy_json,'$.process_isolation_policy_revision')='integer'
             AND json_extract(profile.launch_policy_json,'$.process_isolation_policy_revision')=NEW.process_isolation_policy_revision
             AND json_type(profile.launch_policy_json,'$.process_isolation_policy_digest')='text'
             AND json_extract(profile.launch_policy_json,'$.process_isolation_policy_digest')=NEW.process_isolation_policy_digest
             AND json_type(profile.launch_policy_json,'$.resource_policy_id')='text'
             AND json_extract(profile.launch_policy_json,'$.resource_policy_id')=NEW.resource_policy_id
             AND json_type(profile.launch_policy_json,'$.resource_policy_revision')='integer'
             AND json_extract(profile.launch_policy_json,'$.resource_policy_revision')=NEW.resource_policy_revision
             AND json_type(profile.launch_policy_json,'$.resource_policy_digest')='text'
             AND json_extract(profile.launch_policy_json,'$.resource_policy_digest')=NEW.resource_policy_digest
             AND json_type(profile.launch_policy_json,'$.network_egress_policy_id')='text'
             AND json_extract(profile.launch_policy_json,'$.network_egress_policy_id')=NEW.network_egress_policy_id
             AND json_type(profile.launch_policy_json,'$.network_egress_policy_revision')='integer'
             AND json_extract(profile.launch_policy_json,'$.network_egress_policy_revision')=NEW.network_egress_policy_revision
             AND json_type(profile.launch_policy_json,'$.network_egress_policy_digest')='text'
             AND json_extract(profile.launch_policy_json,'$.network_egress_policy_digest')=NEW.network_egress_policy_digest
             AND NEW.entrypoint_capsule_policy_id='external_pool_adapter_entrypoint_capsule_policy_v1'
             AND NEW.entrypoint_capsule_policy_revision=1
             AND NEW.entrypoint_capsule_policy_digest='710decef25b4d19b33f086239f55f809a513508eb5ba431967971ff89249604f'
             AND (
               (NEW.recorded_by_actor_kind='provider_owner'
                AND NEW.recorded_by_actor_user_id=target.provider_owner_account_id)
               OR
               (NEW.recorded_by_actor_kind='platform_admin'
                AND EXISTS (SELECT 1 FROM users actor
                             WHERE actor.id=NEW.recorded_by_actor_user_id
                               AND actor.role IN ('admin','owner')
                               AND actor.status='active'))))
        BEGIN SELECT RAISE(ABORT,'V259 companion lacks exact current V258/V255 roots'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_supervisor_session_policy_companion_revocation_exact_roots
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
             AND (
               (NEW.revoked_by_actor_kind='provider_owner'
                AND NEW.revoked_by_actor_user_id=companion.provider_owner_account_id)
               OR
               (NEW.revoked_by_actor_kind='platform_admin'
                AND EXISTS (SELECT 1 FROM users actor
                             WHERE actor.id=NEW.revoked_by_actor_user_id
                               AND actor.role IN ('admin','owner')
                               AND actor.status='active'))))
        BEGIN SELECT RAISE(ABORT,'V259 revocation lacks exact companion and authorized actor roots'); END;
        "#,
    )?;
    Ok(())
}
