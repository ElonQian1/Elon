use std::{error::Error as StdError, fmt};

use super::{
    ManagedNamespaceDurabilityFailure, ManagedNamespaceDurabilityFailureCustody,
    ManagedNamespaceDurabilityRetainedCustody, ManagedNamespaceDurabilityRetainedInner,
    ManagedNamespaceDurable, WINDOWS_NAMESPACE_DURABILITY_KIND,
};

impl fmt::Debug for ManagedNamespaceDurable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedNamespaceDurable")
            .field("parent_handle", &"<retained>")
            .field("mutation_fence", &"<retained>")
            .field("durability_kind", &WINDOWS_NAMESPACE_DURABILITY_KIND)
            .field("filesystem_kind", &self.filesystem_kind)
            .field("barrier_completed_at", &self.barrier_completed_at)
            .field("post_absence_observed_at", &self.post_absence_observed_at)
            .field("completed_at", &self.completed_at)
            .finish()
    }
}

impl fmt::Debug for ManagedNamespaceDurabilityRetainedCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let phase = match &self.inner {
            ManagedNamespaceDurabilityRetainedInner::FenceBindingChanged { .. } => {
                "fence_binding_changed"
            }
            ManagedNamespaceDurabilityRetainedInner::FenceOutcomeUncertainBeforeBarrier {
                ..
            } => "fence_outcome_uncertain_before_barrier",
            ManagedNamespaceDurabilityRetainedInner::FenceOutcomeUncertainAfterBarrier {
                ..
            } => "fence_outcome_uncertain_after_barrier",
            ManagedNamespaceDurabilityRetainedInner::PreBarrierQuarantined { .. } => {
                "pre_barrier_quarantined"
            }
            ManagedNamespaceDurabilityRetainedInner::PreBarrierExpectedIdentity { .. } => {
                "pre_barrier_expected_identity"
            }
            ManagedNamespaceDurabilityRetainedInner::PreBarrierIdentityConflict { .. } => {
                "pre_barrier_identity_conflict"
            }
            ManagedNamespaceDurabilityRetainedInner::BarrierOutcomeUncertain { .. } => {
                "barrier_outcome_uncertain"
            }
            ManagedNamespaceDurabilityRetainedInner::BarrierUnsupported { .. } => {
                "barrier_unsupported"
            }
            ManagedNamespaceDurabilityRetainedInner::PostBarrierQuarantined { .. } => {
                "post_barrier_quarantined"
            }
            ManagedNamespaceDurabilityRetainedInner::PostBarrierExpectedIdentity { .. } => {
                "post_barrier_expected_identity"
            }
            ManagedNamespaceDurabilityRetainedInner::PostBarrierIdentityConflict { .. } => {
                "post_barrier_identity_conflict"
            }
        };
        formatter
            .debug_struct("ManagedNamespaceDurabilityRetainedCustody")
            .field("phase", &phase)
            .field("handles", &"<retained>")
            .field("mutation_fence", &"<retained>")
            .finish()
    }
}

impl fmt::Debug for ManagedNamespaceDurabilityFailureCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let custody = match self {
            Self::RetryBeforeBarrier(_) => "retry_before_barrier",
            Self::RetryAfterBarrier(_) => "retry_after_barrier",
            Self::Retained(_) => "retained_terminal",
        };
        formatter
            .debug_struct("ManagedNamespaceDurabilityFailureCustody")
            .field("custody", &custody)
            .finish()
    }
}

impl fmt::Debug for ManagedNamespaceDurabilityFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedNamespaceDurabilityFailure")
            .field("phase", &self.phase)
            .field("error_kind", &self.error.kind())
            .field("raw_os_error", &self.error.raw_os_error())
            .field("custody", &self.custody)
            .finish()
    }
}

impl fmt::Display for ManagedNamespaceDurabilityFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "NODE_MANAGED_NAMESPACE_DURABILITY_FAILED: {}",
            self.error
        )
    }
}

impl StdError for ManagedNamespaceDurabilityFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.error)
    }
}
