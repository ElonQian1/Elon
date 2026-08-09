use anyhow::{bail, Result};

use super::PinnedComputePluginAuthorityController;
use crate::node_agent_compute_plugin_host::{
    fetch_file::{
        pin_compute_plugin_authority_sqlite_namespace, PinnedComputePluginAuthoritySqliteNamespace,
    },
    local_authority::{
        ComputePluginAuthorityInstanceBinding, ComputePluginHandleBoundAuthorityOpenIntent,
    },
};

/// Sealed linear custody for the exact namespace and the controller that authorized its derivation.
/// No caller can split these fields or continue namespace I/O after controller retirement.
#[must_use = "dropping authority-open custody terminally retires its controller"]
pub(in crate::node_agent_compute_plugin_host) struct PinnedAuthorityOpenCustody {
    _namespace: PinnedComputePluginAuthoritySqliteNamespace,
    controller: PinnedComputePluginAuthorityController,
}

impl PinnedComputePluginAuthorityController {
    /// The sole controller-to-open-intent conversion. The intent constructor accepts the
    /// unforgeable controller itself rather than separately supplied directories or lock leases.
    #[allow(dead_code)]
    pub(super) fn into_handle_bound_open_intent(
        self,
    ) -> Result<ComputePluginHandleBoundAuthorityOpenIntent> {
        self.ensure_current()?;
        if !self
            ._authority
            .is_handle_bound_locator_for(&self._root.compute_plugin_root())
        {
            bail!("COMPUTE_PLUGIN_HANDLE_BOUND_AUTHORITY_LOCATOR_CHANGED");
        }
        let custody = self.into_handle_bound_open_custody()?;
        ComputePluginHandleBoundAuthorityOpenIntent::from_controller_custody(custody)
    }

    fn into_handle_bound_open_custody(self) -> Result<PinnedAuthorityOpenCustody> {
        self.ensure_current()?;
        let namespace = pin_compute_plugin_authority_sqlite_namespace(&self._root)?;
        self.ensure_current()?;
        Ok(PinnedAuthorityOpenCustody {
            _namespace: namespace,
            controller: self,
        })
    }

    fn ensure_current(&self) -> Result<()> {
        self._generation_guard
            .ensure_pinned_for_instance_lock(&self._instance_lock_lease)
    }

    fn retire(&self) {
        self._generation_guard.invalidate_terminal();
    }

    fn authority_instance_binding(&self) -> &ComputePluginAuthorityInstanceBinding {
        self._authority.instance_binding()
    }

    fn installation_id_digest(&self) -> &str {
        self._root.installation_id_digest()
    }

    fn root_identity_digest(&self) -> &str {
        self._root.root_identity_digest()
    }
}

impl PinnedAuthorityOpenCustody {
    pub(in crate::node_agent_compute_plugin_host) fn ensure_current(&self) -> Result<()> {
        self.controller.ensure_current()
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        self.controller.authority_instance_binding()
    }

    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        self.controller.installation_id_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn root_identity_digest(&self) -> &str {
        self.controller.root_identity_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn retire(&self) {
        self.controller.retire();
    }
}

impl Drop for PinnedAuthorityOpenCustody {
    fn drop(&mut self) {
        // Revoke before automatic field destruction releases namespace/root/instance custody.
        self.retire();
    }
}
