use std::fmt;

use super::{mutation_fence_client::MinifilterFenceLease, ManagedObjectBinding};

/// The first supported hard-fence backend. A Windows directory oplock is deliberately excluded:
/// directory child-change breaks are advisory and do not wait for acknowledgement.
pub(crate) const WINDOWS_MINIFILTER_MUTATION_FENCE_V1: &str =
    "windows_signed_minifilter_child_namespace_fence_v1";

/// Linear lease for an OS-enforced child-namespace mutation fence.
///
/// There is intentionally no safe constructor yet. The only acceptable future constructor is a
/// verified minifilter grant that atomically fences the exact parent FileId and child name before
/// returning, reports every break/timeout/disconnect as outcome-uncertain, and retains the grant
/// through the physical durability capability. Advisory directory oplocks and process locks must
/// never construct this type.
#[must_use = "dropping user mode does not prove that the kernel deny rule was released"]
pub(crate) struct ManagedNamespaceMutationFence {
    lease: MinifilterFenceLease,
}

impl ManagedNamespaceMutationFence {
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
    ) -> std::io::Result<()> {
        self.lease.validate_binding(
            binding,
            cleanup_id,
            execution_plan_digest,
            authorization_receipt_digest,
            expected_object_digest,
            installation_id_digest,
            authority_epoch,
            process_owner_epoch,
            step_ordinal,
        )
    }

    /// A future inhabited backend must replace this fail-closed stub with an exact kernel
    /// `QueryFence` and only return success for the same active grant generation. Keeping the
    /// call sites live now prevents adding a constructor without also implementing liveness.
    pub(super) fn ensure_active(&self) -> std::io::Result<()> {
        self.lease.ensure_active()
    }
}

impl fmt::Debug for ManagedNamespaceMutationFence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedNamespaceMutationFence")
            .field("backend_kind", &WINDOWS_MINIFILTER_MUTATION_FENCE_V1)
            .field("lease", &self.lease)
            .field("execution_scope", &"<redacted>")
            .field("kernel_lease", &"<retained>")
            .finish()
    }
}
