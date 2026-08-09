use anyhow::{anyhow, ensure, Result};
use chrono::{DateTime, SecondsFormat};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::route_authority::{
    canonical_route_adapter_version_json_and_digest,
    canonical_route_authorization_seal_json_and_digest, canonical_route_capability_set_digest,
    canonical_route_credential_json_and_digest, ComputeRouteAdapterVersionEnvelope,
    ComputeRouteAuthorizationSealEnvelope, ComputeRouteCapabilityRevision,
    ComputeRouteCredentialEnvelope, COMPUTE_ACTOR_PHASE_APPLICATION,
    COMPUTE_ROUTE_ADAPTER_STATUS_ACTIVE, COMPUTE_ROUTE_CAPABILITY_IDEMPOTENT_COMMIT,
};

use super::{source::load_base_on, AcceptedStartCommitFreshness};

struct RegistryRows {
    adapter_json: String,
    adapter_digest: String,
    current_adapter_revision: i64,
    current_adapter_digest: String,
    current_adapter_status: String,
    credential_json: String,
    credential_digest: String,
    current_credential_revision: i64,
    current_credential_digest: String,
    current_credential_status: String,
    seal_json: String,
    seal_digest: String,
    provider_kind: String,
    provider_owner_account_id: String,
    provider_status: String,
}

pub(super) fn ensure_fresh_on(
    connection: &Connection,
    command_id: &str,
    checked_at: &str,
) -> Result<AcceptedStartCommitFreshness> {
    parse_canonical_time(checked_at)?;
    let base = load_base_on(connection, command_id)?;
    let start = &base.command.command;
    let plan = &base.plan.plan;
    if checked_at >= base.command.not_after.as_str()
        || checked_at >= plan.not_after.as_str()
        || checked_at >= plan.capability.expires_at.as_str()
        || checked_at >= plan.lease_authority.valid_until.as_str()
        || plan
            .artifact_accesses
            .iter()
            .any(|access| checked_at >= access.expires_at.as_str())
        || base.command.not_after != plan.not_after
        || start.hard_deadline_at > plan.lease_authority.valid_until
    {
        return Ok(blocked("EXECUTION_PLAN_NOT_CURRENT"));
    }
    let rows = registry_rows_on(connection, &base.route)?;
    audit_registry_rows(&base, &rows)?;
    let route = &base.route.authorization;
    let actor = &base.actor_authority.authorization;
    if rows.provider_kind != route.provider.provider_kind
        || rows.provider_owner_account_id != route.provider.provider_owner_account_id
        || !matches!(rows.provider_status.as_str(), "active" | "draining")
    {
        return Ok(blocked("ROUTE_PROVIDER_NOT_CURRENT"));
    }
    if rows.current_adapter_revision != route.route.adapter.adapter_revision
        || rows.current_adapter_digest != route.route.adapter.adapter_registry_digest
        || rows.current_adapter_status != COMPUTE_ROUTE_ADAPTER_STATUS_ACTIVE
    {
        return Ok(blocked("ROUTE_ADAPTER_NOT_CURRENT"));
    }
    if rows.current_credential_revision != route.credential.credential_revision
        || rows.current_credential_digest != route.credential.credential_digest
        || rows.current_credential_status != COMPUTE_ROUTE_ADAPTER_STATUS_ACTIVE
        || checked_at >= route.credential.expires_at.as_str()
    {
        return Ok(blocked("ROUTE_CREDENTIAL_NOT_CURRENT"));
    }
    let revoked = connection
        .query_row(
            "SELECT 1 FROM compute_route_credential_revocations
              WHERE credential_id=?1 AND credential_revision=?2 LIMIT 1",
            params![
                route.credential.credential_id,
                route.credential.credential_revision
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if revoked {
        return Ok(blocked("ROUTE_CREDENTIAL_REVOKED"));
    }
    if checked_at < route.recorded_at.as_str() || checked_at >= route.expires_at.as_str() {
        return Ok(blocked("ROUTE_NOT_CURRENT"));
    }
    if checked_at < actor.recorded_at.as_str()
        || checked_at >= actor.valid_until.as_str()
        || !actor
            .allowed_route_kinds
            .iter()
            .any(|kind| kind == &route.route.route_kind)
        || !actor
            .allowed_actor_phases
            .iter()
            .any(|phase| phase == COMPUTE_ACTOR_PHASE_APPLICATION)
    {
        return Ok(blocked("APPLICATION_ACTOR_NOT_CURRENT"));
    }
    let route_commit = route
        .capabilities
        .iter()
        .find(|capability| capability.capability_id == COMPUTE_ROUTE_CAPABILITY_IDEMPOTENT_COMMIT);
    let required_commit = plan
        .required_route_capabilities
        .iter()
        .find(|capability| capability.capability_id == COMPUTE_ROUTE_CAPABILITY_IDEMPOTENT_COMMIT);
    if !matches!(
        (route_commit, required_commit),
        (Some(actual), Some(required)) if actual.capability_revision >= required.minimum_revision
    ) {
        return Ok(blocked("COMMIT_CAPABILITY_NOT_CURRENT"));
    }
    Ok(AcceptedStartCommitFreshness::Current)
}

fn registry_rows_on(
    connection: &Connection,
    route: &crate::compute_federation::route_authority::ComputeRouteAuthorizationEnvelope,
) -> Result<RegistryRows> {
    let authorization = &route.authorization;
    connection
        .query_row(
            "SELECT adapter_version.adapter_json, adapter_version.adapter_digest,
                    adapter.current_adapter_revision, adapter.current_adapter_digest,
                    adapter.status, credential_version.credential_json,
                    credential_version.credential_digest,
                    credential.current_credential_revision,
                    credential.current_credential_digest, credential.status,
                    seal.seal_json, seal.seal_digest, provider.provider_kind,
                    provider.owner_account_id, provider.status
               FROM compute_route_adapter_versions adapter_version
               JOIN compute_route_adapters adapter
                 ON adapter.adapter_id=adapter_version.adapter_id
               JOIN compute_route_credential_versions credential_version
                 ON credential_version.credential_id=?4
                AND credential_version.credential_revision=?5
               JOIN compute_route_credentials credential
                 ON credential.credential_id=credential_version.credential_id
               JOIN compute_route_authorization_seals seal
                 ON seal.route_authorization_id=?7
                AND seal.route_authorization_digest=?8
               JOIN compute_providers provider ON provider.provider_id=?9
              WHERE adapter_version.adapter_id=?1
                AND adapter_version.adapter_revision=?2
                AND adapter_version.adapter_digest=?3
                AND credential_version.credential_digest=?6",
            params![
                authorization.route.adapter.adapter_id,
                authorization.route.adapter.adapter_revision,
                authorization.route.adapter.adapter_registry_digest,
                authorization.credential.credential_id,
                authorization.credential.credential_revision,
                authorization.credential.credential_digest,
                route.route_authorization_id,
                route.route_authorization_digest,
                authorization.provider.provider_id,
            ],
            |row| {
                Ok(RegistryRows {
                    adapter_json: row.get(0)?,
                    adapter_digest: row.get(1)?,
                    current_adapter_revision: row.get(2)?,
                    current_adapter_digest: row.get(3)?,
                    current_adapter_status: row.get(4)?,
                    credential_json: row.get(5)?,
                    credential_digest: row.get(6)?,
                    current_credential_revision: row.get(7)?,
                    current_credential_digest: row.get(8)?,
                    current_credential_status: row.get(9)?,
                    seal_json: row.get(10)?,
                    seal_digest: row.get(11)?,
                    provider_kind: row.get(12)?,
                    provider_owner_account_id: row.get(13)?,
                    provider_status: row.get(14)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("accepted closure route registry ledger is incomplete"))
}

fn audit_registry_rows(base: &super::AcceptedCommitBase, rows: &RegistryRows) -> Result<()> {
    let adapter: ComputeRouteAdapterVersionEnvelope = serde_json::from_str(&rows.adapter_json)?;
    let credential: ComputeRouteCredentialEnvelope = serde_json::from_str(&rows.credential_json)?;
    let seal: ComputeRouteAuthorizationSealEnvelope = serde_json::from_str(&rows.seal_json)?;
    let (adapter_json, adapter_digest) = canonical_route_adapter_version_json_and_digest(&adapter)?;
    let (credential_json, credential_digest) =
        canonical_route_credential_json_and_digest(&credential)?;
    let (seal_json, seal_digest) = canonical_route_authorization_seal_json_and_digest(&seal)?;
    let route = &base.route.authorization;
    let capability_digest = canonical_route_capability_set_digest(&route.capabilities)?;
    let route_capabilities = route
        .capabilities
        .iter()
        .map(|capability| ComputeRouteCapabilityRevision {
            capability_id: capability.capability_id.clone(),
            capability_revision: capability.capability_revision,
        })
        .collect::<Vec<_>>();
    ensure!(
        adapter_json == rows.adapter_json
            && adapter.adapter_digest == rows.adapter_digest
            && adapter_digest == rows.adapter_digest
            && credential_json == rows.credential_json
            && credential.credential_digest == rows.credential_digest
            && credential_digest == rows.credential_digest
            && seal_json == rows.seal_json
            && seal.seal_digest == rows.seal_digest
            && seal_digest == rows.seal_digest,
        "accepted closure route registry failed canonical audit"
    );
    ensure!(
        adapter.adapter_id == route.route.adapter.adapter_id
            && adapter.adapter_revision == route.route.adapter.adapter_revision
            && adapter.adapter_digest == route.route.adapter.adapter_registry_digest
            && adapter.adapter.release_version == route.route.adapter.adapter_release_version
            && adapter.adapter.implementation_digest == route.route.adapter.implementation_digest
            && adapter.adapter.route_kind == route.route.route_kind
            && adapter.adapter.status == COMPUTE_ROUTE_ADAPTER_STATUS_ACTIVE
            && adapter
                .adapter
                .supported_provider_kinds
                .contains(&route.provider.provider_kind)
            && route_capabilities
                .iter()
                .all(|capability| adapter.adapter.supported_capabilities.contains(capability))
            && adapter.adapter.registered_by_service_actor_id == route.verified_by_service_actor_id
            && adapter.adapter.actor_authorization_id == route.actor_authorization_id
            && adapter.adapter.actor_authorization_digest == route.actor_authorization_digest
            && credential.credential_id == route.credential.credential_id
            && credential.credential_revision == route.credential.credential_revision
            && credential.credential_digest == route.credential.credential_digest
            && credential.credential.provider == route.provider
            && credential.credential.route == route.route
            && credential.credential.verifier == route.verifier
            && !credential
                .credential
                .non_bearer_credential_ref
                .trim()
                .is_empty()
            && credential.credential.expires_at == route.credential.expires_at
            && credential.credential.cleanup_expires_at == route.credential.cleanup_expires_at
            && credential.credential.verified_by_service_actor_id
                == route.verified_by_service_actor_id
            && credential.credential.actor_authorization_id == route.actor_authorization_id
            && credential.credential.actor_authorization_digest == route.actor_authorization_digest
            && seal.route_authorization_id == base.route.route_authorization_id
            && seal.route_authorization_revision == base.route.route_authorization_revision
            && seal.route_authorization_digest == base.route.route_authorization_digest
            && seal.adapter_id == route.route.adapter.adapter_id
            && seal.adapter_revision == route.route.adapter.adapter_revision
            && seal.adapter_registry_digest == route.route.adapter.adapter_registry_digest
            && seal.credential_id == route.credential.credential_id
            && seal.credential_revision == route.credential.credential_revision
            && seal.credential_digest == route.credential.credential_digest
            && seal.capability_count == i64::try_from(route.capabilities.len())?
            && seal.capability_set_digest == capability_digest,
        "accepted closure route registry conflicts with sealed authority"
    );
    Ok(())
}

fn blocked(reason_code: &'static str) -> AcceptedStartCommitFreshness {
    AcceptedStartCommitFreshness::Quarantine { reason_code }
}

fn parse_canonical_time(value: &str) -> Result<()> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| anyhow!("accepted currentness check time is not RFC3339"))?;
    ensure!(
        parsed.offset().local_minus_utc() == 0
            && parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) == value,
        "accepted currentness check time must use canonical UTC nanoseconds"
    );
    Ok(())
}
