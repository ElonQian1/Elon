use std::fmt;

use super::types::{
    ComputeRouteAdapterVersion, ComputeRouteAdapterVersionEnvelope,
    ComputeRouteAuthorizationEnvelope, ComputeRouteAuthorizationSealEnvelope,
    ComputeRouteCredential, ComputeRouteCredentialEnvelope,
    ComputeRouteCredentialRevocationEnvelope, ComputeServiceActorAuthorization,
    ComputeServiceActorAuthorizationEnvelope,
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
