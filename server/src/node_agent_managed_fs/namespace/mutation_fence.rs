use std::{convert::Infallible, fmt};

use super::ManagedObjectBinding;

/// The first supported hard-fence backend. A Windows directory oplock is deliberately excluded:
/// directory child-change breaks are advisory and do not wait for acknowledgement.
pub(crate) const WINDOWS_MINIFILTER_MUTATION_FENCE_V1: &str =
    "windows_signed_minifilter_child_namespace_fence_v1";

/// Exact namespace scope that a future kernel grant must bind before user mode can observe
/// absence. Keeping the whole handle-derived binding prevents a lease for one parent/name from
/// authorizing another cleanup step.
#[must_use = "mutation-fence scope must remain bound to its kernel lease"]
struct ManagedNamespaceMutationFenceScope {
    binding: ManagedObjectBinding,
}

/// Linear lease for an OS-enforced child-namespace mutation fence.
///
/// There is intentionally no safe constructor yet. The only acceptable future constructor is a
/// verified minifilter grant that atomically fences the exact parent FileId and child name before
/// returning, reports every break/timeout/disconnect as outcome-uncertain, and retains the grant
/// through the physical durability capability. Advisory directory oplocks and process locks must
/// never construct this type.
#[must_use = "dropping the fence releases kernel namespace exclusion"]
pub(crate) struct ManagedNamespaceMutationFence {
    scope: ManagedNamespaceMutationFenceScope,
    backend_kind: &'static str,
    grant_id: String,
    cleanup_id: String,
    execution_plan_digest: String,
    installation_id_digest: String,
    authority_epoch: i64,
    process_owner_epoch: i64,
    _kernel_lease_unavailable: Infallible,
}

impl ManagedNamespaceMutationFence {
    pub(super) fn validate_binding(
        &self,
        binding: &ManagedObjectBinding,
        cleanup_id: &str,
        execution_plan_digest: &str,
        installation_id_digest: &str,
        authority_epoch: i64,
        process_owner_epoch: i64,
    ) -> std::io::Result<()> {
        if self.scope.binding != *binding
            || self.backend_kind != WINDOWS_MINIFILTER_MUTATION_FENCE_V1
            || self.grant_id.is_empty()
            || self.cleanup_id != cleanup_id
            || self.execution_plan_digest != execution_plan_digest
            || self.installation_id_digest != installation_id_digest
            || self.authority_epoch != authority_epoch
            || self.process_owner_epoch != process_owner_epoch
            || authority_epoch <= 0
            || process_owner_epoch <= 0
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "NODE_MANAGED_NAMESPACE_MUTATION_FENCE_BINDING_CHANGED",
            ));
        }
        Ok(())
    }

    /// A future inhabited backend must replace this fail-closed stub with an exact kernel
    /// `QueryFence` and only return success for the same active grant generation. Keeping the
    /// call sites live now prevents adding a constructor without also implementing liveness.
    pub(super) fn ensure_active(&self) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "NODE_MANAGED_NAMESPACE_MUTATION_FENCE_BACKEND_UNAVAILABLE",
        ))
    }
}

impl fmt::Debug for ManagedNamespaceMutationFence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedNamespaceMutationFence")
            .field("scope", &"<handle-derived>")
            .field("backend_kind", &self.backend_kind)
            .field("grant_id", &"<redacted>")
            .field("execution_scope", &"<redacted>")
            .field("kernel_lease", &"<retained>")
            .finish()
    }
}
