//! Historical exactness and live-currentness audit for the V277 route closure.

use anyhow::{ensure, Result};
use rusqlite::{named_params, params, Connection};

use crate::compute_federation::external_pool_adapter_atomic_activation::ExternalPoolAdapterAtomicActivationReceipt;

pub(super) fn audit_historical_route(
    connection: &Connection,
    receipt: &ExternalPoolAdapterAtomicActivationReceipt,
) -> Result<()> {
    let route = &receipt.activation.route_closure;
    let target = &receipt
        .activation
        .provider_transition
        .target_active_provider;
    let projected = &receipt.activation.projected_v211_binding;
    let executor = &receipt.activation.stable_executor;
    let audit = &receipt.activation.audit;
    let exact: bool = connection.query_row(
        "SELECT EXISTS(
             SELECT 1
               FROM compute_route_adapter_versions adapter
               JOIN compute_service_actor_authorizations actor
                 ON actor.actor_authorization_id=:actor_id
               JOIN compute_route_credential_versions credential
                 ON credential.credential_id=:credential_id
                AND credential.credential_revision=:credential_revision
               JOIN compute_route_authorization_receipts authorization
                 ON authorization.route_authorization_id=:authorization_id
               JOIN compute_route_authorization_seals seal ON seal.seal_id=:seal_id
              WHERE adapter.adapter_id=:adapter_id
                AND adapter.adapter_revision=:adapter_revision
                AND adapter.adapter_digest=:adapter_digest AND adapter.status='active'
                AND adapter.registered_by_service_actor_id=:service_actor_id
                AND adapter.actor_authorization_id=:actor_id
                AND adapter.actor_authorization_digest=:actor_digest
                AND actor.actor_authorization_digest=:actor_digest
                AND actor.provider_id=:provider_id AND actor.provider_owner_account_id=:owner_id
                AND actor.service_actor_id=:service_actor_id
                AND actor.service_actor_kind='platform_dispatch_service'
                AND credential.credential_digest=:credential_digest
                AND credential.provider_id=:provider_id AND credential.provider_kind='external_pool'
                AND credential.provider_owner_account_id=:owner_id
                AND credential.adapter_id=:adapter_id
                AND credential.adapter_revision=:adapter_revision
                AND credential.adapter_binding_digest=:projected_binding_digest
                AND credential.route_binding_digest=:projected_binding_digest
                AND credential.verified_by_service_actor_id=:service_actor_id
                AND credential.actor_authorization_id=:actor_id
                AND credential.actor_authorization_digest=:actor_digest
                AND authorization.route_authorization_revision=:authorization_revision
                AND authorization.route_authorization_digest=:authorization_digest
                AND authorization.provider_id=:provider_id
                AND authorization.provider_kind='external_pool'
                AND authorization.provider_owner_account_id=:owner_id
                AND authorization.executor_id=:executor_id
                AND authorization.adapter_id=:adapter_id
                AND authorization.adapter_revision=:adapter_revision
                AND authorization.adapter_binding_digest=:projected_binding_digest
                AND authorization.route_binding_digest=:projected_binding_digest
                AND authorization.credential_id=:credential_id
                AND authorization.credential_revision=:credential_revision
                AND authorization.credential_digest=:credential_digest
                AND authorization.capability_count=:capability_count
                AND authorization.capability_set_digest=:capability_set_digest
                AND authorization.verified_by_service_actor_id=:service_actor_id
                AND authorization.actor_authorization_id=:actor_id
                AND authorization.actor_authorization_digest=:actor_digest
                AND seal.route_authorization_id=:authorization_id
                AND seal.route_authorization_revision=:authorization_revision
                AND seal.route_authorization_digest=:authorization_digest
                AND seal.seal_digest=:seal_digest
                AND seal.adapter_id=:adapter_id AND seal.adapter_revision=:adapter_revision
                AND seal.adapter_registry_digest=:adapter_digest
                AND seal.credential_id=:credential_id
                AND seal.credential_revision=:credential_revision
                AND seal.credential_digest=:credential_digest
                AND seal.capability_count=:capability_count
                AND seal.capability_set_digest=:capability_set_digest
         )",
        named_params! {
            ":adapter_id": route.route_adapter_projection_id,
            ":adapter_revision": route.route_adapter_revision,
            ":adapter_digest": route.route_adapter_digest,
            ":actor_id": route.service_actor_authorization_id,
            ":actor_digest": route.service_actor_authorization_digest,
            ":service_actor_id": route.service_actor_id,
            ":provider_id": target.provider_id,
            ":owner_id": audit.activated_by_actor_user_id,
            ":credential_id": route.route_credential_id,
            ":credential_revision": route.route_credential_revision,
            ":credential_digest": route.route_credential_digest,
            ":authorization_id": route.route_authorization_id,
            ":authorization_revision": route.route_authorization_revision,
            ":authorization_digest": route.route_authorization_digest,
            ":projected_binding_digest": projected.projected_v211_adapter_binding_digest,
            ":executor_id": executor.executor_id,
            ":capability_count": route.route_capability_count,
            ":capability_set_digest": route.route_capability_set_digest,
            ":seal_id": route.route_seal_id,
            ":seal_digest": route.route_seal_digest,
        },
        |row| row.get(0),
    )?;
    ensure!(exact, "V277 immutable route closure is not exact");
    audit_route_capabilities(connection, receipt)
}

pub(super) fn audit_live_route(
    connection: &Connection,
    receipt: &ExternalPoolAdapterAtomicActivationReceipt,
    checked_at: Option<&str>,
) -> Result<()> {
    let route = &receipt.activation.route_closure;
    audit_historical_route(connection, receipt)?;
    let current: bool = connection.query_row(
        "SELECT EXISTS(
             SELECT 1
               FROM compute_route_adapters adapter
               JOIN compute_route_credentials credential
                 ON credential.credential_id=:credential_id
               JOIN compute_route_credential_versions credential_version
                 ON credential_version.credential_id=:credential_id
                AND credential_version.credential_revision=:credential_revision
               JOIN compute_route_authorization_receipts authorization
                 ON authorization.route_authorization_id=:authorization_id
               JOIN compute_service_actor_authorizations actor
                 ON actor.actor_authorization_id=:actor_id
              WHERE adapter.adapter_id=:adapter_id
                AND adapter.current_adapter_revision=:adapter_revision
                AND adapter.current_adapter_digest=:adapter_digest
                 AND adapter.status='active'
                 AND credential.current_credential_revision=:credential_revision
                 AND credential.current_credential_digest=:credential_digest
                 AND credential.status='active'
                 AND NOT EXISTS (
                     SELECT 1 FROM compute_route_credential_revocations revoked
                      WHERE revoked.credential_id=:credential_id
                        AND revoked.credential_revision=:credential_revision)
                 AND (:checked_at IS NULL OR (
                     :checked_at>=actor.issued_at AND :checked_at<actor.valid_until
                     AND :checked_at>=credential_version.authenticated_at
                     AND :checked_at<credential_version.expires_at
                     AND :checked_at>=authorization.authorized_at
                     AND :checked_at>=authorization.recorded_at
                     AND :checked_at<authorization.expires_at
                     AND :checked_at<authorization.credential_expires_at))
         )",
        named_params! {
            ":adapter_id": route.route_adapter_projection_id,
            ":adapter_revision": route.route_adapter_revision,
            ":adapter_digest": route.route_adapter_digest,
            ":credential_id": route.route_credential_id,
            ":credential_revision": route.route_credential_revision,
            ":credential_digest": route.route_credential_digest,
            ":authorization_id": route.route_authorization_id,
            ":actor_id": route.service_actor_authorization_id,
            ":checked_at": checked_at,
        },
        |row| row.get(0),
    )?;
    ensure!(
        current,
        "V277 projected route closure is not live and exact"
    );
    Ok(())
}

fn audit_route_capabilities(
    connection: &Connection,
    receipt: &ExternalPoolAdapterAtomicActivationReceipt,
) -> Result<()> {
    let route = &receipt.activation.route_closure;
    let mut statement = connection.prepare(
        "SELECT ordinal,capability_id,capability_revision
           FROM compute_route_authorization_capabilities
          WHERE route_authorization_id=?1 ORDER BY ordinal",
    )?;
    let stored = statement
        .query_map(params![route.route_authorization_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let expected = route
        .capabilities
        .iter()
        .map(|capability| {
            (
                capability.ordinal,
                capability.capability_id.clone(),
                capability.capability_revision,
            )
        })
        .collect::<Vec<_>>();
    ensure!(stored == expected, "V277 route capability rows drifted");
    Ok(())
}
