use std::{fmt, path::PathBuf, sync::Mutex};

use anyhow::{bail, Result};
use elon_pc_dev_runtime::NodeDataPaths;
use serde::Serialize;

use crate::node_agent_instance_lock::NodeAgentInstanceLockWitness;

use super::{
    identity::ComputePluginInstallationIdentity, local_authority::ComputePluginLocalAuthority,
};

pub(crate) const COMPUTE_PLUGIN_BOOTSTRAP_STATUS_SCHEMA: &str =
    "elon.compute_plugin.bootstrap_status.v1";
const BOOTSTRAP_PHASE_DISABLED: &str = "disabled";
const BOOTSTRAP_PHASE_BLOCKED: &str = "blocked";

/// Default-disabled owner for the future plugin runtime controller. Construction only derives
/// immutable identities and paths. It never pins a root, opens SQLite, starts networking or
/// launches a process.
pub(crate) struct ComputePluginBootstrap {
    state: Mutex<ComputePluginBootstrapState>,
}

struct ComputePluginBootstrapState {
    installation: Option<ComputePluginInstallationIdentity>,
    root: Option<DormantComputePluginRootBinding>,
    instance_lock: Option<NodeAgentInstanceLockWitness>,
    configuration_generation: u64,
    configuration_exhausted: bool,
    root_change_requires_restart: bool,
    sharing_requested: bool,
}

struct DormantComputePluginRootBinding {
    compute_plugin_root: PathBuf,
    authority: ComputePluginLocalAuthority,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ComputePluginBootstrapStatus {
    pub schema: &'static str,
    pub phase: &'static str,
    pub configuration_generation: u64,
    pub sharing_requested: bool,
    pub installation_identity_valid: bool,
    pub node_data_root_bound: bool,
    pub node_instance_lock_live: bool,
    pub compute_plugin_root_lock_acquired: bool,
    pub authority_path_derived: bool,
    pub trusted_time_authority_configured: bool,
    pub rollback_anchor_witness_configured: bool,
    pub root_pinned: bool,
    pub authority_opened: bool,
    pub process_fence_acquired: bool,
    pub side_effects_started: bool,
    pub restart_required: bool,
    pub blocked_reasons: Vec<&'static str>,
}

impl ComputePluginBootstrap {
    pub(crate) fn new_disabled(install_id: &str, node_data_paths: Option<&NodeDataPaths>) -> Self {
        Self {
            state: Mutex::new(ComputePluginBootstrapState {
                installation: ComputePluginInstallationIdentity::derive(install_id).ok(),
                root: node_data_paths.map(DormantComputePluginRootBinding::new),
                instance_lock: None,
                configuration_generation: 1,
                configuration_exhausted: false,
                root_change_requires_restart: false,
                sharing_requested: false,
            }),
        }
    }

    /// Binds only a weak liveness witness. The bootstrap cannot keep the node state-directory lock
    /// alive. A future side-effect transition must retain its lease and independently acquire a
    /// compute-plugin-root lock bound to the installation and canonical pinned root.
    pub(crate) fn bind_instance_lock(&self, witness: NodeAgentInstanceLockWitness) -> Result<()> {
        let Some(_lock_lease) = witness.try_acquire_lease() else {
            bail!("COMPUTE_PLUGIN_BOOTSTRAP_INSTANCE_LOCK_NOT_LIVE");
        };
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_BOOTSTRAP_STATE_POISONED"))?;
        if state.instance_lock.is_some() {
            bail!("COMPUTE_PLUGIN_BOOTSTRAP_INSTANCE_LOCK_ALREADY_BOUND");
        }
        state.instance_lock = Some(witness);
        state.advance_configuration_generation();
        Ok(())
    }

    /// Runtime data-root replacement never hot-rebinds plugin authority. The dormant bootstrap is
    /// invalidated and a process restart is required before any future enable path may use it.
    pub(crate) fn note_node_data_root_changed(&self, paths: &NodeDataPaths) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let next_root = paths.compute_plugins();
        if state
            .root
            .as_ref()
            .is_some_and(|binding| binding.compute_plugin_root == next_root)
        {
            return;
        }
        state.root = None;
        state.root_change_requires_restart = true;
        state.advance_configuration_generation();
    }

    pub(crate) fn status(&self) -> ComputePluginBootstrapStatus {
        match self.state.lock() {
            Ok(state) => state.status(),
            Err(_) => ComputePluginBootstrapStatus::poisoned(),
        }
    }
}

impl ComputePluginBootstrapState {
    fn advance_configuration_generation(&mut self) {
        match self.configuration_generation.checked_add(1) {
            Some(next) => self.configuration_generation = next,
            None => self.configuration_exhausted = true,
        }
    }

    fn status(&self) -> ComputePluginBootstrapStatus {
        let installation_identity_valid = self.installation.is_some();
        let node_data_root_bound = self.root.is_some();
        let node_instance_lock_live = self
            .instance_lock
            .as_ref()
            .is_some_and(NodeAgentInstanceLockWitness::is_live);
        let authority_path_derived = self.root.as_ref().is_some_and(|binding| {
            binding.authority.path().parent() == Some(binding.compute_plugin_root.as_path())
        });
        let mut blocked_reasons = Vec::new();
        if !self.sharing_requested {
            blocked_reasons.push("compute_sharing_disabled");
        }
        if !installation_identity_valid {
            blocked_reasons.push("installation_identity_invalid");
        }
        if !node_data_root_bound {
            blocked_reasons.push("node_data_root_unavailable");
        }
        if !node_instance_lock_live {
            blocked_reasons.push("node_instance_lock_unavailable");
        }
        if self.root_change_requires_restart {
            blocked_reasons.push("node_data_root_change_requires_restart");
        }
        if self.configuration_exhausted {
            blocked_reasons.push("bootstrap_configuration_generation_exhausted");
        }
        blocked_reasons.push("compute_plugin_root_lock_unavailable");
        blocked_reasons.push("authenticated_trusted_time_unavailable");
        blocked_reasons.push("production_rollback_anchor_witness_unavailable");
        ComputePluginBootstrapStatus {
            schema: COMPUTE_PLUGIN_BOOTSTRAP_STATUS_SCHEMA,
            phase: if self.sharing_requested {
                BOOTSTRAP_PHASE_BLOCKED
            } else {
                BOOTSTRAP_PHASE_DISABLED
            },
            configuration_generation: self.configuration_generation,
            sharing_requested: self.sharing_requested,
            installation_identity_valid,
            node_data_root_bound,
            node_instance_lock_live,
            compute_plugin_root_lock_acquired: false,
            authority_path_derived,
            trusted_time_authority_configured: false,
            rollback_anchor_witness_configured: false,
            root_pinned: false,
            authority_opened: false,
            process_fence_acquired: false,
            side_effects_started: false,
            restart_required: self.root_change_requires_restart,
            blocked_reasons,
        }
    }
}

impl ComputePluginBootstrapStatus {
    fn poisoned() -> Self {
        Self {
            schema: COMPUTE_PLUGIN_BOOTSTRAP_STATUS_SCHEMA,
            phase: BOOTSTRAP_PHASE_BLOCKED,
            configuration_generation: 0,
            sharing_requested: false,
            installation_identity_valid: false,
            node_data_root_bound: false,
            node_instance_lock_live: false,
            compute_plugin_root_lock_acquired: false,
            authority_path_derived: false,
            trusted_time_authority_configured: false,
            rollback_anchor_witness_configured: false,
            root_pinned: false,
            authority_opened: false,
            process_fence_acquired: false,
            side_effects_started: false,
            restart_required: true,
            blocked_reasons: vec!["bootstrap_state_poisoned"],
        }
    }
}

impl DormantComputePluginRootBinding {
    fn new(paths: &NodeDataPaths) -> Self {
        let compute_plugin_root = paths.compute_plugins();
        let authority = ComputePluginLocalAuthority::for_compute_plugin_root(&compute_plugin_root);
        Self {
            compute_plugin_root,
            authority,
        }
    }
}

impl fmt::Debug for ComputePluginBootstrap {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginBootstrap")
            .field("state", &"<process-local>")
            .finish()
    }
}
