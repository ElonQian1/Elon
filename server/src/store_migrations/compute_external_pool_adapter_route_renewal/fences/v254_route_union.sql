DROP TRIGGER IF EXISTS v254_external_pool_candidate_service_actor_fence;
CREATE TRIGGER v254_external_pool_candidate_service_actor_fence
BEFORE INSERT ON compute_service_actor_authorizations
WHEN NEW.service_actor_kind='platform_dispatch_service'
 AND (EXISTS (SELECT 1 FROM compute_external_pool_provider_activation_candidates candidate
              WHERE candidate.service_actor_id=NEW.service_actor_id OR candidate.provider_id=NEW.provider_id)
      OR EXISTS (SELECT 1 FROM compute_providers provider
                  WHERE provider.provider_id=NEW.provider_id AND provider.provider_kind='external_pool'))
 AND CASE
       WHEN elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches(
              'service_actor_authorization',NEW.actor_authorization_id,
              NEW.actor_authorization_digest,NEW.actor_authorization_json) IS 1 THEN 0
       WHEN elon_v278_external_pool_adapter_route_renewal_pending_plan_matches(
              'service_actor_authorization',NEW.actor_authorization_id,
              NEW.actor_authorization_digest,NEW.actor_authorization_json) IS 1 THEN 0
       ELSE 1
     END=1
BEGIN SELECT RAISE(ABORT,'external_pool actor authorization lacks V277/V278 exact plan'); END;

DROP TRIGGER IF EXISTS v254_external_pool_route_credential_fence;
CREATE TRIGGER v254_external_pool_route_credential_fence
BEFORE INSERT ON compute_route_credential_versions
WHEN (NEW.provider_kind='external_pool' OR EXISTS (
       SELECT 1 FROM compute_providers provider
        WHERE provider.provider_id=NEW.provider_id AND provider.provider_kind='external_pool'))
 AND CASE
       WHEN elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches(
              'route_credential',NEW.credential_id,NEW.credential_revision,
              NEW.credential_digest,NEW.credential_json) IS 1 THEN 0
       WHEN elon_v278_external_pool_adapter_route_renewal_pending_plan_matches(
              'route_credential',NEW.credential_id,NEW.credential_revision,
              NEW.credential_digest,NEW.credential_json) IS 1 THEN 0
       ELSE 1
     END=1
BEGIN SELECT RAISE(ABORT,'external_pool route credential lacks V277/V278 exact plan'); END;

DROP TRIGGER IF EXISTS v254_external_pool_route_authorization_fence;
CREATE TRIGGER v254_external_pool_route_authorization_fence
BEFORE INSERT ON compute_route_authorization_receipts
WHEN (NEW.provider_kind='external_pool' OR EXISTS (
       SELECT 1 FROM compute_providers provider
        WHERE provider.provider_id=NEW.provider_id AND provider.provider_kind='external_pool'))
 AND CASE
       WHEN elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches(
              'route_authorization',NEW.route_authorization_id,NEW.route_authorization_revision,
              NEW.route_authorization_digest,NEW.route_authorization_json) IS 1 THEN 0
       WHEN elon_v278_external_pool_adapter_route_renewal_pending_plan_matches(
              'route_authorization',NEW.route_authorization_id,NEW.route_authorization_revision,
              NEW.route_authorization_digest,NEW.route_authorization_json) IS 1 THEN 0
       ELSE 1
     END=1
BEGIN SELECT RAISE(ABORT,'external_pool route authorization lacks V277/V278 exact plan'); END;

DROP TRIGGER IF EXISTS v254_external_pool_route_capability_fence;
CREATE TRIGGER v254_external_pool_route_capability_fence
BEFORE INSERT ON compute_route_authorization_capabilities
WHEN EXISTS (SELECT 1 FROM compute_route_authorization_receipts route
             JOIN compute_providers provider ON provider.provider_id=route.provider_id
              WHERE route.route_authorization_id=NEW.route_authorization_id
                AND provider.provider_kind='external_pool')
 AND CASE
       WHEN elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches(
              'route_capability',NEW.route_authorization_id,NEW.ordinal,
              NEW.capability_id,NEW.capability_revision) IS 1 THEN 0
       WHEN elon_v278_external_pool_adapter_route_renewal_pending_plan_matches(
              'route_capability',NEW.route_authorization_id,NEW.ordinal,
              NEW.capability_id,NEW.capability_revision) IS 1 THEN 0
       ELSE 1
     END=1
BEGIN SELECT RAISE(ABORT,'external_pool route capability lacks V277/V278 exact plan'); END;

DROP TRIGGER IF EXISTS v254_external_pool_route_seal_fence;
CREATE TRIGGER v254_external_pool_route_seal_fence
BEFORE INSERT ON compute_route_authorization_seals
WHEN EXISTS (SELECT 1 FROM compute_route_authorization_receipts route
             JOIN compute_providers provider ON provider.provider_id=route.provider_id
              WHERE route.route_authorization_id=NEW.route_authorization_id
                AND provider.provider_kind='external_pool')
 AND CASE
       WHEN elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches(
              'route_seal',NEW.seal_id,NEW.seal_digest,NEW.seal_json) IS 1 THEN 0
       WHEN elon_v278_external_pool_adapter_route_renewal_pending_plan_matches(
              'route_seal',NEW.seal_id,NEW.seal_digest,NEW.seal_json) IS 1 THEN 0
       ELSE 1
     END=1
BEGIN SELECT RAISE(ABORT,'external_pool route seal lacks V277/V278 exact plan'); END;
