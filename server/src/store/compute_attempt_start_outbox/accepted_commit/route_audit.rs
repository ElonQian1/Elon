use anyhow::{anyhow, ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::route_authority::{
    canonical_route_adapter_version_json_and_digest,
    canonical_route_authorization_seal_json_and_digest, canonical_route_capability_set_digest,
    canonical_route_credential_json_and_digest, ComputeRouteAdapterVersionEnvelope,
    ComputeRouteAuthorizationEnvelope, ComputeRouteAuthorizationSealEnvelope,
    ComputeRouteCapabilityRevision, ComputeRouteCredentialEnvelope,
    COMPUTE_PROVIDER_KIND_EXTERNAL_POOL, COMPUTE_ROUTE_ADAPTER_STATUS_ACTIVE,
    COMPUTE_ROUTE_KIND_PROVIDER_ENDPOINT, COMPUTE_ROUTE_KIND_SERVER_ADAPTER,
    COMPUTE_ROUTE_REQUIRED_CAPABILITY_COUNT, COMPUTE_ROUTE_SOURCE_EXTERNAL_POOL_ONBOARDING,
};

pub(super) fn audit_immutable_on(
    connection: &Connection,
    route: &ComputeRouteAuthorizationEnvelope,
) -> Result<()> {
    let authorization = &route.authorization;
    let row = connection
        .query_row(
            "SELECT adapter.adapter_json, adapter.adapter_digest,
                    credential.credential_json, credential.credential_digest,
                    seal.seal_json, seal.seal_digest
               FROM compute_route_adapter_versions adapter
               JOIN compute_route_credential_versions credential
                 ON credential.credential_id=?4 AND credential.credential_revision=?5
               JOIN compute_route_authorization_seals seal
                 ON seal.route_authorization_id=?7 AND seal.route_authorization_digest=?8
              WHERE adapter.adapter_id=?1 AND adapter.adapter_revision=?2
                AND adapter.adapter_digest=?3 AND credential.credential_digest=?6",
            params![
                authorization.route.adapter.adapter_id,
                authorization.route.adapter.adapter_revision,
                authorization.route.adapter.adapter_registry_digest,
                authorization.credential.credential_id,
                authorization.credential.credential_revision,
                authorization.credential.credential_digest,
                route.route_authorization_id,
                route.route_authorization_digest,
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("accepted closure immutable route artifacts are missing"))?;
    let adapter: ComputeRouteAdapterVersionEnvelope = serde_json::from_str(&row.0)?;
    let credential: ComputeRouteCredentialEnvelope = serde_json::from_str(&row.2)?;
    let seal: ComputeRouteAuthorizationSealEnvelope = serde_json::from_str(&row.4)?;
    let (adapter_json, adapter_digest) = canonical_route_adapter_version_json_and_digest(&adapter)?;
    let (credential_json, credential_digest) =
        canonical_route_credential_json_and_digest(&credential)?;
    let (seal_json, seal_digest) = canonical_route_authorization_seal_json_and_digest(&seal)?;
    let capability_digest = canonical_route_capability_set_digest(&authorization.capabilities)?;
    let route_capabilities = authorization
        .capabilities
        .iter()
        .map(|capability| ComputeRouteCapabilityRevision {
            capability_id: capability.capability_id.clone(),
            capability_revision: capability.capability_revision,
        })
        .collect::<Vec<_>>();
    let route_shape_is_valid = match authorization.route.route_kind.as_str() {
        COMPUTE_ROUTE_KIND_PROVIDER_ENDPOINT => {
            authorization.route.endpoint_id.is_some()
                && authorization.route.endpoint_transport.is_some()
        }
        COMPUTE_ROUTE_KIND_SERVER_ADAPTER => {
            authorization.route.endpoint_id.is_none()
                && authorization.route.endpoint_transport.is_none()
        }
        _ => false,
    };
    let required_capabilities = [
        "authenticated_ack",
        "authenticated_events",
        "cancel_no_start",
        "idempotent_commit",
        "prepare",
        "reconcile",
    ];
    ensure!(
        adapter_json == row.0
            && adapter.adapter_digest == row.1
            && adapter_digest == row.1
            && credential_json == row.2
            && credential.credential_digest == row.3
            && credential_digest == row.3
            && seal_json == row.4
            && seal.seal_digest == row.5
            && seal_digest == row.5,
        "accepted closure immutable route artifacts failed canonical audit"
    );
    ensure!(
        adapter.adapter_id == authorization.route.adapter.adapter_id
            && adapter.adapter_revision == authorization.route.adapter.adapter_revision
            && adapter.adapter_digest == authorization.route.adapter.adapter_registry_digest
            && adapter.adapter.release_version
                == authorization.route.adapter.adapter_release_version
            && adapter.adapter.implementation_digest
                == authorization.route.adapter.implementation_digest
            && adapter.adapter.route_kind == authorization.route.route_kind
            && adapter.adapter.status == COMPUTE_ROUTE_ADAPTER_STATUS_ACTIVE
            && route_shape_is_valid
            && authorization.route.route_binding_digest
                == authorization.route.adapter_binding_digest
            && !authorization.route.adapter.config_digest.is_empty()
            && authorization.route.adapter.config_digest.trim()
                == authorization.route.adapter.config_digest
            && authorization.route.adapter.config_digest.len() <= 512
            && authorization.source.approved_by_user_id
                == authorization.provider.provider_owner_account_id
            && (authorization.provider.provider_kind != COMPUTE_PROVIDER_KIND_EXTERNAL_POOL
                || authorization.source.source_kind
                    == COMPUTE_ROUTE_SOURCE_EXTERNAL_POOL_ONBOARDING)
            && adapter
                .adapter
                .supported_provider_kinds
                .contains(&authorization.provider.provider_kind)
            && route_capabilities
                .iter()
                .all(|capability| adapter.adapter.supported_capabilities.contains(capability))
            && adapter.adapter.registered_by_service_actor_id
                == authorization.verified_by_service_actor_id
            && adapter.adapter.actor_authorization_id == authorization.actor_authorization_id
            && adapter.adapter.actor_authorization_digest
                == authorization.actor_authorization_digest
            && credential.credential_id == authorization.credential.credential_id
            && credential.credential_revision == authorization.credential.credential_revision
            && credential.credential_digest == authorization.credential.credential_digest
            && credential.credential.provider == authorization.provider
            && credential.credential.route == authorization.route
            && credential.credential.verifier == authorization.verifier
            && credential.credential.verifier == adapter.adapter.credential_verifier
            && credential.credential.expires_at == authorization.credential.expires_at
            && credential.credential.cleanup_expires_at
                == authorization.credential.cleanup_expires_at
            && credential.credential.verified_by_service_actor_id
                == authorization.verified_by_service_actor_id
            && credential.credential.actor_authorization_id == authorization.actor_authorization_id
            && credential.credential.actor_authorization_digest
                == authorization.actor_authorization_digest
            && !credential
                .credential
                .non_bearer_credential_ref
                .trim()
                .is_empty()
            && seal.route_authorization_id == route.route_authorization_id
            && seal.route_authorization_revision == route.route_authorization_revision
            && seal.route_authorization_digest == route.route_authorization_digest
            && seal.adapter_id == authorization.route.adapter.adapter_id
            && seal.adapter_revision == authorization.route.adapter.adapter_revision
            && seal.adapter_registry_digest == authorization.route.adapter.adapter_registry_digest
            && seal.credential_id == authorization.credential.credential_id
            && seal.credential_revision == authorization.credential.credential_revision
            && seal.credential_digest == authorization.credential.credential_digest
            && authorization.capabilities.len() as i64 == COMPUTE_ROUTE_REQUIRED_CAPABILITY_COUNT
            && authorization
                .capabilities
                .iter()
                .enumerate()
                .all(|(ordinal, capability)| capability.ordinal == ordinal as i64
                    && capability.capability_id == required_capabilities[ordinal])
            && seal.capability_count == i64::try_from(authorization.capabilities.len())?
            && seal.capability_set_digest == capability_digest,
        "accepted closure immutable route artifacts conflict with sealed authority"
    );
    Ok(())
}
