CREATE TRIGGER IF NOT EXISTS v277_atomic_activation_receipt_exact_roots
BEFORE INSERT ON compute_external_pool_adapter_atomic_activation_receipts
WHEN NOT EXISTS (
  SELECT 1
    FROM compute_external_pool_adapter_registry_provider_bindings binding
    JOIN compute_external_pool_adapter_registry_releases release
      ON release.registry_release_id=binding.registry_release_id
     AND release.registry_release_digest=binding.registry_release_digest
    JOIN compute_providers provider
      ON provider.provider_id=NEW.target_active_provider_id
     AND provider.provider_kind='external_pool'
     AND provider.owner_account_id=NEW.activated_by_actor_user_id
     AND provider.status='active'
     AND provider.current_policy_revision=NEW.target_active_provider_policy_revision
     AND provider.current_provider_digest=NEW.target_active_provider_digest
    JOIN compute_provider_versions source_version
      ON source_version.provider_id=NEW.source_registering_provider_id
     AND source_version.policy_revision=NEW.source_registering_provider_policy_revision
     AND source_version.provider_digest=NEW.source_registering_provider_digest
     AND source_version.provider_json=NEW.source_registering_provider_json
    JOIN compute_provider_versions target_version
      ON target_version.provider_id=NEW.target_active_provider_id
     AND target_version.policy_revision=NEW.target_active_provider_policy_revision
     AND target_version.provider_digest=NEW.target_active_provider_digest
     AND target_version.provider_json=NEW.target_active_provider_json
    JOIN compute_external_pool_adapter_credential_reattestation_receipts reattestation
      ON reattestation.reattestation_receipt_id=NEW.registering_reattestation_receipt_id
     AND reattestation.reattestation_receipt_digest=NEW.registering_reattestation_receipt_digest
    JOIN compute_route_adapters adapter_root
      ON adapter_root.adapter_id=NEW.route_adapter_projection_id
     AND adapter_root.current_adapter_revision=NEW.route_adapter_revision
     AND adapter_root.current_adapter_digest=NEW.route_adapter_digest
     AND adapter_root.status='active'
    JOIN compute_route_adapter_versions adapter_version
      ON adapter_version.adapter_id=NEW.route_adapter_projection_id
     AND adapter_version.adapter_revision=NEW.route_adapter_revision
     AND adapter_version.adapter_digest=NEW.route_adapter_digest
     AND adapter_version.status='active'
    JOIN compute_service_actor_authorizations actor
      ON actor.actor_authorization_id=NEW.service_actor_authorization_id
     AND actor.actor_authorization_digest=NEW.service_actor_authorization_digest
     AND actor.provider_id=NEW.target_active_provider_id
     AND actor.provider_owner_account_id=NEW.activated_by_actor_user_id
     AND actor.service_actor_id=NEW.service_actor_id
     AND actor.service_actor_kind='platform_dispatch_service'
    JOIN compute_route_credentials credential_root
      ON credential_root.credential_id=NEW.route_credential_id
     AND credential_root.current_credential_revision=NEW.route_credential_revision
     AND credential_root.current_credential_digest=NEW.route_credential_digest
     AND credential_root.status='active'
    JOIN compute_route_credential_versions credential
      ON credential.credential_id=NEW.route_credential_id
     AND credential.credential_revision=NEW.route_credential_revision
     AND credential.credential_digest=NEW.route_credential_digest
     AND credential.provider_id=NEW.target_active_provider_id
     AND credential.adapter_id=NEW.route_adapter_projection_id
     AND credential.adapter_revision=NEW.route_adapter_revision
     AND credential.adapter_binding_digest=NEW.projected_v211_adapter_binding_digest
    JOIN compute_route_authorization_receipts route
      ON route.route_authorization_id=NEW.route_authorization_id
     AND route.route_authorization_revision=NEW.route_authorization_revision
     AND route.route_authorization_digest=NEW.route_authorization_digest
     AND route.provider_id=NEW.target_active_provider_id
     AND route.executor_id=NEW.executor_id
     AND route.adapter_id=NEW.route_adapter_projection_id
     AND route.adapter_revision=NEW.route_adapter_revision
     AND route.adapter_binding_digest=NEW.projected_v211_adapter_binding_digest
     AND route.route_binding_digest=NEW.projected_v211_adapter_binding_digest
     AND route.credential_id=NEW.route_credential_id
     AND route.credential_revision=NEW.route_credential_revision
     AND route.credential_digest=NEW.route_credential_digest
     AND route.capability_count=NEW.route_capability_count
     AND route.capability_set_digest=NEW.route_capability_set_digest
    JOIN compute_route_authorization_seals seal
      ON seal.route_authorization_id=NEW.route_authorization_id
     AND seal.route_authorization_revision=NEW.route_authorization_revision
     AND seal.route_authorization_digest=NEW.route_authorization_digest
     AND seal.seal_id=NEW.route_seal_id
     AND seal.seal_digest=NEW.route_seal_digest
     AND seal.capability_set_digest=NEW.route_capability_set_digest
    JOIN compute_external_pool_adapter_task_protocol_conformance_run_receipts protocol
      ON protocol.run_receipt_id=NEW.task_protocol_conformance_run_receipt_id
     AND protocol.run_receipt_digest=NEW.task_protocol_conformance_run_receipt_digest
     AND protocol.expires_at=NEW.task_protocol_conformance_expires_at
   WHERE binding.provider_binding_id=NEW.provider_binding_id
     AND binding.provider_binding_digest=NEW.provider_binding_digest
     AND binding.provider_id=NEW.target_active_provider_id
     AND binding.provider_owner_account_id=NEW.activated_by_actor_user_id
     AND binding.route_adapter_projection_id=NEW.route_adapter_projection_id
     AND protocol.registry_release_id=binding.registry_release_id
     AND protocol.registry_release_digest=binding.registry_release_digest
     AND protocol.registry_release_material_digest=release.registry_release_material_digest
     AND protocol.adapter_id=binding.adapter_id
     AND protocol.release_version=binding.release_version
     AND reattestation.provider_binding_id=NEW.provider_binding_id
     AND reattestation.provider_binding_digest=NEW.provider_binding_digest
     AND reattestation.provider_id=NEW.source_registering_provider_id
     AND reattestation.observed_provider_policy_revision=NEW.source_registering_provider_policy_revision
     AND reattestation.observed_provider_digest=NEW.source_registering_provider_digest
     AND reattestation.observed_provider_status='registering'
     AND json_extract(target_version.provider_json,'$.adapter.adapter_id')=NEW.route_adapter_projection_id
     AND json_extract(target_version.provider_json,'$.updated_at')=NEW.activation_target_updated_at
     AND NEW.route_capability_count=(
       SELECT COUNT(*) FROM compute_route_authorization_capabilities capability
        WHERE capability.route_authorization_id=NEW.route_authorization_id)
     AND NOT EXISTS (
       SELECT 1 FROM json_each(NEW.route_capabilities_json) expected
        WHERE NOT EXISTS (
          SELECT 1 FROM compute_route_authorization_capabilities actual
           WHERE actual.route_authorization_id=NEW.route_authorization_id
             AND actual.ordinal=json_extract(expected.value,'$.ordinal')
             AND actual.capability_id=json_extract(expected.value,'$.capability_id')
             AND actual.capability_revision=json_extract(expected.value,'$.capability_revision')))
     AND NOT EXISTS (
       SELECT 1 FROM compute_external_pool_adapter_installation_terminal_receipts terminal
        WHERE terminal.installation_receipt_id=binding.installation_receipt_id
          AND terminal.installation_receipt_digest=binding.installation_receipt_digest)
     AND NOT EXISTS (
       SELECT 1 FROM compute_external_pool_adapter_adoption_terminal_receipts terminal
        WHERE terminal.adoption_receipt_id=binding.adoption_receipt_id
          AND terminal.adoption_receipt_digest=binding.adoption_receipt_digest)
)
BEGIN SELECT RAISE(ABORT,'V277 atomic activation lacks exact committed Provider/route roots'); END;
