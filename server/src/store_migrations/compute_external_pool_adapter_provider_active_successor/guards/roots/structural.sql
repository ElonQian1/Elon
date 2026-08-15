CREATE TRIGGER IF NOT EXISTS v274_provider_active_successor_structural_roots
BEFORE INSERT ON compute_external_pool_adapter_provider_active_successor_receipts
WHEN NOT EXISTS (
  SELECT 1
    FROM compute_external_pool_adapter_registry_provider_bindings binding
    JOIN compute_external_pool_adapter_registry_releases release
      ON release.registry_release_id=binding.registry_release_id
     AND release.registry_release_digest=binding.registry_release_digest
    JOIN compute_external_pool_provider_activation_delegations delegation
      ON delegation.delegation_id=NEW.delegation_id
     AND delegation.delegation_digest=NEW.delegation_digest
    JOIN compute_external_pool_provider_activation_candidates candidate
      ON candidate.candidate_id=NEW.candidate_id
     AND candidate.candidate_digest=NEW.candidate_digest
     AND candidate.delegation_id=delegation.delegation_id
     AND candidate.delegation_digest=delegation.delegation_digest
    JOIN compute_external_pool_adapter_runtime_launch_profiles profile
      ON profile.profile_id=NEW.profile_id
     AND profile.profile_digest=NEW.profile_digest
    JOIN compute_external_pool_adapter_upstream_transport_targets target
      ON target.target_id=NEW.target_id
     AND target.target_digest=NEW.target_digest
    JOIN compute_external_pool_adapter_supervisor_session_policy_companions companion
      ON companion.companion_id=NEW.companion_id
     AND companion.companion_digest=NEW.companion_digest
    JOIN compute_provider_versions source_version
      ON source_version.provider_id=NEW.source_registering_provider_id
     AND source_version.policy_revision=NEW.source_registering_provider_policy_revision
     AND source_version.provider_digest=NEW.source_registering_provider_digest
   WHERE binding.provider_binding_id=NEW.provider_binding_id
     AND binding.provider_binding_digest=NEW.provider_binding_digest
     AND binding.registry_release_id=NEW.registry_release_id
     AND binding.registry_release_digest=NEW.registry_release_digest
     AND binding.installation_receipt_id=NEW.installation_receipt_id
     AND binding.installation_receipt_digest=NEW.installation_receipt_digest
     AND binding.installation_content_digest=NEW.installation_content_digest
     AND binding.provider_id=NEW.provider_id
     AND binding.provider_owner_account_id=NEW.provider_owner_account_id
     AND binding.provider_policy_revision=NEW.source_registering_provider_policy_revision
     AND binding.provider_digest=NEW.source_registering_provider_digest
     AND binding.adapter_id=NEW.logical_adapter_id
     AND binding.route_adapter_projection_id=NEW.route_adapter_projection_id
     AND release.registry_release_material_digest=NEW.registry_release_material_digest
     AND release.adapter_id=NEW.logical_adapter_id
     AND source_version.provider_json=NEW.source_registering_provider_json
     AND json_extract(source_version.provider_json,'$.provider_id')=NEW.provider_id
     AND json_extract(source_version.provider_json,'$.provider_kind')='external_pool'
     AND json_extract(source_version.provider_json,'$.owner_account_id')=NEW.provider_owner_account_id
     AND json_extract(source_version.provider_json,'$.status')='registering'
     AND json_extract(source_version.provider_json,'$.adapter.adapter_id')=NEW.logical_adapter_id
     AND delegation.provider_binding_id=NEW.provider_binding_id
     AND delegation.provider_binding_digest=NEW.provider_binding_digest
     AND delegation.registry_release_id=NEW.registry_release_id
     AND delegation.registry_release_digest=NEW.registry_release_digest
     AND delegation.route_adapter_projection_id=NEW.route_adapter_projection_id
     AND delegation.provider_id=NEW.provider_id
     AND delegation.provider_owner_account_id=NEW.provider_owner_account_id
     AND delegation.provider_policy_revision=NEW.source_registering_provider_policy_revision
     AND delegation.provider_digest=NEW.source_registering_provider_digest
     AND delegation.logical_adapter_id=NEW.logical_adapter_id
     AND delegation.service_actor_id=NEW.service_actor_id
     AND candidate.provider_binding_id=NEW.provider_binding_id
     AND candidate.provider_binding_digest=NEW.provider_binding_digest
     AND candidate.registry_release_id=NEW.registry_release_id
     AND candidate.registry_release_digest=NEW.registry_release_digest
     AND candidate.installation_receipt_id=NEW.installation_receipt_id
     AND candidate.installation_receipt_digest=NEW.installation_receipt_digest
     AND candidate.installation_content_digest=NEW.installation_content_digest
     AND candidate.route_adapter_projection_id=NEW.route_adapter_projection_id
     AND candidate.provider_id=NEW.provider_id
     AND candidate.provider_owner_account_id=NEW.provider_owner_account_id
     AND candidate.provider_policy_revision=NEW.source_registering_provider_policy_revision
     AND candidate.provider_digest=NEW.source_registering_provider_digest
     AND candidate.logical_adapter_id=NEW.logical_adapter_id
     AND candidate.logical_adapter_binding_digest=NEW.logical_adapter_binding_digest
     AND candidate.logical_projection_compatibility_digest=NEW.logical_projection_compatibility_digest
     AND candidate.service_actor_id=NEW.service_actor_id
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
     AND profile.logical_adapter_id=NEW.logical_adapter_id
     AND profile.service_actor_id=NEW.service_actor_id
     AND profile.launch_policy_digest=NEW.launch_policy_digest
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
     AND target.route_adapter_projection_id=NEW.route_adapter_projection_id
     AND target.provider_id=NEW.provider_id
     AND target.provider_owner_account_id=NEW.provider_owner_account_id
     AND target.logical_adapter_id=NEW.logical_adapter_id
     AND target.service_actor_id=NEW.service_actor_id
     AND target.launch_policy_digest=NEW.launch_policy_digest
     AND target.target_policy_digest=NEW.target_policy_digest
     AND companion.profile_id=NEW.profile_id
     AND companion.profile_digest=NEW.profile_digest
     AND companion.candidate_id=NEW.candidate_id
     AND companion.candidate_digest=NEW.candidate_digest
     AND companion.delegation_id=NEW.delegation_id
     AND companion.delegation_digest=NEW.delegation_digest
     AND companion.provider_binding_id=NEW.provider_binding_id
     AND companion.provider_binding_digest=NEW.provider_binding_digest
     AND companion.registry_release_id=NEW.registry_release_id
     AND companion.registry_release_digest=NEW.registry_release_digest
     AND companion.route_adapter_projection_id=NEW.route_adapter_projection_id
     AND companion.provider_id=NEW.provider_id
     AND companion.provider_owner_account_id=NEW.provider_owner_account_id
     AND companion.logical_adapter_id=NEW.logical_adapter_id
     AND companion.service_actor_id=NEW.service_actor_id
     AND companion.launch_policy_digest=NEW.launch_policy_digest
     AND companion.target_id=NEW.target_id
     AND companion.target_digest=NEW.target_digest
     AND companion.target_policy_digest=NEW.target_policy_digest
     AND companion.supervisor_session_policy_digest=NEW.supervisor_session_policy_digest
     AND companion.entrypoint_capsule_policy_digest=NEW.entrypoint_capsule_policy_digest
     AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_release_admission_terminal_receipts terminal WHERE terminal.admission_id=release.admission_id)
     AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_installation_terminal_receipts terminal WHERE terminal.installation_receipt_id=binding.installation_receipt_id AND terminal.installation_receipt_digest=binding.installation_receipt_digest)
     AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_adoption_terminal_receipts terminal WHERE terminal.adoption_receipt_id=binding.adoption_receipt_id AND terminal.adoption_receipt_digest=binding.adoption_receipt_digest)
     AND NOT EXISTS (SELECT 1 FROM compute_external_pool_provider_activation_delegations successor WHERE successor.predecessor_delegation_id=delegation.delegation_id)
     AND NOT EXISTS (SELECT 1 FROM compute_external_pool_provider_activation_candidates successor WHERE successor.predecessor_candidate_id=candidate.candidate_id)
     AND NOT EXISTS (SELECT 1 FROM compute_external_pool_provider_activation_delegation_revocations revoked WHERE revoked.delegation_id=delegation.delegation_id OR revoked.candidate_id=candidate.candidate_id)
     AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_runtime_launch_profiles successor WHERE successor.predecessor_profile_id=profile.profile_id)
     AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_runtime_launch_profile_revocations revoked WHERE revoked.profile_id=profile.profile_id)
     AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_upstream_transport_targets successor WHERE successor.predecessor_target_id=target.target_id)
     AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_upstream_transport_target_revocations revoked WHERE revoked.target_id=target.target_id)
     AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_supervisor_session_policy_companions successor WHERE successor.predecessor_companion_id=companion.companion_id)
     AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_supervisor_session_policy_companion_revocations revoked WHERE revoked.companion_id=companion.companion_id))
BEGIN SELECT RAISE(ABORT,'V274 active successor lacks exact structural activation roots'); END;
