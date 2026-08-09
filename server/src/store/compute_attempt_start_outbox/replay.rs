use anyhow::{anyhow, bail, ensure, Result};
use rusqlite::{named_params, params, Connection, OptionalExtension};

use crate::compute_federation::{
    route_authority::{
        canonical_route_adapter_version_json_and_digest,
        canonical_route_authorization_json_and_digest,
        canonical_route_authorization_seal_json_and_digest, canonical_route_capability_set_digest,
        canonical_route_credential_json_and_digest,
        canonical_service_actor_authorization_json_and_digest, AuthorizedComputeRouteAuthorization,
    },
    start_outbox::{
        canonical_attempt_dispatch_actor_receipt_json_and_digest,
        AuthorizedComputeAttemptDispatchActorReceipt, ValidatedComputeStartOutboxOperation,
    },
};

use super::types::StoredStartOutboxOperation;

pub(super) fn ensure_operation_replay_matches(
    stored: &StoredStartOutboxOperation,
    expected: &ValidatedComputeStartOutboxOperation,
) -> Result<()> {
    if &stored.envelope != expected.envelope() {
        bail!("Start outbox replay conflicts with the immutable sealed operation");
    }
    Ok(())
}

pub(super) fn persist_service_actor_on(
    connection: &Connection,
    envelope: &crate::compute_federation::route_authority::ComputeServiceActorAuthorizationEnvelope,
) -> Result<()> {
    let (json, digest) = canonical_service_actor_authorization_json_and_digest(envelope)?;
    ensure!(
        digest == envelope.actor_authorization_digest,
        "service actor digest mismatch"
    );
    let auth = &envelope.authorization;
    connection.execute(
        "INSERT INTO compute_service_actor_authorizations (
            actor_authorization_id, actor_authorization_revision,
            actor_authorization_schema, actor_authorization_digest,
            actor_authorization_json, canonicalization, digest_algorithm,
            provider_id, provider_owner_account_id, service_actor_id, service_actor_kind,
            allowed_route_kinds_json, allowed_actor_phases_json, issued_by_user_id,
            issued_at, valid_until, recorded_at
         ) SELECT
             :id, :revision, :schema, :digest, :json, :canonicalization, :algorithm,
             :provider_id, :owner_id, :service_actor_id, :kind,
             :route_kinds, :actor_phases, :issued_by, :issued_at, :valid_until, :recorded_at
          WHERE NOT EXISTS (
                SELECT 1 FROM compute_service_actor_authorizations
                 WHERE actor_authorization_id=:id
          )",
        named_params! {
            ":id": envelope.actor_authorization_id,
            ":revision": envelope.actor_authorization_revision,
            ":schema": envelope.schema,
            ":digest": envelope.actor_authorization_digest,
            ":json": json,
            ":canonicalization": envelope.canonicalization,
            ":algorithm": envelope.digest_algorithm,
            ":provider_id": auth.provider_id,
            ":owner_id": auth.provider_owner_account_id,
            ":service_actor_id": auth.service_actor_id,
            ":kind": auth.service_actor_kind,
            ":route_kinds": serde_json::to_string(&auth.allowed_route_kinds)?,
            ":actor_phases": serde_json::to_string(&auth.allowed_actor_phases)?,
            ":issued_by": auth.issued_by_user_id,
            ":issued_at": auth.issued_at,
            ":valid_until": auth.valid_until,
            ":recorded_at": auth.recorded_at,
        },
    )?;
    Ok(())
}

pub(super) fn persist_adapter_on(
    connection: &Connection,
    envelope: &crate::compute_federation::route_authority::ComputeRouteAdapterVersionEnvelope,
) -> Result<()> {
    let (json, digest) = canonical_route_adapter_version_json_and_digest(envelope)?;
    ensure!(
        digest == envelope.adapter_digest,
        "route Adapter digest mismatch"
    );
    let adapter = &envelope.adapter;
    // The version's root FK is deferred. Insert it first because the root AFTER trigger
    // immediately audits that the current version already exists.
    connection.execute(
        "INSERT INTO compute_route_adapter_versions (
            adapter_id, adapter_revision, adapter_schema, adapter_digest, adapter_json,
            canonicalization, digest_algorithm, release_version, implementation_digest,
            route_kind, supported_provider_kinds_json, credential_verification_kind,
            credential_verifier_id, credential_verifier_revision,
            credential_verifier_digest, supported_capabilities_json, status,
            registered_by_service_actor_id, actor_authorization_id,
            actor_authorization_digest, registered_at
         ) SELECT
             :id, :revision, :schema, :digest, :json, :canonicalization, :algorithm,
             :release, :implementation, :route_kind, :provider_kinds, :verification_kind,
             :verifier_id, :verifier_revision, :verifier_digest, :capabilities, :status,
             :registered_by, :actor_id, :actor_digest, :registered_at
          WHERE NOT EXISTS (
                SELECT 1 FROM compute_route_adapter_versions
                 WHERE adapter_id=:id AND adapter_revision=:revision
          )",
        named_params! {
            ":id": envelope.adapter_id,
            ":revision": envelope.adapter_revision,
            ":schema": envelope.schema,
            ":digest": envelope.adapter_digest,
            ":json": json,
            ":canonicalization": envelope.canonicalization,
            ":algorithm": envelope.digest_algorithm,
            ":release": adapter.release_version,
            ":implementation": adapter.implementation_digest,
            ":route_kind": adapter.route_kind,
            ":provider_kinds": serde_json::to_string(&adapter.supported_provider_kinds)?,
            ":verification_kind": adapter.credential_verifier.verification_kind,
            ":verifier_id": adapter.credential_verifier.verifier_id,
            ":verifier_revision": adapter.credential_verifier.verifier_revision,
            ":verifier_digest": adapter.credential_verifier.verifier_digest,
            ":capabilities": serde_json::to_string(&adapter.supported_capabilities)?,
            ":status": adapter.status,
            ":registered_by": adapter.registered_by_service_actor_id,
            ":actor_id": adapter.actor_authorization_id,
            ":actor_digest": adapter.actor_authorization_digest,
            ":registered_at": adapter.registered_at,
        },
    )?;
    connection.execute(
        "INSERT INTO compute_route_adapters (
             adapter_id, current_adapter_revision, current_adapter_digest,
             status, created_at, updated_at
         ) SELECT ?1, ?2, ?3, ?4, ?5, ?5
          WHERE NOT EXISTS (
                SELECT 1 FROM compute_route_adapters WHERE adapter_id=?1
          )",
        params![
            envelope.adapter_id,
            envelope.adapter_revision,
            envelope.adapter_digest,
            adapter.status,
            adapter.registered_at,
        ],
    )?;
    Ok(())
}

pub(super) fn persist_actor_receipt_on(
    connection: &Connection,
    actor: &AuthorizedComputeAttemptDispatchActorReceipt,
) -> Result<()> {
    let envelope = actor.envelope();
    let (json, digest) = canonical_attempt_dispatch_actor_receipt_json_and_digest(envelope)?;
    ensure!(
        digest == envelope.actor_receipt_digest,
        "dispatch actor digest mismatch"
    );
    connection.execute(
        "INSERT INTO compute_attempt_dispatch_actor_receipts (
            actor_receipt_id, actor_receipt_schema, actor_receipt_digest,
            actor_receipt_json, canonicalization, digest_algorithm, actor_phase,
            command_id, command_digest, provider_id, provider_owner_account_id,
            service_actor_id, actor_authorization_id, actor_authorization_digest,
            route_authorization_id, route_authorization_digest, ack_id, ack_digest,
            application_id, application_digest, issued_at, valid_until, recorded_at
         ) SELECT
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
             ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23
          WHERE NOT EXISTS (
                SELECT 1 FROM compute_attempt_dispatch_actor_receipts
                 WHERE actor_receipt_id=?1
          )",
        params![
            envelope.actor_receipt_id,
            envelope.schema,
            envelope.actor_receipt_digest,
            json,
            envelope.canonicalization,
            envelope.digest_algorithm,
            envelope.actor_phase,
            envelope.command_id,
            envelope.command_digest,
            envelope.provider_id,
            envelope.provider_owner_account_id,
            envelope.service_actor_id,
            envelope.actor_authorization_id,
            envelope.actor_authorization_digest,
            envelope.route_authorization_id,
            envelope.route_authorization_digest,
            envelope.ack_id,
            envelope.ack_digest,
            envelope.application_id,
            envelope.application_digest,
            envelope.issued_at,
            envelope.valid_until,
            envelope.recorded_at,
        ],
    )?;
    Ok(())
}

pub(super) fn ensure_route_registry_current_on(
    connection: &Connection,
    expected: &AuthorizedComputeRouteAuthorization,
    checked_at: &str,
) -> Result<()> {
    let route = expected.envelope();
    let adapter = expected.inputs().adapter().envelope();
    let credential = expected.inputs().credential().envelope();
    let actor = expected.inputs().actor().envelope();
    let current = connection
        .query_row(
            "SELECT 1
               FROM compute_route_adapters adapter
               JOIN compute_route_credentials credential
                 ON credential.credential_id=?4
               JOIN compute_providers provider ON provider.provider_id=?7
              WHERE adapter.adapter_id=?1
                AND adapter.current_adapter_revision=?2
                AND adapter.current_adapter_digest=?3 AND adapter.status='active'
                AND credential.current_credential_revision=?5
                AND credential.current_credential_digest=?6 AND credential.status='active'
                AND provider.provider_kind=?8 AND provider.owner_account_id=?9
                AND provider.status IN ('active','draining')
                AND ?10<?11 AND ?10<?12 AND ?10<?13
                AND NOT EXISTS (
                    SELECT 1 FROM compute_route_credential_revocations revoked
                     WHERE revoked.credential_id=?4 AND revoked.credential_revision=?5
                )",
            params![
                adapter.adapter_id,
                adapter.adapter_revision,
                adapter.adapter_digest,
                credential.credential_id,
                credential.credential_revision,
                credential.credential_digest,
                route.authorization.provider.provider_id,
                route.authorization.provider.provider_kind,
                route.authorization.provider.provider_owner_account_id,
                checked_at,
                route.authorization.expires_at,
                credential.credential.expires_at,
                actor.authorization.valid_until,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    ensure!(
        current,
        "fresh Start route registry or credential is not current"
    );
    Ok(())
}

pub(super) fn ensure_route_authority_replay_on(
    connection: &Connection,
    expected: &AuthorizedComputeRouteAuthorization,
) -> Result<()> {
    let inputs = expected.inputs();
    let adapter = inputs.adapter().envelope();
    let credential = inputs.credential().envelope();
    let actor = inputs.actor().envelope();
    let route = expected.envelope();
    let seal = expected.seal();
    let (adapter_json, adapter_digest) = canonical_route_adapter_version_json_and_digest(adapter)?;
    let (credential_json, credential_digest) =
        canonical_route_credential_json_and_digest(credential)?;
    let (actor_json, actor_digest) = canonical_service_actor_authorization_json_and_digest(actor)?;
    let (route_json, route_digest) = canonical_route_authorization_json_and_digest(route)?;
    let (seal_json, seal_digest) = canonical_route_authorization_seal_json_and_digest(seal)?;
    let capability_digest =
        canonical_route_capability_set_digest(&route.authorization.capabilities)?;
    for (label, actual, expected_digest) in [
        (
            "Adapter",
            adapter.adapter_digest.as_str(),
            adapter_digest.as_str(),
        ),
        (
            "credential",
            credential.credential_digest.as_str(),
            credential_digest.as_str(),
        ),
        (
            "service actor",
            actor.actor_authorization_digest.as_str(),
            actor_digest.as_str(),
        ),
        (
            "route authorization",
            route.route_authorization_digest.as_str(),
            route_digest.as_str(),
        ),
        (
            "route seal",
            seal.seal_digest.as_str(),
            seal_digest.as_str(),
        ),
    ] {
        if actual != expected_digest {
            bail!("sealed {label} digest failed canonical replay audit");
        }
    }
    if capability_digest != seal.capability_set_digest {
        bail!("sealed route capability set failed canonical replay audit");
    }
    ensure_json_row_on(
        connection,
        "compute_service_actor_authorizations",
        "actor_authorization_id",
        &actor.actor_authorization_id,
        "actor_authorization_json",
        &actor_json,
    )?;
    ensure_compound_json_row_on(
        connection,
        "compute_route_adapter_versions",
        "adapter_id",
        &adapter.adapter_id,
        "adapter_revision",
        adapter.adapter_revision,
        "adapter_json",
        &adapter_json,
    )?;
    ensure_compound_json_row_on(
        connection,
        "compute_route_credential_versions",
        "credential_id",
        &credential.credential_id,
        "credential_revision",
        credential.credential_revision,
        "credential_json",
        &credential_json,
    )?;
    ensure_json_row_on(
        connection,
        "compute_route_authorization_receipts",
        "route_authorization_id",
        &route.route_authorization_id,
        "route_authorization_json",
        &route_json,
    )?;
    ensure_json_row_on(
        connection,
        "compute_route_authorization_seals",
        "route_authorization_id",
        &route.route_authorization_id,
        "seal_json",
        &seal_json,
    )?;
    let mut statement = connection.prepare(
        "SELECT ordinal, capability_id, capability_revision
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
    let expected_caps = route
        .authorization
        .capabilities
        .iter()
        .map(|cap| {
            (
                cap.ordinal,
                cap.capability_id.clone(),
                cap.capability_revision,
            )
        })
        .collect::<Vec<_>>();
    if stored != expected_caps {
        bail!("stored route capability rows conflict with the sealed authorization");
    }
    Ok(())
}

pub(super) fn ensure_actor_receipt_replay_on(
    connection: &Connection,
    expected: &AuthorizedComputeAttemptDispatchActorReceipt,
) -> Result<()> {
    let envelope = expected.envelope();
    let (json, digest) = canonical_attempt_dispatch_actor_receipt_json_and_digest(envelope)?;
    if digest != envelope.actor_receipt_digest {
        bail!("sealed dispatch actor receipt digest failed canonical replay audit");
    }
    ensure_json_row_on(
        connection,
        "compute_attempt_dispatch_actor_receipts",
        "actor_receipt_id",
        &envelope.actor_receipt_id,
        "actor_receipt_json",
        &json,
    )
}

fn ensure_json_row_on(
    connection: &Connection,
    table: &str,
    key_column: &str,
    key: &str,
    json_column: &str,
    expected_json: &str,
) -> Result<()> {
    let sql = format!("SELECT {json_column} FROM {table} WHERE {key_column}=?1");
    let stored = connection
        .query_row(&sql, params![key], |row| row.get::<_, String>(0))
        .optional()?;
    match stored {
        Some(json) if json == expected_json => Ok(()),
        Some(_) => bail!("stored {table} row conflicts with sealed replay"),
        None => Err(anyhow!(
            "stored {table} closure is missing; backfill is required"
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn ensure_compound_json_row_on(
    connection: &Connection,
    table: &str,
    key_column: &str,
    key: &str,
    revision_column: &str,
    revision: i64,
    json_column: &str,
    expected_json: &str,
) -> Result<()> {
    let sql = format!(
        "SELECT {json_column} FROM {table}
          WHERE {key_column}=?1 AND {revision_column}=?2"
    );
    let stored = connection
        .query_row(&sql, params![key, revision], |row| row.get::<_, String>(0))
        .optional()?;
    match stored {
        Some(json) if json == expected_json => Ok(()),
        Some(_) => bail!("stored {table} revision conflicts with sealed replay"),
        None => Err(anyhow!(
            "stored {table} revision is missing; backfill is required"
        )),
    }
}
