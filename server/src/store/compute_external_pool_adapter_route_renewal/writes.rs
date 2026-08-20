use anyhow::{ensure, Result};
use rusqlite::{named_params, params, Connection};

use crate::compute_federation::route_authority::{
    canonical_route_authorization_json_and_digest,
    canonical_route_authorization_seal_json_and_digest, canonical_route_capability_set_digest,
    canonical_route_credential_json_and_digest,
    canonical_service_actor_authorization_json_and_digest, AuthorizedComputeRouteAuthorization,
    ComputeRouteCredentialEnvelope, ComputeServiceActorAuthorizationEnvelope,
};

pub(super) struct CredentialRootState {
    pub(super) credential_id: String,
    pub(super) revision: i64,
    pub(super) digest: String,
    pub(super) status: String,
    pub(super) updated_at: String,
}

pub(super) fn credential_root_on(
    connection: &Connection,
    credential_id: &str,
) -> Result<CredentialRootState> {
    connection
        .query_row(
            "SELECT credential_id,current_credential_revision,current_credential_digest,status,updated_at
               FROM compute_route_credentials WHERE credential_id=?1",
            [credential_id],
            |row| {
                Ok(CredentialRootState {
                    credential_id: row.get(0)?,
                    revision: row.get(1)?,
                    digest: row.get(2)?,
                    status: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )
        .map_err(Into::into)
}

pub(super) fn insert_actor_on(
    connection: &Connection,
    envelope: &ComputeServiceActorAuthorizationEnvelope,
) -> Result<()> {
    let (json, digest) = canonical_service_actor_authorization_json_and_digest(envelope)?;
    ensure!(
        digest == envelope.actor_authorization_digest,
        "V278 actor digest mismatch"
    );
    let actor = &envelope.authorization;
    connection.execute(
        "INSERT INTO compute_service_actor_authorizations (
           actor_authorization_id,actor_authorization_revision,actor_authorization_schema,
           actor_authorization_digest,actor_authorization_json,canonicalization,digest_algorithm,
           provider_id,provider_owner_account_id,service_actor_id,service_actor_kind,
           allowed_route_kinds_json,allowed_actor_phases_json,issued_by_user_id,
           issued_at,valid_until,recorded_at
         ) VALUES (
           :id,:revision,:schema,:digest,:json,:canonicalization,:algorithm,
           :provider_id,:owner_id,:service_actor_id,:kind,:route_kinds,:actor_phases,
           :issued_by,:issued_at,:valid_until,:recorded_at) ",
        named_params! {
            ":id": envelope.actor_authorization_id, ":revision": envelope.actor_authorization_revision,
            ":schema": envelope.schema, ":digest": envelope.actor_authorization_digest,
            ":json": json, ":canonicalization": envelope.canonicalization,
            ":algorithm": envelope.digest_algorithm, ":provider_id": actor.provider_id,
            ":owner_id": actor.provider_owner_account_id, ":service_actor_id": actor.service_actor_id,
            ":kind": actor.service_actor_kind,
            ":route_kinds": serde_json::to_string(&actor.allowed_route_kinds)?,
            ":actor_phases": serde_json::to_string(&actor.allowed_actor_phases)?,
            ":issued_by": actor.issued_by_user_id, ":issued_at": actor.issued_at,
            ":valid_until": actor.valid_until, ":recorded_at": actor.recorded_at,
        },
    )?;
    Ok(())
}

pub(super) fn insert_credential_on(
    connection: &Connection,
    envelope: &ComputeRouteCredentialEnvelope,
) -> Result<()> {
    let (json, digest) = canonical_route_credential_json_and_digest(envelope)?;
    ensure!(
        digest == envelope.credential_digest,
        "V278 credential digest mismatch"
    );
    let credential = &envelope.credential;
    let route = &credential.route;
    connection.execute(
        "INSERT INTO compute_route_credential_versions (
           credential_id,credential_revision,credential_schema,credential_digest,credential_json,
           canonicalization,digest_algorithm,provider_id,provider_kind,provider_owner_account_id,
           route_kind,route_binding_digest,adapter_binding_digest,endpoint_id,endpoint_transport,
           adapter_id,adapter_revision,adapter_registry_digest,adapter_release_version,
           implementation_digest,adapter_config_revision,adapter_config_digest,
           non_bearer_credential_ref,credential_hint,verification_kind,verifier_id,
           verifier_revision,verifier_digest,verification_receipt_id,verification_receipt_digest,
           verified_by_service_actor_id,actor_authorization_id,actor_authorization_digest,
           authenticated_at,expires_at,cleanup_expires_at,recorded_at
         ) VALUES (
           :id,:revision,:schema,:digest,:json,:canonicalization,:algorithm,:provider_id,
           :provider_kind,:owner_id,:route_kind,:route_digest,:adapter_binding,:endpoint_id,
           :endpoint_transport,:adapter_id,:adapter_revision,:adapter_registry,:release,
           :implementation,:config_revision,:config_digest,:credential_ref,:hint,
           :verification_kind,:verifier_id,:verifier_revision,:verifier_digest,:receipt_id,
           :receipt_digest,:verified_by,:actor_id,:actor_digest,:authenticated_at,:expires_at,
           :cleanup_expires_at,:recorded_at)",
        named_params! {
            ":id": envelope.credential_id, ":revision": envelope.credential_revision,
            ":schema": envelope.schema, ":digest": envelope.credential_digest, ":json": json,
            ":canonicalization": envelope.canonicalization, ":algorithm": envelope.digest_algorithm,
            ":provider_id": credential.provider.provider_id, ":provider_kind": credential.provider.provider_kind,
            ":owner_id": credential.provider.provider_owner_account_id, ":route_kind": route.route_kind,
            ":route_digest": route.route_binding_digest, ":adapter_binding": route.adapter_binding_digest,
            ":endpoint_id": route.endpoint_id, ":endpoint_transport": route.endpoint_transport,
            ":adapter_id": route.adapter.adapter_id, ":adapter_revision": route.adapter.adapter_revision,
            ":adapter_registry": route.adapter.adapter_registry_digest,
            ":release": route.adapter.adapter_release_version, ":implementation": route.adapter.implementation_digest,
            ":config_revision": route.adapter.config_revision, ":config_digest": route.adapter.config_digest,
            ":credential_ref": credential.non_bearer_credential_ref, ":hint": credential.credential_hint,
            ":verification_kind": credential.verifier.verification_kind, ":verifier_id": credential.verifier.verifier_id,
            ":verifier_revision": credential.verifier.verifier_revision, ":verifier_digest": credential.verifier.verifier_digest,
            ":receipt_id": credential.verification_receipt_id, ":receipt_digest": credential.verification_receipt_digest,
            ":verified_by": credential.verified_by_service_actor_id, ":actor_id": credential.actor_authorization_id,
            ":actor_digest": credential.actor_authorization_digest, ":authenticated_at": credential.authenticated_at,
            ":expires_at": credential.expires_at, ":cleanup_expires_at": credential.cleanup_expires_at,
            ":recorded_at": credential.recorded_at,
        },
    )?;
    Ok(())
}

pub(super) fn cas_credential_root_on(
    connection: &Connection,
    old: &CredentialRootState,
    new: &ComputeRouteCredentialEnvelope,
) -> Result<()> {
    ensure!(old.status == "active", "V278 credential root is not active");
    let changed = connection.execute(
        "UPDATE compute_route_credentials
            SET current_credential_revision=?1,current_credential_digest=?2,updated_at=?3
          WHERE credential_id=?4 AND current_credential_revision=?5
            AND current_credential_digest=?6 AND status=?7 AND updated_at=?8",
        params![
            new.credential_revision,
            new.credential_digest,
            new.credential.recorded_at,
            old.credential_id,
            old.revision,
            old.digest,
            old.status,
            old.updated_at,
        ],
    )?;
    ensure!(
        changed == 1,
        "V278 credential-root CAS lost its predecessor"
    );
    Ok(())
}

pub(super) fn insert_authorization_on(
    connection: &Connection,
    route: &AuthorizedComputeRouteAuthorization,
) -> Result<()> {
    let envelope = route.envelope();
    let authorization = &envelope.authorization;
    let shape = &authorization.route;
    let (json, digest) = canonical_route_authorization_json_and_digest(envelope)?;
    let capability_digest = canonical_route_capability_set_digest(&authorization.capabilities)?;
    ensure!(
        digest == envelope.route_authorization_digest,
        "V278 authorization digest mismatch"
    );
    connection.execute(
        "INSERT INTO compute_route_authorization_receipts (
           route_authorization_id,route_authorization_revision,route_authorization_schema,
           route_authorization_digest,route_authorization_json,canonicalization,digest_algorithm,
           provider_id,provider_kind,provider_owner_account_id,executor_id,route_kind,
           route_binding_digest,adapter_binding_digest,endpoint_id,endpoint_transport,adapter_id,
           adapter_revision,adapter_registry_digest,adapter_release_version,implementation_digest,
           adapter_config_revision,adapter_config_digest,credential_id,credential_revision,
           credential_digest,credential_expires_at,credential_cleanup_expires_at,capability_count,
           capability_set_digest,source_kind,source_id,source_digest,approved_by_user_id,
           verification_kind,verifier_id,verifier_revision,verifier_digest,verification_receipt_id,
           verification_receipt_digest,verified_by_service_actor_id,actor_authorization_id,
           actor_authorization_digest,authenticated_at,authorized_at,expires_at,
           cleanup_expires_at,recorded_at
         ) VALUES (
           :id,:revision,:schema,:digest,:json,:canonicalization,:algorithm,:provider_id,
           :provider_kind,:owner_id,:executor_id,:route_kind,:route_digest,:adapter_binding,
           :endpoint_id,:endpoint_transport,:adapter_id,:adapter_revision,:adapter_registry,
           :release,:implementation,:config_revision,:config_digest,:credential_id,
           :credential_revision,:credential_digest,:credential_expires,:credential_cleanup,
           :capability_count,:capability_digest,:source_kind,:source_id,:source_digest,
           :approved_by,:verification_kind,:verifier_id,:verifier_revision,:verifier_digest,
           :receipt_id,:receipt_digest,:verified_by,:actor_id,:actor_digest,:authenticated_at,
           :authorized_at,:expires_at,:cleanup_expires_at,:recorded_at)",
        named_params! {
            ":id": envelope.route_authorization_id, ":revision": envelope.route_authorization_revision,
            ":schema": envelope.schema, ":digest": envelope.route_authorization_digest, ":json": json,
            ":canonicalization": envelope.canonicalization, ":algorithm": envelope.digest_algorithm,
            ":provider_id": authorization.provider.provider_id, ":provider_kind": authorization.provider.provider_kind,
            ":owner_id": authorization.provider.provider_owner_account_id, ":executor_id": authorization.executor_id,
            ":route_kind": shape.route_kind, ":route_digest": shape.route_binding_digest,
            ":adapter_binding": shape.adapter_binding_digest, ":endpoint_id": shape.endpoint_id,
            ":endpoint_transport": shape.endpoint_transport, ":adapter_id": shape.adapter.adapter_id,
            ":adapter_revision": shape.adapter.adapter_revision, ":adapter_registry": shape.adapter.adapter_registry_digest,
            ":release": shape.adapter.adapter_release_version, ":implementation": shape.adapter.implementation_digest,
            ":config_revision": shape.adapter.config_revision, ":config_digest": shape.adapter.config_digest,
            ":credential_id": authorization.credential.credential_id,
            ":credential_revision": authorization.credential.credential_revision,
            ":credential_digest": authorization.credential.credential_digest,
            ":credential_expires": authorization.credential.expires_at,
            ":credential_cleanup": authorization.credential.cleanup_expires_at,
            ":capability_count": authorization.capabilities.len() as i64, ":capability_digest": capability_digest,
            ":source_kind": authorization.source.source_kind, ":source_id": authorization.source.source_id,
            ":source_digest": authorization.source.source_digest, ":approved_by": authorization.source.approved_by_user_id,
            ":verification_kind": authorization.verifier.verification_kind, ":verifier_id": authorization.verifier.verifier_id,
            ":verifier_revision": authorization.verifier.verifier_revision, ":verifier_digest": authorization.verifier.verifier_digest,
            ":receipt_id": authorization.verification_receipt_id, ":receipt_digest": authorization.verification_receipt_digest,
            ":verified_by": authorization.verified_by_service_actor_id, ":actor_id": authorization.actor_authorization_id,
            ":actor_digest": authorization.actor_authorization_digest, ":authenticated_at": authorization.authenticated_at,
            ":authorized_at": authorization.authorized_at, ":expires_at": authorization.expires_at,
            ":cleanup_expires_at": authorization.cleanup_expires_at, ":recorded_at": authorization.recorded_at,
        },
    )?;
    Ok(())
}

pub(super) fn insert_capabilities_and_seal_on(
    connection: &Connection,
    route: &AuthorizedComputeRouteAuthorization,
) -> Result<()> {
    for capability in &route.envelope().authorization.capabilities {
        connection.execute(
            "INSERT INTO compute_route_authorization_capabilities
             (route_authorization_id,ordinal,capability_id,capability_revision)
             VALUES (?1,?2,?3,?4)",
            params![
                route.envelope().route_authorization_id,
                capability.ordinal,
                capability.capability_id,
                capability.capability_revision,
            ],
        )?;
    }
    let seal = route.seal();
    let (json, digest) = canonical_route_authorization_seal_json_and_digest(seal)?;
    ensure!(digest == seal.seal_digest, "V278 seal digest mismatch");
    connection.execute(
        "INSERT INTO compute_route_authorization_seals (
           route_authorization_id,route_authorization_revision,seal_id,seal_schema,seal_digest,
           seal_json,canonicalization,digest_algorithm,route_authorization_digest,adapter_id,
           adapter_revision,adapter_registry_digest,credential_id,credential_revision,
           credential_digest,capability_count,capability_set_digest,sealed_at,recorded_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?18)",
        params![
            seal.route_authorization_id,
            seal.route_authorization_revision,
            seal.seal_id,
            seal.schema,
            seal.seal_digest,
            json,
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
