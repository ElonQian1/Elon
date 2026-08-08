use std::{convert::Infallible, fmt, io};

use super::{
    mutation_fence_protocol::{
        MinifilterDigest, MinifilterFenceGrantReceipt, MINIFILTER_FENCE_REQUIRED_FEATURE_MASK,
    },
    ManagedObjectBinding,
};

/// User-mode failure classification is deliberately independent from Win32 error codes. Once an
/// Acquire message may have reached the driver, a timeout or disconnect is never downgraded to a
/// clean rejection because the deny rule can already exist in kernel state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MinifilterFenceClientFailureClass {
    DefinitiveBeforeGrant,
    RejectedByDriver,
    OutcomeUncertain,
    LeasePoisoned,
    ReleaseOutcomeUncertain,
}

#[derive(Debug)]
pub(super) struct MinifilterFenceClientFailure {
    pub(super) class: MinifilterFenceClientFailureClass,
    pub(super) error: io::Error,
}

/// Exact active grant plus the authenticated Filter Manager connection that owns it.
///
/// The connection is intentionally uninhabited until the signed minifilter transport and
/// first-party component-version gate exist. Parsing a syntactically valid reply never constructs
/// this lease. The future constructor must validate the installed component manifest, connect-port
/// process identity, session receipt, parent handle facts and exact Acquire reply together.
#[must_use = "dropping user mode must not be treated as releasing the kernel deny rule"]
pub(super) struct MinifilterFenceLease {
    receipt: MinifilterFenceGrantReceipt,
    expected_driver_build_digest: MinifilterDigest,
    expected_wire_schema_digest: MinifilterDigest,
    _authenticated_driver_connection_unavailable: Infallible,
}

impl MinifilterFenceLease {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn validate_binding(
        &self,
        binding: &ManagedObjectBinding,
        cleanup_id: &str,
        execution_plan_digest: &str,
        authorization_receipt_digest: &str,
        expected_object_digest: &str,
        installation_id_digest: &str,
        authority_epoch: i64,
        process_owner_epoch: i64,
        step_ordinal: u64,
    ) -> io::Result<()> {
        self.receipt.validate_managed_binding(
            binding,
            cleanup_id,
            execution_plan_digest,
            authorization_receipt_digest,
            expected_object_digest,
            installation_id_digest,
            authority_epoch,
            process_owner_epoch,
            step_ordinal,
            self.expected_driver_build_digest,
            self.expected_wire_schema_digest,
            MINIFILTER_FENCE_REQUIRED_FEATURE_MASK,
        )
    }

    /// This remains a fixed fail-closed seam. The real implementation must issue QueryFence with
    /// the exact secret and boot/session/volume/grant generations, then compare the full snapshot.
    pub(super) fn ensure_active(&self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "NODE_MINIFILTER_FENCE_QUERY_BACKEND_UNAVAILABLE",
        ))
    }
}

impl fmt::Debug for MinifilterFenceLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MinifilterFenceLease")
            .field("receipt", &self.receipt)
            .field("driver_connection", &"<retained-unavailable>")
            .finish()
    }
}
