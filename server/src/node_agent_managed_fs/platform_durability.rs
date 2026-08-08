pub(in crate::node_agent_managed_fs) struct PlatformNamespaceDurabilityReceipt {
    filesystem_kind: &'static str,
}

impl PlatformNamespaceDurabilityReceipt {
    pub(in crate::node_agent_managed_fs) fn new(filesystem_kind: &'static str) -> Self {
        Self { filesystem_kind }
    }

    pub(in crate::node_agent_managed_fs) fn filesystem_kind(&self) -> &'static str {
        self.filesystem_kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_managed_fs) enum PlatformNamespaceFlushFailureKind {
    RetryableBeforeBarrier,
    OutcomeUncertain,
    PlatformUnsupported,
}

pub(in crate::node_agent_managed_fs) struct PlatformNamespaceFlushFailure {
    error: std::io::Error,
    kind: PlatformNamespaceFlushFailureKind,
}

impl PlatformNamespaceFlushFailure {
    pub(in crate::node_agent_managed_fs) fn retryable(error: std::io::Error) -> Self {
        Self {
            error,
            kind: PlatformNamespaceFlushFailureKind::RetryableBeforeBarrier,
        }
    }

    pub(in crate::node_agent_managed_fs) fn outcome_uncertain(error: std::io::Error) -> Self {
        Self {
            error,
            kind: PlatformNamespaceFlushFailureKind::OutcomeUncertain,
        }
    }

    pub(in crate::node_agent_managed_fs) fn unsupported(error: std::io::Error) -> Self {
        Self {
            error,
            kind: PlatformNamespaceFlushFailureKind::PlatformUnsupported,
        }
    }

    pub(in crate::node_agent_managed_fs) fn into_parts(
        self,
    ) -> (std::io::Error, PlatformNamespaceFlushFailureKind) {
        (self.error, self.kind)
    }
}
