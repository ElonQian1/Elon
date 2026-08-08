use std::time::{Duration, Instant};

use super::{
    ManagedNamespaceDurabilityFailure, ManagedNamespaceDurabilityFailureCustody,
    ManagedNamespaceDurabilityFailurePhase, ManagedNamespaceDurabilityRetainedCustody,
    ManagedNamespaceDurabilityRetainedInner, ManagedNamespaceDurable, ManagedObjectBinding,
    WINDOWS_NAMESPACE_DURABILITY_KIND,
};

pub(super) fn retained(
    inner: ManagedNamespaceDurabilityRetainedInner,
) -> ManagedNamespaceDurabilityRetainedCustody {
    ManagedNamespaceDurabilityRetainedCustody { inner }
}

pub(super) fn failure(
    phase: ManagedNamespaceDurabilityFailurePhase,
    error: std::io::Error,
    custody: ManagedNamespaceDurabilityFailureCustody,
) -> ManagedNamespaceDurabilityFailure {
    ManagedNamespaceDurabilityFailure {
        phase,
        error,
        custody,
    }
}

pub(super) fn strictly_later_instant(previous: Instant) -> Instant {
    let now = Instant::now();
    if now > previous {
        now
    } else {
        previous.checked_add(Duration::from_nanos(1)).unwrap_or(now)
    }
}

impl ManagedNamespaceDurable {
    pub(crate) fn ensure_mutation_fence_active(&self) -> std::io::Result<()> {
        self._mutation_fence.ensure_active()
    }

    pub(crate) fn object_binding(&self) -> &ManagedObjectBinding {
        &self.custody.binding
    }

    pub(crate) fn namespace_durability_kind(&self) -> &'static str {
        WINDOWS_NAMESPACE_DURABILITY_KIND
    }

    pub(crate) fn filesystem_kind(&self) -> &'static str {
        self.filesystem_kind
    }

    pub(crate) fn barrier_completed_at(&self) -> Instant {
        self.barrier_completed_at
    }

    pub(crate) fn post_absence_observed_at(&self) -> Instant {
        self.post_absence_observed_at
    }

    pub(crate) fn completed_at(&self) -> Instant {
        self.completed_at
    }
}

impl ManagedNamespaceDurabilityFailure {
    pub(crate) fn phase(&self) -> ManagedNamespaceDurabilityFailurePhase {
        self.phase
    }

    pub(crate) fn into_parts(self) -> (std::io::Error, ManagedNamespaceDurabilityFailureCustody) {
        (self.error, self.custody)
    }
}
