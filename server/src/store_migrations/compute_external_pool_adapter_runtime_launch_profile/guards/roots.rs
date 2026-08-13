use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_runtime_launch_profile_exact_roots
        BEFORE INSERT ON compute_external_pool_adapter_runtime_launch_profiles
        WHEN NOT EXISTS (
          SELECT 1
            FROM compute_external_pool_provider_activation_candidates candidate
            JOIN compute_external_pool_provider_activation_delegations delegation
              ON delegation.delegation_id=candidate.delegation_id
             AND delegation.delegation_digest=candidate.delegation_digest
            JOIN compute_external_pool_adapter_registry_provider_bindings binding
              ON binding.provider_binding_id=candidate.provider_binding_id
             AND binding.provider_binding_digest=candidate.provider_binding_digest
            JOIN compute_external_pool_adapter_registry_provider_binding_current current_binding
              ON current_binding.provider_binding_id=binding.provider_binding_id
             AND current_binding.provider_binding_digest=binding.provider_binding_digest
            JOIN compute_external_pool_adapter_registry_releases release
              ON release.registry_release_id=binding.registry_release_id
             AND release.registry_release_digest=binding.registry_release_digest
            JOIN compute_external_pool_adapter_registry_release_current current_release
              ON current_release.registry_release_id=release.registry_release_id
             AND current_release.registry_release_digest=release.registry_release_digest
            JOIN compute_external_pool_adapter_installation_receipts installation
              ON installation.installation_receipt_id=binding.installation_receipt_id
             AND installation.installation_receipt_digest=binding.installation_receipt_digest
            JOIN compute_external_pool_adapter_installation_current current_installation
              ON current_installation.installation_receipt_id=installation.installation_receipt_id
             AND current_installation.installation_receipt_digest=installation.installation_receipt_digest
            JOIN compute_external_pool_onboarding_applications onboarding
              ON onboarding.application_id=binding.application_id
             AND onboarding.application_digest=binding.application_digest
            JOIN compute_providers provider ON provider.provider_id=binding.provider_id
            JOIN compute_provider_versions provider_version
              ON provider_version.provider_id=provider.provider_id
             AND provider_version.policy_revision=provider.current_policy_revision
             AND provider_version.provider_digest=provider.current_provider_digest
           WHERE candidate.candidate_id=NEW.candidate_id
             AND candidate.candidate_digest=NEW.candidate_digest
             AND candidate.candidate_status='candidate_current_not_activation_ready'
             AND candidate.activation_closure_status='activation_closure_not_implemented'
             AND NOT EXISTS (
               SELECT 1 FROM compute_external_pool_provider_activation_candidates later
                WHERE later.provider_binding_id=candidate.provider_binding_id
                  AND later.sequence>candidate.sequence)
             AND NOT EXISTS (
               SELECT 1 FROM compute_external_pool_provider_activation_delegations later_delegation
                WHERE later_delegation.provider_binding_id=delegation.provider_binding_id
                  AND later_delegation.sequence>delegation.sequence)
             AND NOT EXISTS (
               SELECT 1 FROM compute_external_pool_provider_activation_delegation_revocations revoked
                WHERE revoked.delegation_id=delegation.delegation_id
                  AND revoked.delegation_digest=delegation.delegation_digest)
             AND delegation.issued_at<=NEW.recorded_at
             AND candidate.checked_at<=NEW.recorded_at
             AND delegation.delegation_id=NEW.delegation_id
             AND delegation.delegation_digest=NEW.delegation_digest
             AND candidate.provider_binding_id=NEW.provider_binding_id
             AND candidate.provider_binding_digest=NEW.provider_binding_digest
             AND binding.registry_release_id=NEW.registry_release_id
             AND binding.registry_release_digest=NEW.registry_release_digest
             AND current_binding.current_status='binding_current'
             AND current_binding.projection_status='reserved'
             AND current_release.current_status='release_current'
             AND binding.installation_receipt_id=NEW.installation_receipt_id
             AND binding.installation_receipt_digest=NEW.installation_receipt_digest
             AND binding.installation_content_digest=NEW.installation_content_digest
             AND current_installation.current_status='installed_upstreams_current'
             AND installation.installation_content_digest=NEW.installation_content_digest
             AND installation.declared_implementation_sha256=NEW.implementation_digest
             AND installation.capability_set_digest=NEW.capability_set_digest
             AND installation.entrypoint_sha256=NEW.entrypoint_sha256
             AND installation.entrypoint_size_bytes=NEW.entrypoint_size_bytes
             AND installation.entry_inventory_digest=NEW.entry_inventory_digest
             AND installation.entry_count=NEW.installed_file_count
             AND installation.total_uncompressed_bytes=NEW.installed_total_bytes
             AND release.implementation_digest=NEW.implementation_digest
             AND release.capability_set_digest=NEW.capability_set_digest
             AND release.credential_verifier_digest=NEW.credential_verifier_digest
             AND release.entry_inventory_digest=NEW.entry_inventory_digest
             AND release.entry_count=NEW.installed_file_count
             AND release.total_uncompressed_bytes=NEW.installed_total_bytes
             AND candidate.registry_release_id=NEW.registry_release_id
             AND candidate.registry_release_digest=NEW.registry_release_digest
             AND candidate.installation_receipt_id=NEW.installation_receipt_id
             AND candidate.installation_receipt_digest=NEW.installation_receipt_digest
             AND candidate.installation_content_digest=NEW.installation_content_digest
             AND candidate.route_adapter_projection_id=NEW.route_adapter_projection_id
             AND binding.route_adapter_projection_id=NEW.route_adapter_projection_id
             AND candidate.provider_id=NEW.provider_id
             AND candidate.provider_owner_account_id=NEW.provider_owner_account_id
             AND candidate.provider_policy_revision=NEW.provider_policy_revision
             AND candidate.provider_digest=NEW.provider_digest
             AND candidate.provider_status=NEW.provider_status
             AND candidate.logical_adapter_id=NEW.logical_adapter_id
             AND candidate.release_version=NEW.release_version
             AND candidate.adapter_config_revision=NEW.adapter_config_revision
             AND candidate.adapter_config_digest=NEW.adapter_config_digest
             AND candidate.implementation_digest=NEW.implementation_digest
             AND candidate.capability_set_digest=NEW.capability_set_digest
             AND candidate.credential_verifier_digest=NEW.credential_verifier_digest
             AND binding.credential_locator_commitment=NEW.credential_locator_commitment
             AND NEW.credential_ref_scheme='vault_ref'
             AND substr(onboarding.non_bearer_credential_ref,1,10)='vault-ref:'
             AND candidate.service_actor_id=NEW.service_actor_id
             AND installation.entrypoint_path=NEW.entrypoint_relative_path
             AND provider.provider_kind='external_pool'
             AND provider.owner_account_id=NEW.provider_owner_account_id
             AND provider.status='registering'
             AND provider.current_policy_revision=NEW.provider_policy_revision
             AND provider.current_provider_digest=NEW.provider_digest
             AND json_extract(provider_version.provider_json,'$.provider_id')=NEW.provider_id
             AND json_extract(provider_version.provider_json,'$.provider_kind')='external_pool'
             AND json_extract(provider_version.provider_json,'$.owner_account_id')=NEW.provider_owner_account_id
             AND json_extract(provider_version.provider_json,'$.status')='registering'
             AND json_extract(provider_version.provider_json,'$.policy_revision')=NEW.provider_policy_revision
             AND json_extract(provider_version.provider_json,'$.adapter.adapter_id')=NEW.logical_adapter_id
             AND json_extract(provider_version.provider_json,'$.adapter.adapter_version')=NEW.release_version
             AND json_extract(provider_version.provider_json,'$.adapter.config_revision')=NEW.adapter_config_revision
             AND json_extract(provider_version.provider_json,'$.adapter.config_digest')=NEW.adapter_config_digest
             AND (
               (NEW.recorded_by_actor_kind='provider_owner'
                AND NEW.recorded_by_actor_user_id=NEW.provider_owner_account_id)
               OR
               (NEW.recorded_by_actor_kind='platform_admin'
                AND EXISTS (SELECT 1 FROM users actor
                             WHERE actor.id=NEW.recorded_by_actor_user_id
                               AND actor.role IN ('admin','owner')
                               AND actor.status='active')))
        )
        BEGIN SELECT RAISE(ABORT,'V255 profile lacks exact current V249/V254/registering Provider roots'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_runtime_launch_profile_revocation_exact_roots
        BEFORE INSERT ON compute_external_pool_adapter_runtime_launch_profile_revocations
        WHEN NOT EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_runtime_launch_profiles target
           WHERE target.profile_id=NEW.profile_id
             AND target.profile_digest=NEW.profile_digest
             AND target.provider_binding_id=NEW.provider_binding_id
             AND target.provider_binding_digest=NEW.provider_binding_digest
             AND target.candidate_id=NEW.candidate_id
             AND target.candidate_digest=NEW.candidate_digest
             AND (
               (NEW.revoked_by_actor_kind='provider_owner'
                AND NEW.revoked_by_actor_user_id=target.provider_owner_account_id)
               OR
               (NEW.revoked_by_actor_kind='platform_admin'
                AND EXISTS (SELECT 1 FROM users actor
                             WHERE actor.id=NEW.revoked_by_actor_user_id
                               AND actor.role IN ('admin','owner')
                               AND actor.status='active'))))
        BEGIN SELECT RAISE(ABORT,'V255 revocation lacks exact profile and authorized actor roots'); END;
        "#,
    )?;
    Ok(())
}
