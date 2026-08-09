use std::fmt;

use crate::compute_federation::route_authority::{
    AuthorizedComputeRouteAuthorization, AuthorizedComputeServiceActor,
};

use super::types::{
    ComputeAttemptDispatchActorReceiptEnvelope, ComputeLeaseAuthorityBindingEnvelope,
    ComputeStartNoStartProofEnvelope, ComputeStartOutboxClaimReceiptEnvelope,
    ComputeStartOutboxOperationEnvelope, ComputeStartOutboxRemoteObservationEnvelope,
    ComputeStartOutboxSendAttemptEnvelope,
};

/// One authenticated service actor acting in exactly one command phase.
pub(crate) struct AuthorizedComputeAttemptDispatchActorReceipt {
    envelope: ComputeAttemptDispatchActorReceiptEnvelope,
    actor: AuthorizedComputeServiceActor,
}

impl AuthorizedComputeAttemptDispatchActorReceipt {
    pub(crate) fn envelope(&self) -> &ComputeAttemptDispatchActorReceiptEnvelope {
        &self.envelope
    }

    pub(crate) fn actor(&self) -> &AuthorizedComputeServiceActor {
        &self.actor
    }
}

impl fmt::Debug for AuthorizedComputeAttemptDispatchActorReceipt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizedComputeAttemptDispatchActorReceipt")
            .field("actor_receipt_id", &self.envelope.actor_receipt_id)
            .field("actor_phase", &self.envelope.actor_phase)
            .field("command_id", &self.envelope.command_id)
            .field("service_actor_id", &self.envelope.service_actor_id)
            .finish()
    }
}

/// Sealed non-bearer lease authority and the exact application-phase actor receipt that issued it.
pub(crate) struct AuthorizedComputeLeaseAuthorityBinding {
    envelope: ComputeLeaseAuthorityBindingEnvelope,
    application_actor_receipt: AuthorizedComputeAttemptDispatchActorReceipt,
}

impl AuthorizedComputeLeaseAuthorityBinding {
    pub(crate) fn envelope(&self) -> &ComputeLeaseAuthorityBindingEnvelope {
        &self.envelope
    }

    pub(crate) fn application_actor_receipt(
        &self,
    ) -> &AuthorizedComputeAttemptDispatchActorReceipt {
        &self.application_actor_receipt
    }
}

impl fmt::Debug for AuthorizedComputeLeaseAuthorityBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizedComputeLeaseAuthorityBinding")
            .field("lease_authority_id", &self.envelope.lease_authority_id)
            .field("authority_revision", &self.envelope.authority_revision)
            .field("lease_id", &self.envelope.lease_id)
            .field("authority", &"<sealed non-bearer reference>")
            .finish()
    }
}

/// Immutable operation after operation-shape, exact source, route-currentness, actor-phase, and
/// lease-authority checks. Commit carries application authority; all other operations carry the
/// dispatch receipt. There is intentionally no constructor in this batch.
pub(crate) struct ValidatedComputeStartOutboxOperation {
    envelope: ComputeStartOutboxOperationEnvelope,
    route_authorization: AuthorizedComputeRouteAuthorization,
    dispatch_actor_receipt: Option<AuthorizedComputeAttemptDispatchActorReceipt>,
    lease_authority: Option<AuthorizedComputeLeaseAuthorityBinding>,
}

impl ValidatedComputeStartOutboxOperation {
    pub(crate) fn envelope(&self) -> &ComputeStartOutboxOperationEnvelope {
        &self.envelope
    }

    pub(crate) fn route_authorization(&self) -> &AuthorizedComputeRouteAuthorization {
        &self.route_authorization
    }

    pub(crate) fn actor_receipt(&self) -> Option<&AuthorizedComputeAttemptDispatchActorReceipt> {
        self.dispatch_actor_receipt.as_ref().or_else(|| {
            self.lease_authority
                .as_ref()
                .map(AuthorizedComputeLeaseAuthorityBinding::application_actor_receipt)
        })
    }

    pub(crate) fn lease_authority(&self) -> Option<&AuthorizedComputeLeaseAuthorityBinding> {
        self.lease_authority.as_ref()
    }
}

impl fmt::Debug for ValidatedComputeStartOutboxOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedComputeStartOutboxOperation")
            .field("outbox_id", &self.envelope.outbox_id)
            .field("operation_kind", &self.envelope.operation_kind)
            .field("operation_generation", &self.envelope.operation_generation)
            .field("command_id", &self.envelope.command_id)
            .field("has_lease_authority", &self.lease_authority.is_some())
            .finish()
    }
}

/// Current claim receipt bound to the exact immutable operation; raw claim tokens remain absent.
pub(crate) struct AuthorizedComputeStartOutboxClaim {
    receipt: ComputeStartOutboxClaimReceiptEnvelope,
    operation: ValidatedComputeStartOutboxOperation,
}

impl AuthorizedComputeStartOutboxClaim {
    pub(crate) fn receipt(&self) -> &ComputeStartOutboxClaimReceiptEnvelope {
        &self.receipt
    }

    pub(crate) fn operation(&self) -> &ValidatedComputeStartOutboxOperation {
        &self.operation
    }
}

impl fmt::Debug for AuthorizedComputeStartOutboxClaim {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizedComputeStartOutboxClaim")
            .field("claim_receipt_id", &self.receipt.claim_receipt_id)
            .field("outbox_id", &self.receipt.outbox_id)
            .field("claim_generation", &self.receipt.claim_generation)
            .field("claim_token", &"<digest only>")
            .finish()
    }
}

/// Durable send-start evidence, sealed before any network I/O can begin.
pub(crate) struct ValidatedComputeStartOutboxSendAttempt {
    envelope: ComputeStartOutboxSendAttemptEnvelope,
    claim: AuthorizedComputeStartOutboxClaim,
}

impl ValidatedComputeStartOutboxSendAttempt {
    pub(crate) fn envelope(&self) -> &ComputeStartOutboxSendAttemptEnvelope {
        &self.envelope
    }

    pub(crate) fn claim(&self) -> &AuthorizedComputeStartOutboxClaim {
        &self.claim
    }
}

impl fmt::Debug for ValidatedComputeStartOutboxSendAttempt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedComputeStartOutboxSendAttempt")
            .field("send_attempt_id", &self.envelope.send_attempt_id)
            .field("outbox_id", &self.envelope.outbox_id)
            .field("attempt_no", &self.envelope.attempt_no)
            .field("request", &"<digest only>")
            .finish()
    }
}

/// Authenticated Adapter observation bound to a durable send-start fact.
pub(crate) struct VerifiedComputeStartOutboxRemoteObservation {
    envelope: ComputeStartOutboxRemoteObservationEnvelope,
    send_attempt: ValidatedComputeStartOutboxSendAttempt,
}

impl VerifiedComputeStartOutboxRemoteObservation {
    pub(crate) fn envelope(&self) -> &ComputeStartOutboxRemoteObservationEnvelope {
        &self.envelope
    }

    pub(crate) fn send_attempt(&self) -> &ValidatedComputeStartOutboxSendAttempt {
        &self.send_attempt
    }
}

impl fmt::Debug for VerifiedComputeStartOutboxRemoteObservation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedComputeStartOutboxRemoteObservation")
            .field("observation_id", &self.envelope.observation_id)
            .field("observation_kind", &self.envelope.observation_kind)
            .field(
                "remote_execution_state",
                &self.envelope.remote_execution_state,
            )
            .field("verification", &"<authenticated>")
            .finish()
    }
}

/// Store-verified no-start proof. Cancel responses cannot construct this custody type.
pub(crate) struct VerifiedComputeStartNoStartProof {
    envelope: ComputeStartNoStartProofEnvelope,
    observation: Option<VerifiedComputeStartOutboxRemoteObservation>,
    local_operation: Option<ValidatedComputeStartOutboxOperation>,
}

impl VerifiedComputeStartNoStartProof {
    pub(crate) fn envelope(&self) -> &ComputeStartNoStartProofEnvelope {
        &self.envelope
    }

    pub(crate) fn observation(&self) -> Option<&VerifiedComputeStartOutboxRemoteObservation> {
        self.observation.as_ref()
    }

    pub(crate) fn local_operation(&self) -> Option<&ValidatedComputeStartOutboxOperation> {
        self.local_operation.as_ref()
    }
}

impl fmt::Debug for VerifiedComputeStartNoStartProof {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedComputeStartNoStartProof")
            .field("proof_id", &self.envelope.proof_id)
            .field("proof_kind", &self.envelope.proof_kind)
            .field("command_id", &self.envelope.command_id)
            .field("verification", &"<sealed>")
            .finish()
    }
}
