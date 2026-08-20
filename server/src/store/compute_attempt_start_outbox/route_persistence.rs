//! Shared persistence and exact readback kernel for a sealed compute route closure.
//!
//! Start-outbox and V277 call this module instead of independently ordering the route rows.

use anyhow::Result;
use rusqlite::Connection;

use crate::compute_federation::route_authority::AuthorizedComputeRouteAuthorization;

use super::{enqueue, replay};

pub(super) const INSERT_ROUTE_CREDENTIAL_VERSION: &str =
    "INSERT INTO compute_route_credential_versions (
        credential_id, credential_revision, credential_schema, credential_digest,
        credential_json, canonicalization, digest_algorithm,
        provider_id, provider_kind, provider_owner_account_id,
        route_kind, route_binding_digest, adapter_binding_digest,
        endpoint_id, endpoint_transport, adapter_id, adapter_revision,
        adapter_registry_digest, adapter_release_version, implementation_digest,
        adapter_config_revision, adapter_config_digest, non_bearer_credential_ref,
        credential_hint, verification_kind, verifier_id, verifier_revision,
        verifier_digest, verification_receipt_id, verification_receipt_digest,
        verified_by_service_actor_id, actor_authorization_id,
        actor_authorization_digest, authenticated_at, expires_at,
        cleanup_expires_at, recorded_at
     ) SELECT
         :id, :revision, :schema, :digest, :json, :canonicalization, :algorithm,
        :provider_id, :provider_kind, :owner_id, :route_kind, :route_digest,
        :adapter_digest, :endpoint_id, :endpoint_transport, :adapter_id,
        :adapter_revision, :registry_digest, :release, :implementation,
        :config_revision, :config_digest, :credential_ref, :hint,
        :verification_kind, :verifier_id, :verifier_revision, :verifier_digest,
         :receipt_id, :receipt_digest, :verified_by, :actor_id, :actor_digest,
         :authenticated_at, :expires_at, :cleanup_expires_at, :recorded_at
      WHERE NOT EXISTS (
            SELECT 1 FROM compute_route_credential_versions
             WHERE credential_id=:id AND credential_revision=:revision
      )";

/// Persists the existing route closure in its established FK-safe order.
pub(in crate::store) fn persist_compute_route_authority_on(
    connection: &Connection,
    sealed: &AuthorizedComputeRouteAuthorization,
) -> Result<()> {
    let inputs = sealed.inputs();
    replay::persist_service_actor_on(connection, inputs.actor().envelope())?;
    replay::persist_adapter_on(connection, inputs.adapter().envelope())?;
    enqueue::persist_credential_on(connection, inputs.credential().envelope())?;
    enqueue::persist_authorization_on(connection, sealed)?;
    Ok(())
}

/// Replays every immutable route row and the exact ordered capability set.
pub(in crate::store) fn audit_persisted_compute_route_authority_on(
    connection: &Connection,
    expected: &AuthorizedComputeRouteAuthorization,
) -> Result<()> {
    replay::ensure_route_authority_replay_on(connection, expected)
}

/// Audits the mutable adapter/credential/provider roots at one explicit checked-at anchor.
pub(in crate::store) fn ensure_compute_route_registry_current_on(
    connection: &Connection,
    expected: &AuthorizedComputeRouteAuthorization,
    checked_at: &str,
) -> Result<()> {
    replay::ensure_route_registry_current_on(connection, expected, checked_at)
}
