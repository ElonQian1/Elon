use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS external_pool_provider_activation_delegation_exact_roots
        BEFORE INSERT ON compute_external_pool_provider_activation_delegations
        WHEN NOT EXISTS (
          SELECT 1
            FROM compute_external_pool_adapter_registry_provider_bindings binding
            JOIN compute_external_pool_adapter_registry_provider_binding_current current_binding
              ON current_binding.provider_binding_id=binding.provider_binding_id
             AND current_binding.provider_binding_digest=binding.provider_binding_digest
             AND current_binding.current_status='binding_current'
             AND current_binding.projection_status='reserved'
            JOIN compute_external_pool_adapter_registry_releases release
              ON release.registry_release_id=binding.registry_release_id
             AND release.registry_release_digest=binding.registry_release_digest
            JOIN compute_providers provider ON provider.provider_id=binding.provider_id
            JOIN compute_provider_versions version
              ON version.provider_id=provider.provider_id
             AND version.policy_revision=provider.current_policy_revision
             AND version.provider_digest=provider.current_provider_digest
           WHERE binding.provider_binding_id=NEW.provider_binding_id
             AND binding.provider_binding_digest=NEW.provider_binding_digest
             AND binding.registry_release_id=NEW.registry_release_id
             AND binding.registry_release_digest=NEW.registry_release_digest
             AND binding.route_adapter_projection_id=NEW.route_adapter_projection_id
             AND binding.provider_id=NEW.provider_id
             AND binding.provider_owner_account_id=NEW.provider_owner_account_id
             AND binding.provider_policy_revision=NEW.provider_policy_revision
             AND binding.provider_digest=NEW.provider_digest
             AND binding.adapter_id=NEW.logical_adapter_id
             AND binding.release_version=NEW.release_version
             AND binding.adapter_config_revision=NEW.adapter_config_revision
             AND binding.adapter_config_digest=NEW.adapter_config_digest
             AND provider.provider_kind='external_pool'
             AND provider.owner_account_id=NEW.provider_owner_account_id
             AND provider.status='registering'
             AND provider.current_policy_revision=NEW.provider_policy_revision
             AND provider.current_provider_digest=NEW.provider_digest
             AND json_extract(version.provider_json,'$.provider_id')=NEW.provider_id
             AND json_extract(version.provider_json,'$.provider_kind')='external_pool'
             AND json_extract(version.provider_json,'$.owner_account_id')=NEW.provider_owner_account_id
             AND json_extract(version.provider_json,'$.status')='registering'
             AND json_extract(version.provider_json,'$.policy_revision')=NEW.provider_policy_revision
             AND json_extract(version.provider_json,'$.adapter.adapter_id')=NEW.logical_adapter_id
             AND json_extract(version.provider_json,'$.adapter.adapter_version')=NEW.release_version
             AND json_extract(version.provider_json,'$.adapter.config_revision')=NEW.adapter_config_revision
             AND json_extract(version.provider_json,'$.adapter.config_digest')=NEW.adapter_config_digest
             AND NEW.provider_status=provider.status
             AND NEW.issued_by_owner_user_id=provider.owner_account_id
             AND NOT EXISTS (
                   SELECT 1 FROM compute_route_adapters adapter
                    WHERE adapter.adapter_id=NEW.route_adapter_projection_id
             )
             AND NOT EXISTS (
                   SELECT 1 FROM compute_route_adapter_versions version
                    WHERE version.adapter_id=NEW.route_adapter_projection_id
             )
        )
        BEGIN SELECT RAISE(ABORT,'V254 delegation lacks exact V249/registering Provider roots'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_provider_activation_candidate_exact_roots
        BEFORE INSERT ON compute_external_pool_provider_activation_candidates
        WHEN NOT EXISTS (
          SELECT 1
            FROM compute_external_pool_provider_activation_delegations delegation
            JOIN compute_external_pool_adapter_registry_provider_bindings binding
              ON binding.provider_binding_id=delegation.provider_binding_id
             AND binding.provider_binding_digest=delegation.provider_binding_digest
            JOIN compute_external_pool_adapter_registry_provider_binding_current current_binding
              ON current_binding.provider_binding_id=binding.provider_binding_id
             AND current_binding.provider_binding_digest=binding.provider_binding_digest
             AND current_binding.current_status='binding_current'
             AND current_binding.projection_status='reserved'
            JOIN compute_external_pool_adapter_registry_releases release
              ON release.registry_release_id=binding.registry_release_id
             AND release.registry_release_digest=binding.registry_release_digest
            JOIN compute_external_pool_adapter_installation_receipts installation
              ON installation.installation_receipt_id=binding.installation_receipt_id
             AND installation.installation_receipt_digest=binding.installation_receipt_digest
            JOIN compute_providers provider ON provider.provider_id=binding.provider_id
            JOIN compute_provider_versions version
              ON version.provider_id=provider.provider_id
             AND version.policy_revision=provider.current_policy_revision
             AND version.provider_digest=provider.current_provider_digest
           WHERE delegation.delegation_id=NEW.delegation_id
             AND delegation.delegation_digest=NEW.delegation_digest
             AND delegation.provider_binding_id=NEW.provider_binding_id
             AND delegation.provider_binding_digest=NEW.provider_binding_digest
             AND delegation.registry_release_id=NEW.registry_release_id
             AND delegation.registry_release_digest=NEW.registry_release_digest
             AND delegation.route_adapter_projection_id=NEW.route_adapter_projection_id
             AND delegation.provider_id=NEW.provider_id
             AND delegation.provider_owner_account_id=NEW.provider_owner_account_id
             AND delegation.provider_policy_revision=NEW.provider_policy_revision
             AND delegation.provider_digest=NEW.provider_digest
             AND delegation.provider_status=NEW.provider_status
             AND delegation.logical_adapter_id=NEW.logical_adapter_id
             AND delegation.release_version=NEW.release_version
             AND delegation.adapter_config_revision=NEW.adapter_config_revision
             AND delegation.adapter_config_digest=NEW.adapter_config_digest
             AND delegation.service_actor_id=NEW.service_actor_id
             AND binding.installation_receipt_id=NEW.installation_receipt_id
             AND binding.installation_receipt_digest=NEW.installation_receipt_digest
             AND binding.installation_content_digest=NEW.installation_content_digest
             AND release.implementation_digest=NEW.implementation_digest
             AND release.capability_set_digest=NEW.capability_set_digest
             AND release.credential_verifier_digest=NEW.credential_verifier_digest
             AND installation.installation_content_digest=NEW.installation_content_digest
             AND provider.provider_kind='external_pool'
             AND provider.owner_account_id=NEW.provider_owner_account_id
             AND provider.status='registering'
             AND provider.current_policy_revision=NEW.provider_policy_revision
             AND provider.current_provider_digest=NEW.provider_digest
             AND json_extract(version.provider_json,'$.provider_id')=NEW.provider_id
             AND json_extract(version.provider_json,'$.provider_kind')='external_pool'
             AND json_extract(version.provider_json,'$.owner_account_id')=NEW.provider_owner_account_id
             AND json_extract(version.provider_json,'$.status')='registering'
             AND json_extract(version.provider_json,'$.policy_revision')=NEW.provider_policy_revision
             AND json_extract(version.provider_json,'$.adapter.adapter_id')=NEW.logical_adapter_id
             AND json_extract(version.provider_json,'$.adapter.adapter_version')=NEW.release_version
             AND json_extract(version.provider_json,'$.adapter.config_revision')=NEW.adapter_config_revision
             AND json_extract(version.provider_json,'$.adapter.config_digest')=NEW.adapter_config_digest
             AND NEW.provider_status=provider.status
             AND NOT EXISTS (
                   SELECT 1 FROM compute_external_pool_adapter_installation_terminal_receipts terminal
                    WHERE terminal.installation_receipt_id=NEW.installation_receipt_id
                      AND terminal.installation_receipt_digest=NEW.installation_receipt_digest
             )
             AND NOT EXISTS (
                   SELECT 1 FROM compute_route_adapters adapter
                    WHERE adapter.adapter_id=NEW.route_adapter_projection_id
             )
             AND NOT EXISTS (
                   SELECT 1 FROM compute_route_adapter_versions version
                    WHERE version.adapter_id=NEW.route_adapter_projection_id
             )
             AND NOT EXISTS (
                   SELECT 1 FROM compute_service_actor_authorizations actor
                    WHERE actor.service_actor_id=NEW.service_actor_id
                       OR (actor.provider_id=NEW.provider_id
                           AND actor.service_actor_kind='platform_dispatch_service')
             )
        )
        BEGIN SELECT RAISE(ABORT,'V254 candidate lacks exact delegation/V249/registering Provider roots'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_provider_activation_revocation_exact_roots
        BEFORE INSERT ON compute_external_pool_provider_activation_delegation_revocations
        WHEN NOT EXISTS (
          SELECT 1
            FROM compute_external_pool_provider_activation_delegations delegation
            JOIN compute_external_pool_provider_activation_candidates candidate
              ON candidate.delegation_id=delegation.delegation_id
             AND candidate.delegation_digest=delegation.delegation_digest
            JOIN compute_external_pool_adapter_registry_provider_bindings binding
              ON binding.provider_binding_id=delegation.provider_binding_id
             AND binding.provider_binding_digest=delegation.provider_binding_digest
            JOIN compute_providers provider ON provider.provider_id=delegation.provider_id
           WHERE delegation.delegation_id=NEW.delegation_id
             AND delegation.delegation_digest=NEW.delegation_digest
             AND candidate.candidate_id=NEW.candidate_id
             AND candidate.candidate_digest=NEW.candidate_digest
             AND delegation.provider_binding_id=NEW.provider_binding_id
             AND delegation.provider_binding_digest=NEW.provider_binding_digest
             AND delegation.provider_id=NEW.provider_id
             AND candidate.provider_binding_id=NEW.provider_binding_id
             AND candidate.provider_binding_digest=NEW.provider_binding_digest
             AND candidate.provider_id=NEW.provider_id
             AND provider.provider_kind='external_pool'
             AND provider.owner_account_id=NEW.revoked_by_owner_user_id
        )
        BEGIN SELECT RAISE(ABORT,'V254 revocation lacks exact owner/delegation/candidate roots'); END;
        "#,
    )?;
    Ok(())
}
