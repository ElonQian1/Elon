use anyhow::{anyhow, bail, ensure, Result};
use chrono::{DateTime, FixedOffset};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::{
    route_authority::{
        canonical_route_adapter_version_json_and_digest,
        canonical_route_authorization_json_and_digest,
        canonical_route_authorization_seal_json_and_digest, canonical_route_capability_set_digest,
        canonical_route_credential_json_and_digest,
        canonical_service_actor_authorization_json_and_digest, ComputeRouteAdapterVersionEnvelope,
        ComputeRouteAuthorizationEnvelope, ComputeRouteAuthorizationSealEnvelope,
        ComputeRouteCredentialEnvelope, ComputeServiceActorAuthorizationEnvelope,
        COMPUTE_PROVIDER_KIND_EXTERNAL_POOL, COMPUTE_ROUTE_ADAPTER_STATUS_ACTIVE,
        COMPUTE_ROUTE_KIND_PROVIDER_ENDPOINT, COMPUTE_ROUTE_KIND_SERVER_ADAPTER,
        COMPUTE_ROUTE_REQUIRED_CAPABILITY_COUNT, COMPUTE_ROUTE_SOURCE_EXTERNAL_POOL_ONBOARDING,
    },
    start_outbox::{
        COMPUTE_START_OPERATION_CANCEL, COMPUTE_START_OPERATION_COMMIT,
        COMPUTE_START_OPERATION_PREPARE, COMPUTE_START_OPERATION_RECONCILE,
    },
};

use super::types::{StartOutboxCurrentnessPhase, StoredStartOutboxOperation};

struct RouteRows {
    route_json: String,
    route_digest: String,
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
    actor_json: String,
    actor_digest: String,
    receipt_phase: String,
    receipt_service_actor_id: String,
    receipt_authorization_id: String,
    receipt_authorization_digest: String,
    receipt_valid_until: String,
    seal_json: String,
    seal_digest: String,
    provider_kind: String,
    provider_owner_account_id: String,
    provider_status: String,
}

pub(super) fn ensure_send_current_on(
    connection: &Connection,
    stored: &StoredStartOutboxOperation,
    checked_at: &str,
) -> Result<()> {
    parse_timestamp(checked_at)?;
    let phase = phase_for_operation(&stored.envelope.operation_kind)?;
    ensure!(
        stored.envelope.not_before.as_str() <= checked_at
            && checked_at < stored.envelope.not_after.as_str(),
        "Start outbox operation is outside its durable delivery window"
    );
    let rows = route_rows_on(connection, stored)?
        .ok_or_else(|| anyhow!("Start outbox route authority closure is missing or stale"))?;
    audit_route_rows(connection, stored, phase, checked_at, rows)?;
    match phase {
        StartOutboxCurrentnessPhase::Prepare => {
            crate::store::compute_attempt_dispatches::ensure_start_outbox_prepare_current_on(
                connection,
                &stored.envelope.command_id,
                &stored.envelope.command_digest,
                checked_at,
            )?
        }
        StartOutboxCurrentnessPhase::Commit => {
            super::send::ensure_commit_source_current_on(connection, stored, checked_at)?
        }
        StartOutboxCurrentnessPhase::CleanupCancel
        | StartOutboxCurrentnessPhase::CleanupReconcile => {
            super::cleanup::ensure_cleanup_send_source_exact_on(connection, stored)?
        }
    }
    Ok(())
}

fn phase_for_operation(operation_kind: &str) -> Result<StartOutboxCurrentnessPhase> {
    match operation_kind {
        COMPUTE_START_OPERATION_PREPARE => Ok(StartOutboxCurrentnessPhase::Prepare),
        COMPUTE_START_OPERATION_COMMIT => Ok(StartOutboxCurrentnessPhase::Commit),
        COMPUTE_START_OPERATION_CANCEL => Ok(StartOutboxCurrentnessPhase::CleanupCancel),
        COMPUTE_START_OPERATION_RECONCILE => Ok(StartOutboxCurrentnessPhase::CleanupReconcile),
        _ => bail!("unsupported Start outbox operation kind"),
    }
}

fn route_rows_on(
    connection: &Connection,
    stored: &StoredStartOutboxOperation,
) -> Result<Option<RouteRows>> {
    connection
        .query_row(
            "SELECT route.route_authorization_json, route.route_authorization_digest,
                    adapter_version.adapter_json, adapter_version.adapter_digest,
                    adapter.current_adapter_revision, adapter.current_adapter_digest,
                    adapter.status, credential_version.credential_json,
                    credential_version.credential_digest,
                    credential.current_credential_revision,
                    credential.current_credential_digest, credential.status,
                    actor_authority.actor_authorization_json,
                    actor_authority.actor_authorization_digest,
                    actor_receipt.actor_phase, actor_receipt.service_actor_id,
                    actor_receipt.actor_authorization_id,
                    actor_receipt.actor_authorization_digest,
                    actor_receipt.valid_until, seal.seal_json, seal.seal_digest,
                    provider.provider_kind, provider.owner_account_id, provider.status
               FROM compute_route_authorization_receipts route
               JOIN compute_route_adapter_versions adapter_version
                 ON adapter_version.adapter_id=route.adapter_id
                AND adapter_version.adapter_revision=route.adapter_revision
               JOIN compute_route_adapters adapter ON adapter.adapter_id=route.adapter_id
               JOIN compute_route_credential_versions credential_version
                 ON credential_version.credential_id=route.credential_id
                AND credential_version.credential_revision=route.credential_revision
               JOIN compute_route_credentials credential
                 ON credential.credential_id=route.credential_id
               JOIN compute_attempt_dispatch_actor_receipts actor_receipt
                 ON actor_receipt.actor_receipt_id=?3
                AND actor_receipt.actor_receipt_digest=?4
               JOIN compute_service_actor_authorizations actor_authority
                 ON actor_authority.actor_authorization_id=actor_receipt.actor_authorization_id
                AND actor_authority.actor_authorization_digest=actor_receipt.actor_authorization_digest
               JOIN compute_route_authorization_seals seal
                 ON seal.route_authorization_id=route.route_authorization_id
                AND seal.route_authorization_digest=route.route_authorization_digest
               JOIN compute_providers provider ON provider.provider_id=route.provider_id
              WHERE route.route_authorization_id=?1
                AND route.route_authorization_digest=?2
                AND route.provider_id=?5 AND route.adapter_id=?6
                AND route.adapter_binding_digest=?7",
            params![
                stored.envelope.route_authorization_id,
                stored.envelope.route_authorization_digest,
                stored.envelope.actor_receipt_id,
                stored.envelope.actor_receipt_digest,
                stored.provider_id,
                stored.adapter_id,
                stored.envelope.adapter_binding_digest,
            ],
            |row| {
                Ok(RouteRows {
                    route_json: row.get(0)?,
                    route_digest: row.get(1)?,
                    adapter_json: row.get(2)?,
                    adapter_digest: row.get(3)?,
                    current_adapter_revision: row.get(4)?,
                    current_adapter_digest: row.get(5)?,
                    current_adapter_status: row.get(6)?,
                    credential_json: row.get(7)?,
                    credential_digest: row.get(8)?,
                    current_credential_revision: row.get(9)?,
                    current_credential_digest: row.get(10)?,
                    current_credential_status: row.get(11)?,
                    actor_json: row.get(12)?,
                    actor_digest: row.get(13)?,
                    receipt_phase: row.get(14)?,
                    receipt_service_actor_id: row.get(15)?,
                    receipt_authorization_id: row.get(16)?,
                    receipt_authorization_digest: row.get(17)?,
                    receipt_valid_until: row.get(18)?,
                    seal_json: row.get(19)?,
                    seal_digest: row.get(20)?,
                    provider_kind: row.get(21)?,
                    provider_owner_account_id: row.get(22)?,
                    provider_status: row.get(23)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn audit_route_rows(
    connection: &Connection,
    stored: &StoredStartOutboxOperation,
    phase: StartOutboxCurrentnessPhase,
    checked_at: &str,
    rows: RouteRows,
) -> Result<()> {
    let route: ComputeRouteAuthorizationEnvelope = serde_json::from_str(&rows.route_json)?;
    let adapter: ComputeRouteAdapterVersionEnvelope = serde_json::from_str(&rows.adapter_json)?;
    let credential: ComputeRouteCredentialEnvelope = serde_json::from_str(&rows.credential_json)?;
    let actor: ComputeServiceActorAuthorizationEnvelope = serde_json::from_str(&rows.actor_json)?;
    let seal: ComputeRouteAuthorizationSealEnvelope = serde_json::from_str(&rows.seal_json)?;
    let (route_json, route_digest) = canonical_route_authorization_json_and_digest(&route)?;
    let (adapter_json, adapter_digest) = canonical_route_adapter_version_json_and_digest(&adapter)?;
    let (credential_json, credential_digest) =
        canonical_route_credential_json_and_digest(&credential)?;
    let (actor_json, actor_digest) = canonical_service_actor_authorization_json_and_digest(&actor)?;
    let (seal_json, seal_digest) = canonical_route_authorization_seal_json_and_digest(&seal)?;
    ensure!(
        route_json == rows.route_json
            && route_digest == rows.route_digest
            && route.route_authorization_digest == rows.route_digest
            && adapter_json == rows.adapter_json
            && adapter_digest == rows.adapter_digest
            && adapter.adapter_digest == rows.adapter_digest
            && credential_json == rows.credential_json
            && credential_digest == rows.credential_digest
            && credential.credential_digest == rows.credential_digest
            && actor_json == rows.actor_json
            && actor_digest == rows.actor_digest
            && actor.actor_authorization_digest == rows.actor_digest
            && seal_json == rows.seal_json
            && seal_digest == rows.seal_digest
            && seal.seal_digest == rows.seal_digest,
        "Start outbox route authority failed canonical currentness audit"
    );
    let auth = &route.authorization;
    let route_shape = &auth.route;
    let credential_body = &credential.credential;
    let actor_body = &actor.authorization;
    let requires_live_registry = !phase.uses_cleanup_horizon();
    let horizon = if phase.uses_cleanup_horizon() {
        &auth.cleanup_expires_at
    } else {
        &auth.expires_at
    };
    ensure!(
        route.route_authorization_id == stored.envelope.route_authorization_id
            && route.route_authorization_digest == stored.envelope.route_authorization_digest
            && auth.provider.provider_id == stored.provider_id
            && (!requires_live_registry
                || (auth.provider.provider_kind == rows.provider_kind
                    && auth.provider.provider_owner_account_id == rows.provider_owner_account_id
                    && matches!(rows.provider_status.as_str(), "active" | "draining")))
            && route_shape.adapter_binding_digest == stored.envelope.adapter_binding_digest
            && route_shape.route_binding_digest == route_shape.adapter_binding_digest
            && route_shape.adapter.adapter_id == stored.adapter_id
            && (!requires_live_registry
                || (rows.current_adapter_revision == route_shape.adapter.adapter_revision
                    && rows.current_adapter_digest == route_shape.adapter.adapter_registry_digest
                    && rows.current_adapter_status == COMPUTE_ROUTE_ADAPTER_STATUS_ACTIVE))
            && adapter.adapter_id == route_shape.adapter.adapter_id
            && adapter.adapter_revision == route_shape.adapter.adapter_revision
            && adapter.adapter_digest == route_shape.adapter.adapter_registry_digest
            && adapter.adapter.release_version == route_shape.adapter.adapter_release_version
            && adapter.adapter.implementation_digest == route_shape.adapter.implementation_digest
            && adapter.adapter.route_kind == route_shape.route_kind
            && adapter.adapter.status == COMPUTE_ROUTE_ADAPTER_STATUS_ACTIVE
            && adapter.adapter.registered_by_service_actor_id == rows.receipt_service_actor_id
            && adapter.adapter.actor_authorization_id == rows.receipt_authorization_id
            && adapter.adapter.actor_authorization_digest == rows.receipt_authorization_digest
            && adapter
                .adapter
                .supported_provider_kinds
                .contains(&auth.provider.provider_kind)
            && credential.credential_id == auth.credential.credential_id
            && credential.credential_revision == auth.credential.credential_revision
            && credential.credential_digest == auth.credential.credential_digest
            && (!requires_live_registry
                || (rows.current_credential_revision == credential.credential_revision
                    && rows.current_credential_digest == credential.credential_digest
                    && rows.current_credential_status == "active"))
            && credential_body.provider == auth.provider
            && credential_body.route == *route_shape
            && credential_body.verifier == auth.verifier
            && credential_body.verifier == adapter.adapter.credential_verifier
            && credential_body.verified_by_service_actor_id == rows.receipt_service_actor_id
            && credential_body.actor_authorization_id == rows.receipt_authorization_id
            && credential_body.actor_authorization_digest == rows.receipt_authorization_digest
            && credential_body.expires_at == auth.credential.expires_at
            && credential_body.cleanup_expires_at == auth.credential.cleanup_expires_at
            && !credential_body.non_bearer_credential_ref.trim().is_empty()
            && actor.actor_authorization_id == rows.receipt_authorization_id
            && actor.actor_authorization_digest == rows.receipt_authorization_digest
            && actor_body.provider_id == auth.provider.provider_id
            && actor_body.provider_owner_account_id == auth.provider.provider_owner_account_id
            && actor_body.service_actor_id == rows.receipt_service_actor_id
            && actor_body.service_actor_id != actor_body.provider_owner_account_id
            && actor_body
                .allowed_route_kinds
                .contains(&route_shape.route_kind)
            && actor_body
                .allowed_actor_phases
                .iter()
                .any(|value| value == phase.required_actor_phase())
            && rows.receipt_phase == phase.required_actor_phase()
            && auth.verified_by_service_actor_id == rows.receipt_service_actor_id
            && auth.actor_authorization_id == rows.receipt_authorization_id
            && auth.actor_authorization_digest == rows.receipt_authorization_digest
            && auth.source.approved_by_user_id == auth.provider.provider_owner_account_id
            && seal.route_authorization_id == route.route_authorization_id
            && seal.route_authorization_revision == route.route_authorization_revision
            && seal.route_authorization_digest == route.route_authorization_digest
            && seal.adapter_id == route_shape.adapter.adapter_id
            && seal.adapter_revision == route_shape.adapter.adapter_revision
            && seal.adapter_registry_digest == route_shape.adapter.adapter_registry_digest
            && seal.credential_id == credential.credential_id
            && seal.credential_revision == credential.credential_revision
            && seal.credential_digest == credential.credential_digest
            && (!requires_live_registry
                || (checked_at < rows.receipt_valid_until.as_str()
                    && checked_at < actor_body.valid_until.as_str()))
            && auth.recorded_at.as_str() <= checked_at
            && checked_at < horizon.as_str(),
        "Start outbox route, credential, Provider, or actor is not current"
    );
    let shape_ok = match route_shape.route_kind.as_str() {
        COMPUTE_ROUTE_KIND_PROVIDER_ENDPOINT => {
            route_shape.endpoint_id.is_some() && route_shape.endpoint_transport.is_some()
        }
        COMPUTE_ROUTE_KIND_SERVER_ADAPTER => {
            route_shape.endpoint_id.is_none() && route_shape.endpoint_transport.is_none()
        }
        _ => false,
    };
    ensure!(shape_ok, "Start outbox route shape is invalid");
    ensure!(
        auth.provider.provider_kind != COMPUTE_PROVIDER_KIND_EXTERNAL_POOL
            || auth.source.source_kind == COMPUTE_ROUTE_SOURCE_EXTERNAL_POOL_ONBOARDING,
        "external-pool route lacks exact onboarding authority"
    );
    ensure!(
        !route_shape.adapter.config_digest.is_empty()
            && route_shape.adapter.config_digest.trim() == route_shape.adapter.config_digest
            && route_shape.adapter.config_digest.len() <= 512,
        "route Adapter config digest is not an opaque exact identifier"
    );
    let capability_digest = canonical_route_capability_set_digest(&auth.capabilities)?;
    let required_capabilities = [
        "authenticated_ack",
        "authenticated_events",
        "cancel_no_start",
        "idempotent_commit",
        "prepare",
        "reconcile",
    ];
    ensure!(
        auth.capabilities.len() as i64 == COMPUTE_ROUTE_REQUIRED_CAPABILITY_COUNT
            && auth
                .capabilities
                .iter()
                .enumerate()
                .all(|(ordinal, capability)| capability.ordinal == ordinal as i64
                    && capability.capability_id == required_capabilities[ordinal])
            && capability_digest == seal.capability_set_digest
            && auth
                .capabilities
                .iter()
                .any(|cap| cap.capability_id == phase.required_capability())
            && auth
                .capabilities
                .iter()
                .all(|cap| adapter.adapter.supported_capabilities.contains(
                    &crate::compute_federation::route_authority::ComputeRouteCapabilityRevision {
                        capability_id: cap.capability_id.clone(),
                        capability_revision: cap.capability_revision,
                    }
                )),
        "Start outbox capability seal is incomplete or stale"
    );
    ensure!(
        super::read::route_capabilities_exact_on(
            connection,
            &route.route_authorization_id,
            &rows.route_json,
        )?,
        "stored route capability rows failed exact audit"
    );
    let revoked = connection
        .query_row(
            "SELECT 1 FROM compute_route_credential_revocations
              WHERE credential_id=?1 AND credential_revision=?2 LIMIT 1",
            params![credential.credential_id, credential.credential_revision],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    ensure!(
        !requires_live_registry || !revoked,
        "Start outbox credential has been revoked"
    );
    Ok(())
}

fn parse_timestamp(value: &str) -> Result<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(value).map_err(Into::into)
}
