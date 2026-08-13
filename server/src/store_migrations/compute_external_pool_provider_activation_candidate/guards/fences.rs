use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS v254_external_pool_provider_activation_fence
        BEFORE UPDATE ON compute_providers
        WHEN NEW.provider_kind='external_pool' AND NEW.status='active'
        BEGIN SELECT RAISE(ABORT,'V254 external_pool Provider activation closure is not implemented'); END;

        CREATE TRIGGER IF NOT EXISTS v254_external_pool_provider_insert_active_fence
        BEFORE INSERT ON compute_providers
        WHEN NEW.provider_kind='external_pool'
          AND (
            NEW.status='active'
            OR EXISTS (
                 SELECT 1 FROM compute_capacity_pools pool
                  WHERE pool.provider_id=NEW.provider_id AND pool.status='active'
            )
            OR EXISTS (
                 SELECT 1 FROM compute_provider_versions version
                  WHERE version.provider_id=NEW.provider_id
                    AND json_extract(version.provider_json,'$.status')='active'
            )
            OR EXISTS (
                 SELECT 1 FROM compute_offers offer
                  WHERE offer.provider_id=NEW.provider_id
                    AND offer.status IN ('draft','active')
            )
            OR EXISTS (
                 SELECT 1
                   FROM compute_offer_versions version
                   LEFT JOIN compute_offers offer ON offer.offer_id=version.offer_id
                  WHERE (version.status IN ('draft','active')
                         OR json_extract(version.offer_json,'$.status') IN ('draft','active'))
                    AND (version.provider_id=NEW.provider_id
                         OR json_extract(version.offer_json,'$.provider_id')=NEW.provider_id
                         OR offer.provider_id=NEW.provider_id)
            )
            OR EXISTS (
                 SELECT 1 FROM compute_service_actor_authorizations actor
                  WHERE actor.provider_id=NEW.provider_id
                    AND actor.service_actor_kind='platform_dispatch_service'
            )
            OR EXISTS (
                 SELECT 1 FROM compute_route_credential_versions credential
                  WHERE credential.provider_id=NEW.provider_id
            )
            OR EXISTS (
                 SELECT 1 FROM compute_route_authorization_receipts route
                  WHERE route.provider_id=NEW.provider_id
            )
          )
        BEGIN SELECT RAISE(ABORT,'V254 external_pool Provider activation closure is not implemented'); END;

        CREATE TRIGGER IF NOT EXISTS v254_external_pool_provider_identity_update_fence
        BEFORE UPDATE ON compute_providers
        WHEN OLD.provider_id IS NOT NEW.provider_id
          AND (OLD.provider_kind='external_pool' OR NEW.provider_kind='external_pool')
        BEGIN SELECT RAISE(ABORT,'V254 external_pool Provider identity is immutable'); END;

        CREATE TRIGGER IF NOT EXISTS v254_external_pool_provider_kind_update_fence
        BEFORE UPDATE ON compute_providers
        WHEN OLD.provider_kind IS NOT NEW.provider_kind
          AND (OLD.provider_kind='external_pool' OR NEW.provider_kind='external_pool')
        BEGIN SELECT RAISE(ABORT,'V254 external_pool Provider kind conflicts with preexisting authority'); END;

        CREATE TRIGGER IF NOT EXISTS v254_external_pool_provider_version_active_fence
        BEFORE INSERT ON compute_provider_versions
        WHEN json_extract(NEW.provider_json,'$.status')='active'
          AND EXISTS (
                SELECT 1 FROM compute_providers provider
                 WHERE provider.provider_id=NEW.provider_id
                   AND provider.provider_kind='external_pool'
          )
        BEGIN SELECT RAISE(ABORT,'V254 external_pool active Provider version is not issuable'); END;

        CREATE TRIGGER IF NOT EXISTS v254_external_pool_candidate_projection_adapter_fence
        BEFORE INSERT ON compute_route_adapters
        WHEN EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_registry_provider_bindings binding
           WHERE binding.route_adapter_projection_id=NEW.adapter_id
        )
        BEGIN SELECT RAISE(ABORT,'V254 candidate route projection is reserved and inert'); END;

        CREATE TRIGGER IF NOT EXISTS v254_external_pool_candidate_projection_adapter_version_fence
        BEFORE INSERT ON compute_route_adapter_versions
        WHEN EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_registry_provider_bindings binding
           WHERE binding.route_adapter_projection_id=NEW.adapter_id
        )
        BEGIN SELECT RAISE(ABORT,'V254 candidate route projection is reserved and inert'); END;

        CREATE TRIGGER IF NOT EXISTS v254_external_pool_candidate_service_actor_fence
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
        BEGIN SELECT RAISE(ABORT,'V254 candidate service actor authorization is not issuable'); END;

        CREATE TRIGGER IF NOT EXISTS v254_external_pool_route_credential_fence
        BEFORE INSERT ON compute_route_credential_versions
        WHEN NEW.provider_kind='external_pool'
          OR EXISTS (
               SELECT 1 FROM compute_providers provider
                WHERE provider.provider_id=NEW.provider_id
                  AND provider.provider_kind='external_pool'
          )
        BEGIN SELECT RAISE(ABORT,'V254 external_pool route credential closure is not implemented'); END;

        CREATE TRIGGER IF NOT EXISTS v254_external_pool_route_authorization_fence
        BEFORE INSERT ON compute_route_authorization_receipts
        WHEN NEW.provider_kind='external_pool'
          OR EXISTS (
               SELECT 1 FROM compute_providers provider
                WHERE provider.provider_id=NEW.provider_id
                  AND provider.provider_kind='external_pool'
          )
        BEGIN SELECT RAISE(ABORT,'V254 external_pool route authorization closure is not implemented'); END;

        CREATE TRIGGER IF NOT EXISTS v254_external_pool_route_capability_fence
        BEFORE INSERT ON compute_route_authorization_capabilities
        WHEN EXISTS (
          SELECT 1
            FROM compute_route_authorization_receipts route
            JOIN compute_providers provider ON provider.provider_id=route.provider_id
           WHERE route.route_authorization_id=NEW.route_authorization_id
             AND provider.provider_kind='external_pool'
        )
        BEGIN SELECT RAISE(ABORT,'V254 external_pool route capability closure is not implemented'); END;

        CREATE TRIGGER IF NOT EXISTS v254_external_pool_route_seal_fence
        BEFORE INSERT ON compute_route_authorization_seals
        WHEN EXISTS (
          SELECT 1
            FROM compute_route_authorization_receipts route
            JOIN compute_providers provider ON provider.provider_id=route.provider_id
           WHERE route.route_authorization_id=NEW.route_authorization_id
             AND provider.provider_kind='external_pool'
        )
        BEGIN SELECT RAISE(ABORT,'V254 external_pool route seal closure is not implemented'); END;

        CREATE TRIGGER IF NOT EXISTS v254_external_pool_capacity_pool_insert_active_fence
        BEFORE INSERT ON compute_capacity_pools
        WHEN NEW.status='active'
          AND EXISTS (
                SELECT 1 FROM compute_providers provider
                 WHERE provider.provider_id=NEW.provider_id
                   AND provider.provider_kind='external_pool'
          )
        BEGIN SELECT RAISE(ABORT,'V254 external_pool market admission is not implemented'); END;

        CREATE TRIGGER IF NOT EXISTS v254_external_pool_capacity_pool_update_active_fence
        BEFORE UPDATE ON compute_capacity_pools
        WHEN NEW.status='active'
          AND EXISTS (
                SELECT 1 FROM compute_providers provider
                 WHERE provider.provider_kind='external_pool'
                   AND provider.provider_id IN (OLD.provider_id,NEW.provider_id)
          )
        BEGIN SELECT RAISE(ABORT,'V254 external_pool market admission is not implemented'); END;

        CREATE TRIGGER IF NOT EXISTS v254_external_pool_capacity_pool_version_active_fence
        BEFORE INSERT ON compute_capacity_pool_versions
        WHEN EXISTS (
          SELECT 1
            FROM compute_capacity_pools pool
            JOIN compute_providers provider ON provider.provider_id=pool.provider_id
           WHERE pool.pool_id=NEW.pool_id
             AND pool.status='active'
             AND provider.provider_kind='external_pool'
        )
        BEGIN SELECT RAISE(ABORT,'V254 external_pool active Pool version is not issuable'); END;

        CREATE TRIGGER IF NOT EXISTS v254_external_pool_offer_insert_market_fence
        BEFORE INSERT ON compute_offers
        WHEN NEW.status IN ('draft','active')
          AND (
            NEW.provider_kind='external_pool'
            OR EXISTS (
                 SELECT 1 FROM compute_providers provider
                  WHERE provider.provider_id=NEW.provider_id
                    AND provider.provider_kind='external_pool'
            )
          )
        BEGIN SELECT RAISE(ABORT,'V254 external_pool market admission is not implemented'); END;

        CREATE TRIGGER IF NOT EXISTS v254_external_pool_offer_update_market_fence
        BEFORE UPDATE ON compute_offers
        WHEN NEW.status IN ('draft','active')
          AND (
            OLD.provider_kind='external_pool'
            OR NEW.provider_kind='external_pool'
            OR EXISTS (
                 SELECT 1 FROM compute_providers provider
                  WHERE provider.provider_kind='external_pool'
                    AND provider.provider_id IN (OLD.provider_id,NEW.provider_id)
            )
          )
        BEGIN SELECT RAISE(ABORT,'V254 external_pool market admission is not implemented'); END;

        CREATE TRIGGER IF NOT EXISTS v254_external_pool_offer_version_market_fence
        BEFORE INSERT ON compute_offer_versions
        WHEN (NEW.status IN ('draft','active')
              OR json_extract(NEW.offer_json,'$.status') IN ('draft','active'))
          AND (
            json_extract(NEW.offer_json,'$.provider_kind')='external_pool'
            OR EXISTS (
                 SELECT 1 FROM compute_providers provider
                  WHERE provider.provider_id=NEW.provider_id
                    AND provider.provider_kind='external_pool'
            )
            OR EXISTS (
                 SELECT 1 FROM compute_offers offer
                  WHERE offer.offer_id=NEW.offer_id
                    AND offer.provider_kind='external_pool'
            )
          )
        BEGIN SELECT RAISE(ABORT,'V254 external_pool Offer version is not issuable'); END;
        "#,
    )?;
    Ok(())
}
