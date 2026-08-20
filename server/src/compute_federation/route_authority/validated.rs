use std::fmt;

use anyhow::{ensure, Result};

use super::canonical::{
    canonical_route_adapter_version_json_and_digest, canonical_route_authorization_json_and_digest,
    canonical_route_authorization_seal_json_and_digest, canonical_route_capability_set_digest,
    canonical_route_credential_json_and_digest,
    canonical_service_actor_authorization_json_and_digest,
};

use super::types::{
    ComputeRouteAdapterVersion, ComputeRouteAdapterVersionEnvelope,
    ComputeRouteAuthorizationEnvelope, ComputeRouteAuthorizationSealEnvelope,
    ComputeRouteCredential, ComputeRouteCredentialEnvelope,
    ComputeRouteCredentialRevocationEnvelope, ComputeServiceActorAuthorization,
    ComputeServiceActorAuthorizationEnvelope, COMPUTE_ACTOR_PHASE_DISPATCH,
    COMPUTE_ROUTE_ADAPTER_STATUS_ACTIVE, COMPUTE_ROUTE_ADAPTER_VERSION_SCHEMA,
    COMPUTE_ROUTE_AUTHORIZATION_SCHEMA, COMPUTE_ROUTE_AUTHORIZATION_SEAL_SCHEMA,
    COMPUTE_ROUTE_CANONICALIZATION, COMPUTE_ROUTE_CAPABILITY_AUTHENTICATED_ACK,
    COMPUTE_ROUTE_CAPABILITY_AUTHENTICATED_EVENTS, COMPUTE_ROUTE_CAPABILITY_CANCEL_NO_START,
    COMPUTE_ROUTE_CAPABILITY_IDEMPOTENT_COMMIT, COMPUTE_ROUTE_CAPABILITY_PREPARE,
    COMPUTE_ROUTE_CAPABILITY_RECONCILE, COMPUTE_ROUTE_CREDENTIAL_SCHEMA,
    COMPUTE_ROUTE_DIGEST_ALGORITHM, COMPUTE_ROUTE_KIND_PROVIDER_ENDPOINT,
    COMPUTE_ROUTE_KIND_SERVER_ADAPTER, COMPUTE_ROUTE_REQUIRED_CAPABILITY_COUNT,
    COMPUTE_SERVICE_ACTOR_AUTHORIZATION_SCHEMA,
};

/// Registry custody after immutable revision, digest, status, ordering, and actor checks.
pub(crate) struct ValidatedComputeRouteAdapterVersion {
    envelope: ComputeRouteAdapterVersionEnvelope,
}

impl ValidatedComputeRouteAdapterVersion {
    pub(crate) fn envelope(&self) -> &ComputeRouteAdapterVersionEnvelope {
        &self.envelope
    }

    pub(crate) fn adapter(&self) -> &ComputeRouteAdapterVersion {
        &self.envelope.adapter
    }
}

impl fmt::Debug for ValidatedComputeRouteAdapterVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedComputeRouteAdapterVersion")
            .field("adapter_id", &self.envelope.adapter_id)
            .field("adapter_revision", &self.envelope.adapter_revision)
            .field("status", &self.envelope.adapter.status)
            .finish()
    }
}

/// Authenticated lookup authority. Its non-bearer reference is never exposed by Debug.
pub(crate) struct VerifiedComputeRouteCredential {
    envelope: ComputeRouteCredentialEnvelope,
    adapter: ValidatedComputeRouteAdapterVersion,
}

impl VerifiedComputeRouteCredential {
    pub(crate) fn envelope(&self) -> &ComputeRouteCredentialEnvelope {
        &self.envelope
    }

    pub(crate) fn credential(&self) -> &ComputeRouteCredential {
        &self.envelope.credential
    }

    pub(crate) fn adapter(&self) -> &ValidatedComputeRouteAdapterVersion {
        &self.adapter
    }
}

impl fmt::Debug for VerifiedComputeRouteCredential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedComputeRouteCredential")
            .field("credential_id", &self.envelope.credential_id)
            .field("credential_revision", &self.envelope.credential_revision)
            .field(
                "provider_id",
                &self.envelope.credential.provider.provider_id,
            )
            .field("credential", &"<authenticated non-bearer reference>")
            .finish()
    }
}

/// Current service-actor delegation, distinct from the Provider owner account.
pub(crate) struct AuthorizedComputeServiceActor {
    envelope: ComputeServiceActorAuthorizationEnvelope,
}

impl AuthorizedComputeServiceActor {
    pub(crate) fn envelope(&self) -> &ComputeServiceActorAuthorizationEnvelope {
        &self.envelope
    }

    pub(crate) fn authorization(&self) -> &ComputeServiceActorAuthorization {
        &self.envelope.authorization
    }
}

impl fmt::Debug for AuthorizedComputeServiceActor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizedComputeServiceActor")
            .field(
                "actor_authorization_id",
                &self.envelope.actor_authorization_id,
            )
            .field(
                "service_actor_id",
                &self.envelope.authorization.service_actor_id,
            )
            .field("provider_id", &self.envelope.authorization.provider_id)
            .finish()
    }
}

/// Authorized immutable credential invalidation; it cannot be reconstructed from a DTO.
pub(crate) struct AuthorizedComputeRouteCredentialRevocation {
    envelope: ComputeRouteCredentialRevocationEnvelope,
    actor: AuthorizedComputeServiceActor,
}

impl AuthorizedComputeRouteCredentialRevocation {
    pub(crate) fn envelope(&self) -> &ComputeRouteCredentialRevocationEnvelope {
        &self.envelope
    }

    pub(crate) fn actor(&self) -> &AuthorizedComputeServiceActor {
        &self.actor
    }
}

impl fmt::Debug for AuthorizedComputeRouteCredentialRevocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizedComputeRouteCredentialRevocation")
            .field("revocation_id", &self.envelope.revocation_id)
            .field("credential_id", &self.envelope.credential_id)
            .field("credential_revision", &self.envelope.credential_revision)
            .field("actor", &self.actor)
            .finish()
    }
}

/// Linear custody after exact source, Provider kind, Adapter, credential, currentness, actor,
/// time-window, and ordered six-capability checks. There is intentionally no constructor here.
pub(crate) struct ValidatedComputeRouteAuthorizationInputs {
    authorization: ComputeRouteAuthorizationEnvelope,
    adapter: ValidatedComputeRouteAdapterVersion,
    credential: VerifiedComputeRouteCredential,
    actor: AuthorizedComputeServiceActor,
}

impl ValidatedComputeRouteAuthorizationInputs {
    pub(crate) fn authorization(&self) -> &ComputeRouteAuthorizationEnvelope {
        &self.authorization
    }

    pub(crate) fn adapter(&self) -> &ValidatedComputeRouteAdapterVersion {
        &self.adapter
    }

    pub(crate) fn credential(&self) -> &VerifiedComputeRouteCredential {
        &self.credential
    }

    pub(crate) fn actor(&self) -> &AuthorizedComputeServiceActor {
        &self.actor
    }
}

impl fmt::Debug for ValidatedComputeRouteAuthorizationInputs {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedComputeRouteAuthorizationInputs")
            .field(
                "route_authorization_id",
                &self.authorization.route_authorization_id,
            )
            .field(
                "provider_id",
                &self.authorization.authorization.provider.provider_id,
            )
            .field(
                "route_kind",
                &self.authorization.authorization.route.route_kind,
            )
            .field(
                "capability_count",
                &self.authorization.authorization.capabilities.len(),
            )
            .finish()
    }
}

/// Final sealed authorization consumed by execution planning and outbox Store gates.
pub(crate) struct AuthorizedComputeRouteAuthorization {
    inputs: ValidatedComputeRouteAuthorizationInputs,
    seal: ComputeRouteAuthorizationSealEnvelope,
}

impl AuthorizedComputeRouteAuthorization {
    pub(crate) fn inputs(&self) -> &ValidatedComputeRouteAuthorizationInputs {
        &self.inputs
    }

    pub(crate) fn envelope(&self) -> &ComputeRouteAuthorizationEnvelope {
        self.inputs.authorization()
    }

    pub(crate) fn seal(&self) -> &ComputeRouteAuthorizationSealEnvelope {
        &self.seal
    }
}

impl fmt::Debug for AuthorizedComputeRouteAuthorization {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizedComputeRouteAuthorization")
            .field(
                "route_authorization_id",
                &self.inputs.authorization.route_authorization_id,
            )
            .field("seal_id", &self.seal.seal_id)
            .field("authorization", &"<sealed>")
            .finish()
    }
}

/// Narrow Store reconstruction seam for immutable canonical route envelopes read from typed
/// custody. This is not a DTO constructor: every canonical and cross-root relation is checked
/// before sealed custody is returned.
pub(crate) fn validated_compute_route_authorization_from_canonical_envelopes(
    adapter: ComputeRouteAdapterVersionEnvelope,
    credential: ComputeRouteCredentialEnvelope,
    actor: ComputeServiceActorAuthorizationEnvelope,
    authorization: ComputeRouteAuthorizationEnvelope,
    seal: ComputeRouteAuthorizationSealEnvelope,
) -> Result<AuthorizedComputeRouteAuthorization> {
    let credential_adapter = adapter.clone();
    let (_, adapter_digest) = canonical_route_adapter_version_json_and_digest(&adapter)?;
    let (_, credential_digest) = canonical_route_credential_json_and_digest(&credential)?;
    let (_, actor_digest) = canonical_service_actor_authorization_json_and_digest(&actor)?;
    let (_, authorization_digest) = canonical_route_authorization_json_and_digest(&authorization)?;
    let (_, seal_digest) = canonical_route_authorization_seal_json_and_digest(&seal)?;
    let route = &authorization.authorization;
    let shape = &route.route;
    let credential_body = &credential.credential;
    let actor_body = &actor.authorization;
    let capability_digest = canonical_route_capability_set_digest(&route.capabilities)?;
    ensure!(
        adapter.schema == COMPUTE_ROUTE_ADAPTER_VERSION_SCHEMA
            && credential.schema == COMPUTE_ROUTE_CREDENTIAL_SCHEMA
            && actor.schema == COMPUTE_SERVICE_ACTOR_AUTHORIZATION_SCHEMA
            && authorization.schema == COMPUTE_ROUTE_AUTHORIZATION_SCHEMA
            && seal.schema == COMPUTE_ROUTE_AUTHORIZATION_SEAL_SCHEMA
            && [
                adapter.canonicalization.as_str(),
                credential.canonicalization.as_str(),
                actor.canonicalization.as_str(),
                authorization.canonicalization.as_str(),
                seal.canonicalization.as_str(),
            ]
            .into_iter()
            .all(|value| value == COMPUTE_ROUTE_CANONICALIZATION)
            && [
                adapter.digest_algorithm.as_str(),
                credential.digest_algorithm.as_str(),
                actor.digest_algorithm.as_str(),
                authorization.digest_algorithm.as_str(),
                seal.digest_algorithm.as_str(),
            ]
            .into_iter()
            .all(|value| value == COMPUTE_ROUTE_DIGEST_ALGORITHM),
        "canonical route envelope metadata mismatch"
    );
    ensure!(
        adapter_digest == adapter.adapter_digest
            && credential_digest == credential.credential_digest
            && actor_digest == actor.actor_authorization_digest
            && authorization_digest == authorization.route_authorization_digest
            && seal_digest == seal.seal_digest,
        "canonical route envelope digest mismatch"
    );
    ensure!(
        credential_body.route.adapter.adapter_id == adapter.adapter_id
            && credential_body.route.adapter.adapter_revision == adapter.adapter_revision
            && credential_body.route.adapter.adapter_registry_digest == adapter.adapter_digest
            && route.route == credential_body.route
            && route.provider == credential_body.provider
            && route.credential.credential_id == credential.credential_id
            && route.credential.credential_revision == credential.credential_revision
            && route.credential.credential_digest == credential.credential_digest
            && route.credential.expires_at == credential_body.expires_at
            && route.credential.cleanup_expires_at == credential_body.cleanup_expires_at
            && route.verified_by_service_actor_id == actor_body.service_actor_id
            && route.actor_authorization_id == actor.actor_authorization_id
            && route.actor_authorization_digest == actor.actor_authorization_digest
            && credential_body.verified_by_service_actor_id == actor_body.service_actor_id
            && credential_body.actor_authorization_id == actor.actor_authorization_id
            && credential_body.actor_authorization_digest == actor.actor_authorization_digest
            && credential_body.verifier == route.verifier
            && credential_body.verifier == adapter.adapter.credential_verifier
            && credential_body.verification_receipt_id == route.verification_receipt_id
            && credential_body.verification_receipt_digest == route.verification_receipt_digest
            && route.provider.provider_id == actor_body.provider_id
            && route.provider.provider_owner_account_id == actor_body.provider_owner_account_id,
        "canonical route credential/actor roots mismatch"
    );
    let required_capabilities = [
        COMPUTE_ROUTE_CAPABILITY_AUTHENTICATED_ACK,
        COMPUTE_ROUTE_CAPABILITY_AUTHENTICATED_EVENTS,
        COMPUTE_ROUTE_CAPABILITY_CANCEL_NO_START,
        COMPUTE_ROUTE_CAPABILITY_IDEMPOTENT_COMMIT,
        COMPUTE_ROUTE_CAPABILITY_PREPARE,
        COMPUTE_ROUTE_CAPABILITY_RECONCILE,
    ];
    ensure!(
        adapter.adapter.status == COMPUTE_ROUTE_ADAPTER_STATUS_ACTIVE
            && adapter.adapter.route_kind == shape.route_kind
            && adapter
                .adapter
                .supported_provider_kinds
                .contains(&route.provider.provider_kind)
            && adapter.adapter.registered_by_service_actor_id == actor_body.service_actor_id
            && shape.route_binding_digest == shape.adapter_binding_digest
            && shape.adapter.adapter_release_version == adapter.adapter.release_version
            && shape.adapter.implementation_digest == adapter.adapter.implementation_digest
            && actor_body.allowed_route_kinds.contains(&shape.route_kind)
            && actor_body
                .allowed_actor_phases
                .iter()
                .any(|phase| phase == COMPUTE_ACTOR_PHASE_DISPATCH)
            && !credential_body.non_bearer_credential_ref.trim().is_empty()
            && route.capabilities.len() == COMPUTE_ROUTE_REQUIRED_CAPABILITY_COUNT as usize
            && route
                .capabilities
                .iter()
                .enumerate()
                .all(|(ordinal, capability)| {
                    capability.ordinal == ordinal as i64
                        && capability.capability_id == required_capabilities[ordinal]
                        && capability.capability_revision > 0
                        && adapter
                            .adapter
                            .supported_capabilities
                            .iter()
                            .any(|supported| {
                                supported.capability_id == capability.capability_id
                                    && supported.capability_revision
                                        == capability.capability_revision
                            })
                })
            && match shape.route_kind.as_str() {
                COMPUTE_ROUTE_KIND_PROVIDER_ENDPOINT => {
                    shape.endpoint_id.is_some() && shape.endpoint_transport.is_some()
                }
                COMPUTE_ROUTE_KIND_SERVER_ADAPTER => {
                    shape.endpoint_id.is_none() && shape.endpoint_transport.is_none()
                }
                _ => false,
            },
        "canonical route Adapter, actor, shape, or capability mismatch"
    );
    ensure!(
        seal.route_authorization_id == authorization.route_authorization_id
            && seal.route_authorization_revision == authorization.route_authorization_revision
            && seal.route_authorization_digest == authorization.route_authorization_digest
            && seal.adapter_id == adapter.adapter_id
            && seal.adapter_revision == adapter.adapter_revision
            && seal.adapter_registry_digest == adapter.adapter_digest
            && seal.credential_id == credential.credential_id
            && seal.credential_revision == credential.credential_revision
            && seal.credential_digest == credential.credential_digest
            && usize::try_from(seal.capability_count).ok() == Some(route.capabilities.len())
            && seal.capability_set_digest == capability_digest,
        "canonical route seal roots mismatch"
    );
    Ok(AuthorizedComputeRouteAuthorization {
        inputs: ValidatedComputeRouteAuthorizationInputs {
            authorization,
            adapter: ValidatedComputeRouteAdapterVersion { envelope: adapter },
            credential: VerifiedComputeRouteCredential {
                envelope: credential,
                adapter: ValidatedComputeRouteAdapterVersion {
                    envelope: credential_adapter,
                },
            },
            actor: AuthorizedComputeServiceActor { envelope: actor },
        },
        seal,
    })
}
