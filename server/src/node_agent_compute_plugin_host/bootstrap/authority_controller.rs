use std::fmt;

use anyhow::{bail, Result};

use super::{ComputePluginBootstrap, ComputePluginBootstrapState};
use crate::{
    node_agent_compute_plugin_host::{
        fetch_file::{pin_compute_plugin_root, PinnedComputePluginRoot},
        identity::ComputePluginInstallationIdentity,
        local_authority::ComputePluginLocalAuthority,
    },
    node_agent_instance_lock::NodeAgentInstanceLockLease,
};

use super::authority_controller_state::{
    binding_is_exact, ComputePluginAuthorityControllerActivationMarker,
    ComputePluginAuthorityControllerBinding, ComputePluginAuthorityControllerGenerationGuard,
};
pub(super) use super::authority_controller_state::{
    ComputePluginAuthorityControllerGenerationSource, ComputePluginAuthorityControllerState,
    DormantComputePluginRootBinding,
};

/// First linear half of activation. The instance lease and authority locator come from one state
/// lock acquisition; callers cannot supply either ingredient. Field order makes controller
/// revocation happen before the locator or instance lease is released on abandonment.
#[must_use = "dropping a prepared activation terminally retires this controller generation"]
pub(super) struct PreparedComputePluginAuthorityControllerActivation {
    generation_guard: ComputePluginAuthorityControllerGenerationGuard,
    marker: ComputePluginAuthorityControllerActivationMarker,
    installation: ComputePluginInstallationIdentity,
    dormant: DormantComputePluginRootBinding,
    instance_lock_lease: NodeAgentInstanceLockLease,
}

struct PinnedComputePluginAuthorityControllerActivation {
    generation_guard: ComputePluginAuthorityControllerGenerationGuard,
    marker: ComputePluginAuthorityControllerActivationMarker,
    authority: ComputePluginLocalAuthority,
    root: PinnedComputePluginRoot,
    instance_lock_lease: NodeAgentInstanceLockLease,
}

/// Linear output of the safe front half. It deliberately exposes no opened-authority constructor,
/// SQLite connection, process fence, Store operation or Host capability.
#[must_use = "dropping the pinned controller terminally retires its root/authority custody"]
pub(super) struct PinnedComputePluginAuthorityController {
    _generation_guard: ComputePluginAuthorityControllerGenerationGuard,
    _authority: ComputePluginLocalAuthority,
    _root: PinnedComputePluginRoot,
    _instance_lock_lease: NodeAgentInstanceLockLease,
}

impl ComputePluginBootstrap {
    #[allow(dead_code)]
    pub(super) fn begin_authority_controller_activation(
        &self,
    ) -> Result<PreparedComputePluginAuthorityControllerActivation> {
        let mut state = self.state.lock().map_err(|_| {
            self.invalidate_policy_binding_intents_after_poison();
            anyhow::anyhow!("COMPUTE_PLUGIN_BOOTSTRAP_STATE_POISONED")
        })?;
        match state.begin_authority_controller_activation(
            &self.bootstrap_instance_id,
            &self.authority_controller_generation,
        ) {
            Ok(prepared) => Ok(prepared),
            Err(error) => {
                self.authority_controller_generation.invalidate_terminal();
                state.authority_controller.retire_preserving_binding();
                Err(error)
            }
        }
    }

    /// Performs only existing-root pinning outside the state mutex, then re-locks and exact-matches
    /// the same nonce/epoch/bindings before returning the single pinned output.
    #[allow(dead_code)]
    pub(super) fn complete_authority_controller_activation(
        &self,
        prepared: PreparedComputePluginAuthorityControllerActivation,
    ) -> Result<PinnedComputePluginAuthorityController> {
        let marker = prepared.marker.clone();
        let pinned = match prepared.pin_existing_root() {
            Ok(pinned) => pinned,
            Err(error) => {
                self.retire_failed_activation(&marker);
                return Err(error);
            }
        };
        self.finalize_authority_controller_activation(pinned)
    }

    fn finalize_authority_controller_activation(
        &self,
        pinned: PinnedComputePluginAuthorityControllerActivation,
    ) -> Result<PinnedComputePluginAuthorityController> {
        let mut state = self.state.lock().map_err(|_| {
            pinned.generation_guard.invalidate_terminal();
            self.invalidate_policy_binding_intents_after_poison();
            anyhow::anyhow!("COMPUTE_PLUGIN_BOOTSTRAP_STATE_POISONED")
        })?;
        if let Err(error) = state.validate_pinned_activation(&pinned, &self.bootstrap_instance_id) {
            pinned.generation_guard.invalidate_terminal();
            state.authority_controller.retire_preserving_binding();
            return Err(error);
        }
        if let Err(error) = pinned.generation_guard.mark_pinned() {
            pinned.generation_guard.invalidate_terminal();
            state.authority_controller.retire_preserving_binding();
            return Err(error);
        }
        state
            .authority_controller
            .publish_pinned(pinned.marker.clone());
        Ok(pinned.into_output())
    }

    fn retire_failed_activation(&self, marker: &ComputePluginAuthorityControllerActivationMarker) {
        match self.state.lock() {
            Ok(mut state) => {
                self.authority_controller_generation.invalidate_terminal();
                marker.retire();
                state.authority_controller.retire_preserving_binding();
            }
            Err(_) => {
                marker.retire();
                self.invalidate_policy_binding_intents_after_poison();
            }
        }
    }
}

impl ComputePluginBootstrapState {
    fn begin_authority_controller_activation(
        &mut self,
        bootstrap_instance_id: &str,
        generation: &ComputePluginAuthorityControllerGenerationSource,
    ) -> Result<PreparedComputePluginAuthorityControllerActivation> {
        if self.configuration_exhausted || self.root_change_requires_restart {
            bail!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_CONFIGURATION_RETIRED");
        }
        self.authority_controller.ensure_dormant()?;
        let account = self.account.as_ref().cloned().ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_ACCOUNT_UNAVAILABLE")
        })?;
        let installation = self.installation.as_ref().cloned().ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_INSTALLATION_UNAVAILABLE")
        })?;
        let (node_data_paths, compute_plugin_root, authority_path) = {
            let dormant = self.root.as_ref().ok_or_else(|| {
                anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_ROOT_UNAVAILABLE")
            })?;
            if !dormant.binding_is_exact()
                || bootstrap_instance_id.is_empty()
                || account.node_id.is_empty()
                || account.owner_user_id.is_empty()
            {
                bail!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_BINDING_INVALID");
            }
            (
                dormant.node_data_paths.clone(),
                dormant.compute_plugin_root.clone(),
                dormant.authority.path().to_path_buf(),
            )
        };
        let instance_lock_lease = self
            .instance_lock
            .as_ref()
            .and_then(|witness| witness.try_acquire_lease())
            .ok_or_else(|| {
                anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_INSTANCE_LOCK_UNAVAILABLE")
            })?;
        // Controller activation is the one-way custody handoff away from the dormant legacy
        // path locator. Revoke every earlier policy-binding intent before moving that locator;
        // later policy revisions still use their own generation and do not revoke the controller.
        if !self.invalidate_local_policy_binding_intents() {
            bail!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_POLICY_INTENT_GENERATION_EXHAUSTED");
        }
        let mut generation_guard = generation.capture()?;
        let binding = ComputePluginAuthorityControllerBinding {
            transition_nonce: uuid::Uuid::new_v4().to_string(),
            controller_epoch: generation_guard.observed_epoch,
            bootstrap_instance_id: bootstrap_instance_id.to_string(),
            node_id: account.node_id.clone(),
            owner_user_id: account.owner_user_id.clone(),
            installation_id_digest: installation.digest().to_string(),
            node_data_paths,
            compute_plugin_root,
            authority_path,
        };
        let marker = ComputePluginAuthorityControllerActivationMarker::new(binding);
        generation_guard.bind_marker(marker.clone());
        let Some(dormant) = self.root.take() else {
            generation_guard.invalidate_terminal();
            self.authority_controller.retire_without_binding();
            bail!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_ROOT_UNAVAILABLE");
        };
        self.authority_controller.publish_activating(marker.clone());
        Ok(PreparedComputePluginAuthorityControllerActivation {
            generation_guard,
            marker,
            installation,
            dormant,
            instance_lock_lease,
        })
    }

    fn validate_pinned_activation(
        &self,
        pinned: &PinnedComputePluginAuthorityControllerActivation,
        bootstrap_instance_id: &str,
    ) -> Result<()> {
        let binding = &pinned.marker.binding;
        pinned.generation_guard.ensure_activating()?;
        if !self.authority_controller.matches_activating(&pinned.marker)
            || self.root.is_some()
            || self.account.as_ref().is_none_or(|account| {
                account.node_id != binding.node_id || account.owner_user_id != binding.owner_user_id
            })
            || self
                .installation
                .as_ref()
                .is_none_or(|installation| installation.digest() != binding.installation_id_digest)
            || self
                .instance_lock
                .as_ref()
                .is_none_or(|witness| !witness.is_live())
            || binding.bootstrap_instance_id != bootstrap_instance_id
            || binding.controller_epoch != pinned.generation_guard.observed_epoch
            || !binding_is_exact(binding)
            || pinned.authority.path() != binding.authority_path.as_path()
            || pinned.root.installation_id_digest() != binding.installation_id_digest
            || pinned.root.node_data_paths() != &binding.node_data_paths
            || pinned.root.compute_plugin_root().as_path() != binding.compute_plugin_root.as_path()
        {
            bail!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_FINALIZE_BINDING_CHANGED");
        }
        Ok(())
    }
}

impl PreparedComputePluginAuthorityControllerActivation {
    fn pin_existing_root(self) -> Result<PinnedComputePluginAuthorityControllerActivation> {
        self.generation_guard.ensure_activating()?;
        let root = pin_compute_plugin_root(&self.dormant.node_data_paths, &self.installation)?;
        if let Err(error) = self.generation_guard.ensure_activating() {
            self.generation_guard.invalidate_terminal();
            return Err(error);
        }
        let Self {
            generation_guard,
            marker,
            installation: _,
            dormant,
            instance_lock_lease,
        } = self;
        let DormantComputePluginRootBinding { authority, .. } = dormant;
        Ok(PinnedComputePluginAuthorityControllerActivation {
            generation_guard,
            marker,
            authority,
            root,
            instance_lock_lease,
        })
    }
}

impl PinnedComputePluginAuthorityControllerActivation {
    fn into_output(self) -> PinnedComputePluginAuthorityController {
        PinnedComputePluginAuthorityController {
            _generation_guard: self.generation_guard,
            _authority: self.authority,
            _root: self.root,
            _instance_lock_lease: self.instance_lock_lease,
        }
    }
}

impl PinnedComputePluginAuthorityController {
    /// Every future conversion into an opened authority, VFS namespace or process-fence owner must
    /// call this immediately before consuming custody. No such conversion is exposed in this batch.
    #[allow(dead_code)]
    fn ensure_current(&self) -> Result<()> {
        self._generation_guard.ensure_pinned()
    }
}

impl fmt::Debug for PreparedComputePluginAuthorityControllerActivation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedComputePluginAuthorityControllerActivation")
            .field("binding", &"<redacted>")
            .field("instance_lock", &"<retained>")
            .finish()
    }
}

impl fmt::Debug for PinnedComputePluginAuthorityController {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedComputePluginAuthorityController")
            .field("binding", &"<redacted>")
            .field("root", &"<pinned-retained>")
            .field("authority", &"<dormant-locator>")
            .field("instance_lock", &"<retained>")
            .finish()
    }
}
