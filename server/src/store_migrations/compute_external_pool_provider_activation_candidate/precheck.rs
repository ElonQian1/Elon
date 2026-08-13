use anyhow::{bail, Result};
use rusqlite::Connection;

pub(super) fn reject_existing_anomalies(conn: &Connection) -> Result<()> {
    reject(
        conn,
        "SELECT EXISTS(SELECT 1 FROM compute_providers WHERE provider_kind='external_pool' AND status='active')",
        "V254 refuses an existing active external_pool Provider",
    )?;
    reject(
        conn,
        "SELECT EXISTS(
           SELECT 1
             FROM compute_provider_versions version
             JOIN compute_providers provider ON provider.provider_id=version.provider_id
            WHERE provider.provider_kind='external_pool'
              AND json_extract(version.provider_json,'$.status')='active'
         )",
        "V254 refuses an existing active external_pool Provider version",
    )?;
    reject(
        conn,
        "SELECT EXISTS(
           SELECT 1
             FROM compute_external_pool_adapter_registry_provider_bindings binding
             JOIN compute_route_adapters adapter
               ON adapter.adapter_id=binding.route_adapter_projection_id
         )",
        "V254 refuses an existing V249 projection in compute_route_adapters",
    )?;
    reject(
        conn,
        "SELECT EXISTS(
           SELECT 1
             FROM compute_external_pool_adapter_registry_provider_bindings binding
             JOIN compute_route_adapter_versions version
               ON version.adapter_id=binding.route_adapter_projection_id
         )",
        "V254 refuses an existing V249 projection in compute_route_adapter_versions",
    )?;
    reject(
        conn,
        "SELECT EXISTS(
           SELECT 1
             FROM compute_providers provider
             JOIN compute_service_actor_authorizations actor
               ON actor.provider_id=provider.provider_id
              AND actor.service_actor_kind='platform_dispatch_service'
            WHERE provider.provider_kind='external_pool'
         )",
        "V254 refuses an existing external_pool platform dispatch actor authority",
    )?;
    reject(
        conn,
        "SELECT EXISTS(
           SELECT 1 FROM compute_route_credential_versions credential
           JOIN compute_providers provider ON provider.provider_id=credential.provider_id
          WHERE provider.provider_kind='external_pool'
         )",
        "V254 refuses an existing external_pool route credential",
    )?;
    reject(
        conn,
        "SELECT EXISTS(
           SELECT 1 FROM compute_route_authorization_receipts route
           JOIN compute_providers provider ON provider.provider_id=route.provider_id
          WHERE provider.provider_kind='external_pool'
        )",
        "V254 refuses an existing external_pool route authorization",
    )?;
    reject(
        conn,
        "SELECT EXISTS(
           SELECT 1 FROM compute_capacity_pools pool
           JOIN compute_providers provider ON provider.provider_id=pool.provider_id
          WHERE pool.status='active' AND provider.provider_kind='external_pool'
         )",
        "V254 refuses an existing active external_pool CapacityPool",
    )?;
    reject(
        conn,
        "SELECT EXISTS(
           SELECT 1 FROM compute_capacity_pool_versions version
           JOIN compute_capacity_pools pool ON pool.pool_id=version.pool_id
           JOIN compute_providers provider ON provider.provider_id=pool.provider_id
          WHERE pool.status='active' AND provider.provider_kind='external_pool'
         )",
        "V254 refuses an existing active external_pool CapacityPool version",
    )?;
    reject(
        conn,
        "SELECT EXISTS(
           SELECT 1 FROM compute_offers offer
           LEFT JOIN compute_providers provider ON provider.provider_id=offer.provider_id
          WHERE offer.status IN ('draft','active')
            AND (offer.provider_kind='external_pool'
                 OR provider.provider_kind='external_pool')
         )",
        "V254 refuses an existing market-admitted external_pool Offer",
    )?;
    reject(
        conn,
        "SELECT EXISTS(
           SELECT 1 FROM compute_offer_versions version
           LEFT JOIN compute_offers offer ON offer.offer_id=version.offer_id
           LEFT JOIN compute_providers provider ON provider.provider_id=version.provider_id
          WHERE (version.status IN ('draft','active')
                 OR json_extract(version.offer_json,'$.status') IN ('draft','active'))
            AND (json_extract(version.offer_json,'$.provider_kind')='external_pool'
                 OR offer.provider_kind='external_pool'
                 OR provider.provider_kind='external_pool')
         )",
        "V254 refuses an existing market-admitted external_pool Offer version",
    )
}

fn reject(conn: &Connection, sql: &str, message: &str) -> Result<()> {
    let found: bool = conn.query_row(sql, [], |row| row.get(0))?;
    if found {
        bail!(message.to_owned());
    }
    Ok(())
}
