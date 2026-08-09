use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, AtomicU8, Ordering},
        Arc,
    },
};

use anyhow::{bail, Result};
use elon_pc_dev_runtime::NodeDataPaths;

use crate::{
    node_agent_compute_plugin_host::local_authority::ComputePluginLocalAuthority,
    node_agent_instance_lock::{NodeAgentInstanceLockBinding, NodeAgentInstanceLockLease},
};

const TERMINAL_CONTROLLER_EPOCH: u64 = u64::MAX;
const ACTIVATION_PHASE_ACTIVATING: u8 = 1;
const ACTIVATION_PHASE_PINNED: u8 = 2;
const ACTIVATION_PHASE_RETIRED: u8 = 3;

/// Process-local revocation domain dedicated to root/authority ownership. Policy revisions use a
/// separate generation because changing desired work must not silently reassemble filesystem and
/// instance-lock leases into a new controller.
#[derive(Clone)]
pub(super) struct ComputePluginAuthorityControllerGenerationSource {
    epoch: Arc<AtomicU64>,
    outstanding_epoch: Arc<AtomicU64>,
}

impl ComputePluginAuthorityControllerGenerationSource {
    pub(super) fn new() -> Self {
        Self {
            epoch: Arc::new(AtomicU64::new(1)),
            outstanding_epoch: Arc::new(AtomicU64::new(0)),
        }
    }

    pub(super) fn capture(&self) -> Result<ComputePluginAuthorityControllerGenerationGuard> {
        let observed_epoch = self.epoch.load(Ordering::Acquire);
        if observed_epoch == TERMINAL_CONTROLLER_EPOCH {
            bail!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_RETIRED");
        }
        self.outstanding_epoch
            .compare_exchange(0, observed_epoch, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_ACTIVATION_OUTSTANDING")
            })?;
        if self.epoch.load(Ordering::Acquire) != observed_epoch {
            self.outstanding_epoch.store(0, Ordering::Release);
            bail!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_RETIRED");
        }
        Ok(ComputePluginAuthorityControllerGenerationGuard {
            source: self.clone(),
            observed_epoch,
            marker: None,
        })
    }

    pub(super) fn invalidate_terminal(&self) {
        self.epoch
            .store(TERMINAL_CONTROLLER_EPOCH, Ordering::Release);
        self.outstanding_epoch.store(0, Ordering::Release);
    }

    fn ensure_current(&self, observed_epoch: u64) -> Result<()> {
        if observed_epoch == 0
            || observed_epoch == TERMINAL_CONTROLLER_EPOCH
            || self.epoch.load(Ordering::Acquire) != observed_epoch
            || self.outstanding_epoch.load(Ordering::Acquire) != observed_epoch
        {
            bail!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_RETIRED");
        }
        Ok(())
    }
}

pub(super) struct ComputePluginAuthorityControllerGenerationGuard {
    source: ComputePluginAuthorityControllerGenerationSource,
    pub(super) observed_epoch: u64,
    marker: Option<ComputePluginAuthorityControllerActivationMarker>,
}

impl ComputePluginAuthorityControllerGenerationGuard {
    pub(super) fn bind_marker(&mut self, marker: ComputePluginAuthorityControllerActivationMarker) {
        self.marker = Some(marker);
    }

    fn marker(&self) -> Result<&ComputePluginAuthorityControllerActivationMarker> {
        self.marker.as_ref().ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_GENERATION_UNBOUND")
        })
    }

    pub(super) fn ensure_activating(&self) -> Result<()> {
        self.source.ensure_current(self.observed_epoch)?;
        self.marker()?.ensure_activating()
    }

    pub(super) fn mark_pinned(&self) -> Result<()> {
        self.ensure_activating()?;
        self.marker()?.mark_pinned()
    }

    pub(super) fn ensure_pinned(&self) -> Result<()> {
        self.source.ensure_current(self.observed_epoch)?;
        self.marker()?.ensure_pinned()
    }

    pub(super) fn ensure_pinned_for_instance_lock(
        &self,
        lease: &NodeAgentInstanceLockLease,
    ) -> Result<()> {
        self.ensure_pinned()?;
        if !self
            .marker()?
            .binding
            .instance_lock_binding
            .matches_lease(lease)
        {
            bail!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_INSTANCE_LOCK_CHANGED");
        }
        Ok(())
    }

    pub(super) fn invalidate_terminal(&self) {
        if let Some(marker) = &self.marker {
            marker.retire();
        }
        // Publish the state-visible retirement first. `ensure_*` also checks the marker, so this
        // order cannot authorize work while it prevents a concurrent status read from observing
        // a terminal source behind an apparently live Activating/Pinned marker.
        self.source.invalidate_terminal();
    }
}

impl Drop for ComputePluginAuthorityControllerGenerationGuard {
    fn drop(&mut self) {
        // This guard is deliberately terminal-on-drop. There is no safe path from an abandoned
        // activation or released pinned output back to Dormant in the same Bootstrap lifetime.
        self.invalidate_terminal();
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct ComputePluginAuthorityControllerBinding {
    pub(super) transition_nonce: String,
    pub(super) controller_epoch: u64,
    pub(super) bootstrap_instance_id: String,
    pub(super) node_id: String,
    pub(super) owner_user_id: String,
    pub(super) installation_id_digest: String,
    pub(super) node_data_paths: NodeDataPaths,
    pub(super) compute_plugin_root: PathBuf,
    pub(super) authority_path: PathBuf,
    pub(super) instance_lock_binding: NodeAgentInstanceLockBinding,
}

#[derive(Clone)]
pub(super) struct ComputePluginAuthorityControllerActivationMarker {
    pub(super) binding: ComputePluginAuthorityControllerBinding,
    phase: Arc<AtomicU8>,
}

impl ComputePluginAuthorityControllerActivationMarker {
    pub(super) fn new(binding: ComputePluginAuthorityControllerBinding) -> Self {
        Self {
            binding,
            phase: Arc::new(AtomicU8::new(ACTIVATION_PHASE_ACTIVATING)),
        }
    }

    fn ensure_activating(&self) -> Result<()> {
        if self.phase.load(Ordering::Acquire) != ACTIVATION_PHASE_ACTIVATING {
            bail!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_ACTIVATION_RETIRED");
        }
        Ok(())
    }

    fn mark_pinned(&self) -> Result<()> {
        self.phase
            .compare_exchange(
                ACTIVATION_PHASE_ACTIVATING,
                ACTIVATION_PHASE_PINNED,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map_err(|_| {
                anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_ACTIVATION_RETIRED")
            })?;
        Ok(())
    }

    fn ensure_pinned(&self) -> Result<()> {
        if self.phase.load(Ordering::Acquire) != ACTIVATION_PHASE_PINNED {
            bail!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_PINNED_CUSTODY_RETIRED");
        }
        Ok(())
    }

    pub(super) fn retire(&self) {
        self.phase
            .store(ACTIVATION_PHASE_RETIRED, Ordering::Release);
    }

    pub(super) fn matches(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.phase, &other.phase) && self.binding == other.binding
    }

    pub(super) fn is_activating(&self) -> bool {
        self.phase.load(Ordering::Acquire) == ACTIVATION_PHASE_ACTIVATING
    }

    fn is_retired(&self) -> bool {
        self.phase.load(Ordering::Acquire) == ACTIVATION_PHASE_RETIRED
    }
}

pub(super) enum ComputePluginAuthorityControllerState {
    Unavailable,
    Dormant,
    Activating(ComputePluginAuthorityControllerActivationMarker),
    Pinned(ComputePluginAuthorityControllerActivationMarker),
    Retired(Option<ComputePluginAuthorityControllerBinding>),
}

impl ComputePluginAuthorityControllerState {
    pub(super) fn for_root(root: Option<&DormantComputePluginRootBinding>) -> Self {
        if root.is_some() {
            Self::Dormant
        } else {
            Self::Unavailable
        }
    }

    pub(super) fn ensure_dormant(&self) -> Result<()> {
        match self {
            Self::Dormant => Ok(()),
            Self::Unavailable => bail!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_ROOT_UNAVAILABLE"),
            Self::Activating(_) => {
                bail!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_ACTIVATION_OUTSTANDING")
            }
            Self::Pinned(_) => bail!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_ALREADY_PINNED"),
            Self::Retired(_) => bail!("COMPUTE_PLUGIN_AUTHORITY_CONTROLLER_RETIRED"),
        }
    }

    pub(super) fn publish_activating(
        &mut self,
        marker: ComputePluginAuthorityControllerActivationMarker,
    ) {
        *self = Self::Activating(marker);
    }

    pub(super) fn matches_activating(
        &self,
        marker: &ComputePluginAuthorityControllerActivationMarker,
    ) -> bool {
        matches!(self, Self::Activating(current) if current.matches(marker))
            && marker.is_activating()
    }

    pub(super) fn publish_pinned(
        &mut self,
        marker: ComputePluginAuthorityControllerActivationMarker,
    ) {
        *self = Self::Pinned(marker);
    }

    pub(super) fn retire_preserving_binding(&mut self) {
        let binding = match self {
            Self::Activating(marker) | Self::Pinned(marker) => Some(marker.binding.clone()),
            Self::Retired(binding) => binding.clone(),
            Self::Unavailable | Self::Dormant => None,
        };
        if let Self::Activating(marker) | Self::Pinned(marker) = self {
            marker.retire();
        }
        *self = Self::Retired(binding);
    }

    pub(super) fn retire_without_binding(&mut self) {
        if let Self::Activating(marker) | Self::Pinned(marker) = self {
            marker.retire();
        }
        *self = Self::Retired(None);
    }

    pub(super) fn has_bound_root(&self) -> bool {
        matches!(
            self,
            Self::Activating(_) | Self::Pinned(_) | Self::Retired(Some(_))
        )
    }

    pub(super) fn restart_required(&self) -> bool {
        match self {
            Self::Activating(marker) | Self::Pinned(marker) => marker.is_retired(),
            Self::Retired(_) => true,
            Self::Unavailable | Self::Dormant => false,
        }
    }

    pub(super) fn matches_node_data_paths(&self, paths: &NodeDataPaths) -> bool {
        match self {
            Self::Activating(marker) | Self::Pinned(marker) => {
                marker.binding.node_data_paths == *paths
                    && marker.binding.compute_plugin_root == paths.compute_plugins()
            }
            Self::Retired(Some(binding)) => {
                binding.node_data_paths == *paths
                    && binding.compute_plugin_root == paths.compute_plugins()
            }
            Self::Unavailable | Self::Dormant | Self::Retired(None) => false,
        }
    }

    pub(super) fn authority_path_derived(
        &self,
        dormant: Option<&DormantComputePluginRootBinding>,
    ) -> bool {
        if let Some(dormant) = dormant {
            return dormant.binding_is_exact();
        }
        let binding = match self {
            Self::Activating(marker) | Self::Pinned(marker) => Some(&marker.binding),
            Self::Retired(Some(binding)) => Some(binding),
            Self::Unavailable | Self::Dormant | Self::Retired(None) => None,
        };
        binding.is_some_and(binding_is_exact)
    }
}

pub(super) struct DormantComputePluginRootBinding {
    pub(super) node_data_paths: NodeDataPaths,
    pub(super) compute_plugin_root: PathBuf,
    pub(super) authority: ComputePluginLocalAuthority,
}

impl DormantComputePluginRootBinding {
    pub(super) fn new(paths: &NodeDataPaths) -> Self {
        let compute_plugin_root = paths.compute_plugins();
        let authority = ComputePluginLocalAuthority::for_compute_plugin_root(&compute_plugin_root);
        Self {
            node_data_paths: paths.clone(),
            compute_plugin_root,
            authority,
        }
    }

    pub(super) fn binding_is_exact(&self) -> bool {
        self.node_data_paths.compute_plugins() == self.compute_plugin_root
            && self.authority.path().parent() == Some(self.compute_plugin_root.as_path())
    }
}

pub(super) fn binding_is_exact(binding: &ComputePluginAuthorityControllerBinding) -> bool {
    binding.node_data_paths.compute_plugins() == binding.compute_plugin_root
        && binding.authority_path.parent() == Some(binding.compute_plugin_root.as_path())
}
