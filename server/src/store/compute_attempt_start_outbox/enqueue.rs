use anyhow::{anyhow, ensure, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{named_params, params, Connection, OptionalExtension};

use crate::compute_federation::{
    attempt_gateway::{
        canonical_adapter_binding_json_and_digest, ValidatedComputeAttemptStartDispatch,
    },
    route_authority::{
        canonical_route_authorization_json_and_digest,
        canonical_route_authorization_seal_json_and_digest, canonical_route_capability_set_digest,
        canonical_route_credential_json_and_digest, AuthorizedComputeRouteAuthorization,
    },
    start_outbox::{
        canonical_start_outbox_operation_json_and_digest, ValidatedComputeStartOutboxOperation,
        COMPUTE_ACTOR_RECEIPT_PHASE_DISPATCH, COMPUTE_START_OPERATION_PREPARE,
        COMPUTE_START_OUTBOX_CANONICALIZATION, COMPUTE_START_OUTBOX_DIGEST_ALGORITHM,
        COMPUTE_START_OUTBOX_OPERATION_SCHEMA,
    },
};

use super::{
    read::{persist_prepare_operation_on, prepare_by_command_on},
    replay::{
        ensure_actor_receipt_replay_on, ensure_operation_replay_matches,
        ensure_route_authority_replay_on, ensure_route_registry_current_on,
        persist_actor_receipt_on, persist_adapter_on, persist_service_actor_on,
    },
    types::StartOutboxEnqueueReceipt,
};

/// Must run before the command INSERT in the same outer IMMEDIATE transaction.
pub(super) fn enqueue_prepare_on(
    connection: &Connection,
    dispatch: &ValidatedComputeAttemptStartDispatch,
) -> Result<StartOutboxEnqueueReceipt> {
    let operation = dispatch.prepare_outbox();
    ensure_prepare_shape(dispatch, operation)?;
    let command_exists = connection
        .query_row(
            "SELECT 1 FROM compute_attempt_dispatch_commands WHERE command_id=?1",
            params![dispatch.command().command_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if command_exists {
        let stored = prepare_by_command_on(connection, &dispatch.command().command_id)?
            .ok_or_else(|| {
                anyhow!("historical command lacks prepare outbox; backfill is required")
            })?;
        ensure_route_authority_replay_on(connection, operation.route_authorization())?;
        ensure_actor_receipt_replay_on(
            connection,
            operation
                .actor_receipt()
                .ok_or_else(|| anyhow!("prepare operation lacks dispatch actor custody"))?,
        )?;
        ensure_operation_replay_matches(&stored, operation)?;
        return Ok(receipt(operation, true));
    }

    persist_route_authority_on(connection, operation.route_authorization())?;
    let created_at = now_nanos();
    ensure_route_registry_current_on(connection, operation.route_authorization(), &created_at)?;
    let actor = operation
        .actor_receipt()
        .ok_or_else(|| anyhow!("prepare operation lacks dispatch actor custody"))?;
    persist_actor_receipt_on(connection, actor)?;
    persist_prepare_operation_on(connection, operation, &created_at)?;
    let stored = prepare_by_command_on(connection, &dispatch.command().command_id)?
        .ok_or_else(|| anyhow!("prepare outbox is not visible after insert"))?;
    ensure_route_authority_replay_on(connection, operation.route_authorization())?;
    ensure_actor_receipt_replay_on(connection, actor)?;
    ensure_operation_replay_matches(&stored, operation)?;
    Ok(receipt(operation, false))
}

fn ensure_prepare_shape(
    dispatch: &ValidatedComputeAttemptStartDispatch,
    operation: &ValidatedComputeStartOutboxOperation,
) -> Result<()> {
    let envelope = operation.envelope();
    let command = dispatch.command();
    let route = operation.route_authorization().envelope();
    let actor_custody = operation
        .actor_receipt()
        .ok_or_else(|| anyhow!("prepare operation lacks dispatch actor custody"))?;
    let actor = actor_custody.envelope();
    let actor_authority_envelope = actor_custody.actor().envelope();
    let actor_authority = actor_custody.actor().authorization();
    let (json, digest) = canonical_start_outbox_operation_json_and_digest(envelope)?;
    let (_, adapter_binding_digest) =
        canonical_adapter_binding_json_and_digest(dispatch.adapter())?;
    ensure!(
        !json.is_empty() && digest == envelope.outbox_digest,
        "prepare outbox digest mismatch"
    );
    ensure!(
        envelope.schema == COMPUTE_START_OUTBOX_OPERATION_SCHEMA
            && envelope.canonicalization == COMPUTE_START_OUTBOX_CANONICALIZATION
            && envelope.digest_algorithm == COMPUTE_START_OUTBOX_DIGEST_ALGORITHM
            && envelope.operation_kind == COMPUTE_START_OPERATION_PREPARE
            && envelope.operation_generation == 1
            && envelope.subject_outbox_id.is_none()
            && envelope.ack_id.is_none()
            && envelope.ack_digest.is_none()
            && envelope.application_id.is_none()
            && envelope.application_digest.is_none()
            && envelope.lease_authority_id.is_none()
            && envelope.lease_authority_revision.is_none()
            && envelope.lease_authority_digest.is_none()
            && operation.lease_authority().is_none(),
        "prepare outbox has an invalid operation shape"
    );
    ensure!(
        envelope.command_id == command.command_id
            && envelope.command_digest == command.command_digest
            && envelope.plan_id == command.command.execution_plan.plan_id
            && envelope.plan_digest == command.command.execution_plan.plan_digest
            && envelope.lease_id == command.command.identity.attempt_lease_id
            && envelope.fencing_generation == command.command.identity.fencing_generation
            && envelope.issued_at == command.issued_at
            && envelope.not_after == command.not_after
            && route.authorization.provider.provider_id == command.command.provider.provider_id
            && route.authorization.provider.provider_id == dispatch.adapter().provider_id
            && route.authorization.provider.provider_kind == dispatch.adapter().provider_kind
            && route.authorization.provider.provider_owner_account_id
                == dispatch.activation().activated_by_user_id()
            && route.authorization.source.approved_by_user_id
                == dispatch.activation().activated_by_user_id()
            && route.authorization.executor_id == command.command.executor_id
            && route.route_authorization_id == envelope.route_authorization_id
            && route.route_authorization_digest == envelope.route_authorization_digest
            && route.authorization.route.adapter.adapter_id == dispatch.adapter().adapter_id
            && route.authorization.route.route_kind == dispatch.adapter().route_kind
            && route.authorization.route.endpoint_id == dispatch.adapter().endpoint_id
            && route.authorization.route.endpoint_transport
                == dispatch.adapter().endpoint_transport
            && route.authorization.route.adapter.adapter_release_version
                == dispatch.adapter().adapter_version
            && route.authorization.route.adapter.config_revision
                == dispatch.adapter().config_revision
            && route.authorization.route.adapter.config_digest == dispatch.adapter().config_digest
            && route.authorization.route.route_binding_digest == adapter_binding_digest
            && route.authorization.route.adapter_binding_digest == adapter_binding_digest
            && envelope.adapter_binding_digest == adapter_binding_digest
            && actor.actor_phase == COMPUTE_ACTOR_RECEIPT_PHASE_DISPATCH
            && actor.actor_receipt_id == envelope.actor_receipt_id
            && actor.actor_receipt_digest == envelope.actor_receipt_digest
            && actor.command_id == envelope.command_id
            && actor.command_digest == envelope.command_digest
            && actor.provider_id == route.authorization.provider.provider_id
            && actor.provider_owner_account_id
                == route.authorization.provider.provider_owner_account_id
            && actor.service_actor_id == actor_authority.service_actor_id
            && actor.actor_authorization_id == actor_authority_envelope.actor_authorization_id
            && actor.actor_authorization_digest
                == actor_authority_envelope.actor_authorization_digest
            && actor.service_actor_id != actor.provider_owner_account_id,
        "prepare outbox does not exactly bind the command, route, and dispatch actor"
    );
    Ok(())
}

fn persist_route_authority_on(
    connection: &Connection,
    sealed: &AuthorizedComputeRouteAuthorization,
) -> Result<()> {
    let inputs = sealed.inputs();
    persist_service_actor_on(connection, inputs.actor().envelope())?;
    persist_adapter_on(connection, inputs.adapter().envelope())?;
    persist_credential_on(connection, inputs.credential().envelope())?;
    persist_authorization_on(connection, sealed)?;
    Ok(())
}

fn persist_credential_on(
    connection: &Connection,
    envelope: &crate::compute_federation::route_authority::ComputeRouteCredentialEnvelope,
) -> Result<()> {
    let (json, digest) = canonical_route_credential_json_and_digest(envelope)?;
    ensure!(
        digest == envelope.credential_digest,
        "route credential digest mismatch"
    );
    let credential = &envelope.credential;
    let route = &credential.route;
    ensure!(
        route.route_binding_digest == route.adapter_binding_digest,
        "credential route binding does not retain the exact v211 Adapter digest"
    );
    // The version FK back to its current root is deferred. Insert the immutable version first;
    // the current-root AFTER trigger immediately requires that version to be visible.
    connection.execute(
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
          )",
        named_params! {
            ":id": envelope.credential_id,
            ":revision": envelope.credential_revision,
            ":schema": envelope.schema,
            ":digest": envelope.credential_digest,
            ":json": json,
            ":canonicalization": envelope.canonicalization,
            ":algorithm": envelope.digest_algorithm,
            ":provider_id": credential.provider.provider_id,
            ":provider_kind": credential.provider.provider_kind,
            ":owner_id": credential.provider.provider_owner_account_id,
            ":route_kind": route.route_kind,
            ":route_digest": route.route_binding_digest,
            ":adapter_digest": route.adapter_binding_digest,
            ":endpoint_id": route.endpoint_id,
            ":endpoint_transport": route.endpoint_transport,
            ":adapter_id": route.adapter.adapter_id,
            ":adapter_revision": route.adapter.adapter_revision,
            ":registry_digest": route.adapter.adapter_registry_digest,
            ":release": route.adapter.adapter_release_version,
            ":implementation": route.adapter.implementation_digest,
            ":config_revision": route.adapter.config_revision,
            ":config_digest": route.adapter.config_digest,
            ":credential_ref": credential.non_bearer_credential_ref,
            ":hint": credential.credential_hint,
            ":verification_kind": credential.verifier.verification_kind,
            ":verifier_id": credential.verifier.verifier_id,
            ":verifier_revision": credential.verifier.verifier_revision,
            ":verifier_digest": credential.verifier.verifier_digest,
            ":receipt_id": credential.verification_receipt_id,
            ":receipt_digest": credential.verification_receipt_digest,
            ":verified_by": credential.verified_by_service_actor_id,
            ":actor_id": credential.actor_authorization_id,
            ":actor_digest": credential.actor_authorization_digest,
            ":authenticated_at": credential.authenticated_at,
            ":expires_at": credential.expires_at,
            ":cleanup_expires_at": credential.cleanup_expires_at,
            ":recorded_at": credential.recorded_at,
        },
    )?;
    connection.execute(
        "INSERT INTO compute_route_credentials (
             credential_id, current_credential_revision, current_credential_digest,
             status, created_at, updated_at
         ) SELECT ?1, ?2, ?3, 'active', ?4, ?4
          WHERE NOT EXISTS (
                SELECT 1 FROM compute_route_credentials WHERE credential_id=?1
          )",
        params![
            envelope.credential_id,
            envelope.credential_revision,
            envelope.credential_digest,
            credential.recorded_at,
        ],
    )?;
    Ok(())
}

fn persist_authorization_on(
    connection: &Connection,
    sealed: &AuthorizedComputeRouteAuthorization,
) -> Result<()> {
    let envelope = sealed.envelope();
    let authorization = &envelope.authorization;
    let route = &authorization.route;
    let (json, digest) = canonical_route_authorization_json_and_digest(envelope)?;
    let capability_digest = canonical_route_capability_set_digest(&authorization.capabilities)?;
    ensure!(
        digest == envelope.route_authorization_digest
            && capability_digest == sealed.seal().capability_set_digest
            && route.route_binding_digest == route.adapter_binding_digest,
        "route authorization canonical binding mismatch"
    );
    connection.execute(
        "INSERT INTO compute_route_authorization_receipts (
            route_authorization_id, route_authorization_revision,
            route_authorization_schema, route_authorization_digest,
            route_authorization_json, canonicalization, digest_algorithm,
            provider_id, provider_kind, provider_owner_account_id, executor_id,
            route_kind, route_binding_digest, adapter_binding_digest,
            endpoint_id, endpoint_transport, adapter_id, adapter_revision,
            adapter_registry_digest, adapter_release_version, implementation_digest,
            adapter_config_revision, adapter_config_digest,
            credential_id, credential_revision, credential_digest,
            credential_expires_at, credential_cleanup_expires_at,
            capability_count, capability_set_digest, source_kind, source_id,
            source_digest, approved_by_user_id, verification_kind, verifier_id,
            verifier_revision, verifier_digest, verification_receipt_id,
            verification_receipt_digest, verified_by_service_actor_id,
            actor_authorization_id, actor_authorization_digest,
            authenticated_at, authorized_at, expires_at, cleanup_expires_at, recorded_at
         ) SELECT
            :id, :revision, :schema, :digest, :json, :canonicalization, :algorithm,
            :provider_id, :provider_kind, :owner_id, :executor_id, :route_kind,
            :route_digest, :adapter_digest, :endpoint_id, :endpoint_transport,
            :adapter_id, :adapter_revision, :registry_digest, :release,
            :implementation, :config_revision, :config_digest, :credential_id,
            :credential_revision, :credential_digest, :credential_expires_at,
            :credential_cleanup_expires_at, :capability_count, :capability_digest,
            :source_kind, :source_id, :source_digest, :approved_by, :verification_kind,
            :verifier_id, :verifier_revision, :verifier_digest, :receipt_id,
             :receipt_digest, :verified_by, :actor_id, :actor_digest,
             :authenticated_at, :authorized_at, :expires_at, :cleanup_expires_at, :recorded_at
          WHERE NOT EXISTS (
                SELECT 1 FROM compute_route_authorization_receipts
                 WHERE route_authorization_id=:id
          )",
        named_params! {
            ":id": envelope.route_authorization_id,
            ":revision": envelope.route_authorization_revision,
            ":schema": envelope.schema,
            ":digest": envelope.route_authorization_digest,
            ":json": json,
            ":canonicalization": envelope.canonicalization,
            ":algorithm": envelope.digest_algorithm,
            ":provider_id": authorization.provider.provider_id,
            ":provider_kind": authorization.provider.provider_kind,
            ":owner_id": authorization.provider.provider_owner_account_id,
            ":executor_id": authorization.executor_id,
            ":route_kind": route.route_kind,
            ":route_digest": route.route_binding_digest,
            ":adapter_digest": route.adapter_binding_digest,
            ":endpoint_id": route.endpoint_id,
            ":endpoint_transport": route.endpoint_transport,
            ":adapter_id": route.adapter.adapter_id,
            ":adapter_revision": route.adapter.adapter_revision,
            ":registry_digest": route.adapter.adapter_registry_digest,
            ":release": route.adapter.adapter_release_version,
            ":implementation": route.adapter.implementation_digest,
            ":config_revision": route.adapter.config_revision,
            ":config_digest": route.adapter.config_digest,
            ":credential_id": authorization.credential.credential_id,
            ":credential_revision": authorization.credential.credential_revision,
            ":credential_digest": authorization.credential.credential_digest,
            ":credential_expires_at": authorization.credential.expires_at,
            ":credential_cleanup_expires_at": authorization.credential.cleanup_expires_at,
            ":capability_count": authorization.capabilities.len() as i64,
            ":capability_digest": capability_digest,
            ":source_kind": authorization.source.source_kind,
            ":source_id": authorization.source.source_id,
            ":source_digest": authorization.source.source_digest,
            ":approved_by": authorization.source.approved_by_user_id,
            ":verification_kind": authorization.verifier.verification_kind,
            ":verifier_id": authorization.verifier.verifier_id,
            ":verifier_revision": authorization.verifier.verifier_revision,
            ":verifier_digest": authorization.verifier.verifier_digest,
            ":receipt_id": authorization.verification_receipt_id,
            ":receipt_digest": authorization.verification_receipt_digest,
            ":verified_by": authorization.verified_by_service_actor_id,
            ":actor_id": authorization.actor_authorization_id,
            ":actor_digest": authorization.actor_authorization_digest,
            ":authenticated_at": authorization.authenticated_at,
            ":authorized_at": authorization.authorized_at,
            ":expires_at": authorization.expires_at,
            ":cleanup_expires_at": authorization.cleanup_expires_at,
            ":recorded_at": authorization.recorded_at,
        },
    )?;
    for capability in &authorization.capabilities {
        connection.execute(
            "INSERT INTO compute_route_authorization_capabilities (
                 route_authorization_id, ordinal, capability_id, capability_revision
             ) SELECT ?1, ?2, ?3, ?4
              WHERE NOT EXISTS (
                    SELECT 1 FROM compute_route_authorization_capabilities
                     WHERE route_authorization_id=?1 AND ordinal=?2
              )",
            params![
                envelope.route_authorization_id,
                capability.ordinal,
                capability.capability_id,
                capability.capability_revision,
            ],
        )?;
    }
    let seal = sealed.seal();
    let (seal_json, seal_digest) = canonical_route_authorization_seal_json_and_digest(seal)?;
    ensure!(
        seal_digest == seal.seal_digest,
        "route authorization seal digest mismatch"
    );
    connection.execute(
        "INSERT INTO compute_route_authorization_seals (
            route_authorization_id, route_authorization_revision, seal_id, seal_schema,
            seal_digest, seal_json, canonicalization, digest_algorithm,
            route_authorization_digest, adapter_id, adapter_revision,
            adapter_registry_digest, credential_id, credential_revision,
            credential_digest, capability_count, capability_set_digest,
            sealed_at, recorded_at
         ) SELECT
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
             ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?18
          WHERE NOT EXISTS (
                SELECT 1 FROM compute_route_authorization_seals
                 WHERE route_authorization_id=?1
          )",
        params![
            seal.route_authorization_id,
            seal.route_authorization_revision,
            seal.seal_id,
            seal.schema,
            seal.seal_digest,
            seal_json,
            seal.canonicalization,
            seal.digest_algorithm,
            seal.route_authorization_digest,
            seal.adapter_id,
            seal.adapter_revision,
            seal.adapter_registry_digest,
            seal.credential_id,
            seal.credential_revision,
            seal.credential_digest,
            seal.capability_count,
            seal.capability_set_digest,
            seal.sealed_at,
        ],
    )?;
    Ok(())
}

fn receipt(
    operation: &ValidatedComputeStartOutboxOperation,
    replayed: bool,
) -> StartOutboxEnqueueReceipt {
    StartOutboxEnqueueReceipt {
        outbox_id: operation.envelope().outbox_id.clone(),
        outbox_digest: operation.envelope().outbox_digest.clone(),
        replayed,
    }
}

fn now_nanos() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
