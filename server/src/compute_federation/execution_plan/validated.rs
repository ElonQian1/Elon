use std::fmt;

use super::types::{
    ComputeArtifactAccessEnvelope, ComputeAttemptExecutionPlanEnvelope,
    ComputeExecutionCapabilityEnvelope,
};

/// Authenticated technical readiness. A raw DTO or Provider declaration cannot construct it.
pub(crate) struct VerifiedComputeExecutionCapability {
    envelope: ComputeExecutionCapabilityEnvelope,
}

impl VerifiedComputeExecutionCapability {
    pub(crate) fn envelope(&self) -> &ComputeExecutionCapabilityEnvelope {
        &self.envelope
    }

    pub(crate) fn capability(&self) -> &super::types::ComputeExecutionCapability {
        &self.envelope.capability
    }
}

impl fmt::Debug for VerifiedComputeExecutionCapability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedComputeExecutionCapability")
            .field("capability_id", &self.envelope.capability_id)
            .field("capability_kind", &self.envelope.capability.capability_kind)
            .field("verification", &"<authenticated>")
            .finish()
    }
}

/// Authenticated non-bearer artifact authorization.
pub(crate) struct VerifiedComputeArtifactAccess {
    envelope: ComputeArtifactAccessEnvelope,
}

impl VerifiedComputeArtifactAccess {
    pub(crate) fn envelope(&self) -> &ComputeArtifactAccessEnvelope {
        &self.envelope
    }

    pub(crate) fn access(&self) -> &super::types::ComputeArtifactAccess {
        &self.envelope.access
    }
}

impl fmt::Debug for VerifiedComputeArtifactAccess {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedComputeArtifactAccess")
            .field("access_id", &self.envelope.access_id)
            .field("authorization", &"<authenticated>")
            .finish()
    }
}

/// Linear planner custody. Future authenticated verifier children may assemble it; there is no
/// constructor in the current batch. Store still recomputes canonical digests, source currency,
/// artifact ordering and numeric grants before persisting anything.
pub(crate) struct ValidatedComputeAttemptExecutionPlanInputs {
    plan: ComputeAttemptExecutionPlanEnvelope,
    capability: VerifiedComputeExecutionCapability,
    artifact_accesses: Vec<VerifiedComputeArtifactAccess>,
}

impl ValidatedComputeAttemptExecutionPlanInputs {
    pub(crate) fn plan(&self) -> &ComputeAttemptExecutionPlanEnvelope {
        &self.plan
    }

    pub(crate) fn capability(&self) -> &VerifiedComputeExecutionCapability {
        &self.capability
    }

    pub(crate) fn artifact_accesses(&self) -> &[VerifiedComputeArtifactAccess] {
        &self.artifact_accesses
    }
}

impl fmt::Debug for ValidatedComputeAttemptExecutionPlanInputs {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedComputeAttemptExecutionPlanInputs")
            .field("plan_id", &self.plan.plan_id)
            .field("job_id", &self.plan.plan.attempt.job_id)
            .field("reservation_id", &self.plan.plan.attempt.reservation_id)
            .field("capability", &self.capability)
            .field("artifact_access_count", &self.artifact_accesses.len())
            .finish()
    }
}
