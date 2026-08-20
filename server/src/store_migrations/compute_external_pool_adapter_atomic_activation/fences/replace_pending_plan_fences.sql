DROP TRIGGER IF EXISTS v254_external_pool_provider_activation_fence;
CREATE TRIGGER v254_external_pool_provider_activation_fence
BEFORE UPDATE ON compute_providers
WHEN NEW.provider_kind='external_pool'
 AND NEW.status='active'
 AND elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches(
       'provider_update',
       OLD.provider_id,OLD.provider_kind,OLD.owner_account_id,OLD.settlement_account_id,
       OLD.display_name,OLD.status,OLD.trust_tier,OLD.home_region,
       OLD.current_policy_revision,OLD.current_provider_digest,OLD.created_at,OLD.updated_at,
       NEW.provider_id,NEW.provider_kind,NEW.owner_account_id,NEW.settlement_account_id,
       NEW.display_name,NEW.status,NEW.trust_tier,NEW.home_region,
       NEW.current_policy_revision,NEW.current_provider_digest,NEW.created_at,NEW.updated_at
     ) IS NOT 1
BEGIN SELECT RAISE(ABORT,'V277 external_pool Provider activation lacks exact pending plan'); END;

DROP TRIGGER IF EXISTS v254_external_pool_provider_version_active_fence;
CREATE TRIGGER v254_external_pool_provider_version_active_fence
BEFORE INSERT ON compute_provider_versions
WHEN json_extract(NEW.provider_json,'$.status')='active'
 AND EXISTS (
       SELECT 1 FROM compute_providers provider
        WHERE provider.provider_id=NEW.provider_id
          AND provider.provider_kind='external_pool'
     )
 AND elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches(
       'provider_version',NEW.provider_id,NEW.policy_revision,NEW.provider_digest,
       NEW.provider_json,NEW.created_at
     ) IS NOT 1
BEGIN SELECT RAISE(ABORT,'V277 external_pool active Provider version lacks exact pending plan'); END;

DROP TRIGGER IF EXISTS v254_external_pool_candidate_projection_adapter_fence;
CREATE TRIGGER v254_external_pool_candidate_projection_adapter_fence
BEFORE INSERT ON compute_route_adapters
WHEN EXISTS (
       SELECT 1 FROM compute_external_pool_adapter_registry_provider_bindings binding
        WHERE binding.route_adapter_projection_id=NEW.adapter_id
     )
 AND elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches(
       'projection_adapter',NEW.adapter_id,NEW.current_adapter_revision,
       NEW.current_adapter_digest,NEW.status,NEW.created_at,NEW.updated_at
     ) IS NOT 1
BEGIN SELECT RAISE(ABORT,'V277 projection Adapter lacks exact pending plan'); END;

DROP TRIGGER IF EXISTS v254_external_pool_candidate_projection_adapter_version_fence;
CREATE TRIGGER v254_external_pool_candidate_projection_adapter_version_fence
BEFORE INSERT ON compute_route_adapter_versions
WHEN EXISTS (
       SELECT 1 FROM compute_external_pool_adapter_registry_provider_bindings binding
        WHERE binding.route_adapter_projection_id=NEW.adapter_id
     )
 AND elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches(
       'projection_adapter_version',NEW.adapter_id,NEW.adapter_revision,
       NEW.adapter_digest,NEW.adapter_json
     ) IS NOT 1
BEGIN SELECT RAISE(ABORT,'V277 projection Adapter version lacks exact pending plan'); END;

DROP TRIGGER IF EXISTS v254_external_pool_candidate_service_actor_fence;
CREATE TRIGGER v254_external_pool_candidate_service_actor_fence
BEFORE INSERT ON compute_service_actor_authorizations
WHEN NEW.service_actor_kind='platform_dispatch_service'
 AND (
       EXISTS (
         SELECT 1 FROM compute_external_pool_provider_activation_candidates candidate
          WHERE candidate.service_actor_id=NEW.service_actor_id
             OR candidate.provider_id=NEW.provider_id
       )
       OR EXISTS (
         SELECT 1 FROM compute_providers provider
          WHERE provider.provider_id=NEW.provider_id
            AND provider.provider_kind='external_pool'
       )
     )
 AND elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches(
       'service_actor_authorization',NEW.actor_authorization_id,
       NEW.actor_authorization_digest,NEW.actor_authorization_json
     ) IS NOT 1
BEGIN SELECT RAISE(ABORT,'V277 service actor authorization lacks exact pending plan'); END;

DROP TRIGGER IF EXISTS v254_external_pool_route_credential_fence;
CREATE TRIGGER v254_external_pool_route_credential_fence
BEFORE INSERT ON compute_route_credential_versions
WHEN (
       NEW.provider_kind='external_pool'
       OR EXISTS (
         SELECT 1 FROM compute_providers provider
          WHERE provider.provider_id=NEW.provider_id
            AND provider.provider_kind='external_pool'
       )
     )
 AND elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches(
       'route_credential',NEW.credential_id,NEW.credential_revision,
       NEW.credential_digest,NEW.credential_json
     ) IS NOT 1
BEGIN SELECT RAISE(ABORT,'V277 external_pool route credential lacks exact pending plan'); END;

DROP TRIGGER IF EXISTS v254_external_pool_route_authorization_fence;
CREATE TRIGGER v254_external_pool_route_authorization_fence
BEFORE INSERT ON compute_route_authorization_receipts
WHEN (
       NEW.provider_kind='external_pool'
       OR EXISTS (
         SELECT 1 FROM compute_providers provider
          WHERE provider.provider_id=NEW.provider_id
            AND provider.provider_kind='external_pool'
       )
     )
 AND elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches(
       'route_authorization',NEW.route_authorization_id,NEW.route_authorization_revision,
       NEW.route_authorization_digest,NEW.route_authorization_json
     ) IS NOT 1
BEGIN SELECT RAISE(ABORT,'V277 external_pool route authorization lacks exact pending plan'); END;

DROP TRIGGER IF EXISTS v254_external_pool_route_capability_fence;
CREATE TRIGGER v254_external_pool_route_capability_fence
BEFORE INSERT ON compute_route_authorization_capabilities
WHEN EXISTS (
       SELECT 1
         FROM compute_route_authorization_receipts route
         JOIN compute_providers provider ON provider.provider_id=route.provider_id
        WHERE route.route_authorization_id=NEW.route_authorization_id
          AND provider.provider_kind='external_pool'
     )
 AND elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches(
       'route_capability',NEW.route_authorization_id,NEW.ordinal,
       NEW.capability_id,NEW.capability_revision
     ) IS NOT 1
BEGIN SELECT RAISE(ABORT,'V277 external_pool route capability lacks exact pending plan'); END;

DROP TRIGGER IF EXISTS v254_external_pool_route_seal_fence;
CREATE TRIGGER v254_external_pool_route_seal_fence
BEFORE INSERT ON compute_route_authorization_seals
WHEN EXISTS (
       SELECT 1
         FROM compute_route_authorization_receipts route
         JOIN compute_providers provider ON provider.provider_id=route.provider_id
        WHERE route.route_authorization_id=NEW.route_authorization_id
          AND provider.provider_kind='external_pool'
     )
 AND elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches(
       'route_seal',NEW.seal_id,NEW.seal_digest,NEW.seal_json
     ) IS NOT 1
BEGIN SELECT RAISE(ABORT,'V277 external_pool route seal lacks exact pending plan'); END;
