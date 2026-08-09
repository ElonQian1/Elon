use std::{fmt, path::PathBuf, sync::Mutex};

use anyhow::{bail, Result};
use elon_pc_dev_runtime::NodeDataPaths;
use homecli_proto::{
    ComputePluginInstallPlanPlanningSnapshotRequestV2, ComputePluginSharingAuthorizationBindingV1,
    ComputePluginSharingPolicyObservedV1, ComputePluginSharingPolicySnapshotV1,
};
use serde::Serialize;

use self::install_plan_preparation::ComputePluginInstallPlanPreparationWitnessV1;
use crate::compute_plugin_sharing_directive::compute_plugin_sharing_policy_snapshot_digest;
use crate::node_agent_instance_lock::NodeAgentInstanceLockWitness;

use super::{
    identity::ComputePluginInstallationIdentity, local_authority::ComputePluginLocalAuthority,
};

pub(crate) const COMPUTE_PLUGIN_BOOTSTRAP_STATUS_SCHEMA: &str =
    "elon.compute_plugin.bootstrap_status.v2";
const BOOTSTRAP_PHASE_DISABLED: &str = "disabled";
const BOOTSTRAP_PHASE_BLOCKED: &str = "blocked";

mod install_plan_planning_snapshot;
mod install_plan_preparation;
mod policy_binding_intent;
mod sharing_policy;

use policy_binding_intent::ComputePluginLocalPolicyBindingGenerationSource;
pub(super) use policy_binding_intent::ComputePluginLocalPolicyBindingIntent;

/// Default-disabled owner for the future plugin runtime controller. Applying a desired policy only
/// replaces dormant in-memory intent. It never pins a root, opens SQLite, performs network I/O,
/// downloads an artifact, admits work, or launches a process.
pub(crate) struct ComputePluginBootstrap {
    bootstrap_instance_id: String,
    policy_binding_generation: ComputePluginLocalPolicyBindingGenerationSource,
    state: Mutex<ComputePluginBootstrapState>,
}

struct ComputePluginBootstrapState {
    installation: Option<ComputePluginInstallationIdentity>,
    root: Option<DormantComputePluginRootBinding>,
    instance_lock: Option<NodeAgentInstanceLockWitness>,
    configuration_generation: u64,
    cancellation_generation: u64,
    policy_binding_generation: ComputePluginLocalPolicyBindingGenerationSource,
    configuration_exhausted: bool,
    root_change_requires_restart: bool,
    sharing_requested: bool,
    desired_policy: Option<AcceptedComputePluginSharingPolicy>,
    authorization_high_water: Option<ComputePluginSharingAuthorizationBindingV1>,
    initialization_plan: Option<DormantComputePluginInitializationPlan>,
    last_install_plan_preparation: Option<ComputePluginInstallPlanPreparationWitnessV1>,
    last_install_plan_planning_snapshot: Option<ComputePluginInstallPlanPlanningSnapshotRequestV2>,
}

#[derive(Clone)]
struct AcceptedComputePluginSharingPolicy {
    snapshot: ComputePluginSharingPolicySnapshotV1,
    snapshot_digest: String,
}

struct DormantComputePluginInitializationPlan {
    snapshot_digest: String,
    policy_revision: u64,
    policy_digest: String,
    authorization: ComputePluginSharingAuthorizationBindingV1,
    cancellation_generation: u64,
}

struct DormantComputePluginRootBinding {
    node_data_paths: NodeDataPaths,
    compute_plugin_root: PathBuf,
    authority: ComputePluginLocalAuthority,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ComputePluginBootstrapStatus {
    pub schema: &'static str,
    pub bootstrap_instance_id: String,
    pub phase: &'static str,
    pub configuration_generation: u64,
    pub cancellation_generation: u64,
    pub sharing_requested: bool,
    pub desired_policy_revision: Option<u64>,
    pub desired_policy_digest: Option<String>,
    pub desired_snapshot_digest: Option<String>,
    pub authorization_revision: Option<u64>,
    pub authorization_digest: Option<String>,
    pub initialization_plan_prepared: bool,
    pub install_plan_preparation_observed: bool,
    pub install_plan_context_ready: bool,
    pub local_confirmation_available: bool,
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
    pub new_work_admission_enabled: bool,
    pub downloads_allowed: bool,
    pub side_effects_started: bool,
    pub restart_required: bool,
    pub blocked_reasons: Vec<&'static str>,
}

impl ComputePluginBootstrap {
    pub(crate) fn new_disabled(install_id: &str, node_data_paths: Option<&NodeDataPaths>) -> Self {
        let policy_binding_generation = ComputePluginLocalPolicyBindingGenerationSource::new();
        Self {
            bootstrap_instance_id: uuid::Uuid::new_v4().to_string(),
            policy_binding_generation: policy_binding_generation.clone(),
            state: Mutex::new(ComputePluginBootstrapState {
                installation: ComputePluginInstallationIdentity::derive(install_id).ok(),
                root: node_data_paths.map(DormantComputePluginRootBinding::new),
                instance_lock: None,
                configuration_generation: 1,
                cancellation_generation: 1,
                policy_binding_generation,
                configuration_exhausted: false,
                root_change_requires_restart: false,
                sharing_requested: false,
                desired_policy: None,
                authorization_high_water: None,
                initialization_plan: None,
                last_install_plan_preparation: None,
                last_install_plan_planning_snapshot: None,
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
        let mut state = self.state.lock().map_err(|_| {
            self.invalidate_policy_binding_intents_after_poison();
            anyhow::anyhow!("COMPUTE_PLUGIN_BOOTSTRAP_STATE_POISONED")
        })?;
        if state.instance_lock.is_some() {
            bail!("COMPUTE_PLUGIN_BOOTSTRAP_INSTANCE_LOCK_ALREADY_BOUND");
        }
        state.advance_configuration_generation();
        state.instance_lock = Some(witness);
        Ok(())
    }

    /// Records one complete desired snapshot after exact local binding and anti-rollback checks.
    /// Enabled snapshots only prepare an inert initialization plan. Disabled snapshots revoke that
    /// plan and advance the independent cancellation generation.
    pub(crate) fn apply_sharing_policy_snapshot_v1(
        &self,
        snapshot: &ComputePluginSharingPolicySnapshotV1,
        session_node_id: &str,
        session_owner_user_id: &str,
    ) -> ComputePluginSharingPolicyObservedV1 {
        let snapshot_digest = match compute_plugin_sharing_policy_snapshot_digest(snapshot) {
            Ok(digest) => digest,
            Err(error) => {
                return self.rejected_observation(
                    session_node_id,
                    session_owner_user_id,
                    error.code(),
                )
            }
        };
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => {
                self.invalidate_policy_binding_intents_after_poison();
                return ComputePluginBootstrapState::poisoned_observation(
                    session_node_id,
                    session_owner_user_id,
                );
            }
        };
        state.apply_policy_snapshot(
            snapshot,
            snapshot_digest,
            session_node_id,
            session_owner_user_id,
        )
    }

    /// Runtime data-root replacement never hot-rebinds plugin authority. The dormant bootstrap is
    /// invalidated and a process restart is required before any future enable path may use it.
    pub(crate) fn note_node_data_root_changed(&self, paths: &NodeDataPaths) {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => {
                self.invalidate_policy_binding_intents_after_poison();
                return;
            }
        };
        let next_root = paths.compute_plugins();
        if state.root.as_ref().is_some_and(|binding| {
            binding.node_data_paths.eq(paths) && binding.compute_plugin_root.eq(&next_root)
        }) {
            return;
        }
        // Revoke every outstanding linear intent before publishing any changed root fact.
        state.advance_configuration_generation();
        state.root = None;
        state.root_change_requires_restart = true;
        state.initialization_plan = None;
        state.last_install_plan_preparation = None;
        state.last_install_plan_planning_snapshot = None;
    }

    pub(crate) fn status(&self) -> ComputePluginBootstrapStatus {
        let mut status = match self.state.lock() {
            Ok(state) => state.status(),
            Err(_) => {
                self.invalidate_policy_binding_intents_after_poison();
                ComputePluginBootstrapStatus::poisoned()
            }
        };
        status.bootstrap_instance_id = self.bootstrap_instance_id.clone();
        status
    }

    /// Produces only an opaque, process-local input for a future authority policy-binding
    /// transition. The returned custody retains the node instance-lock lease, but it does not pin
    /// the compute-plugin root, open SQLite, authorize downloads, or make preparation context ready.
    /// This host-only seam intentionally has no production call site: credential/account changes
    /// do not yet notify Bootstrap, so that hook must invalidate this same generation source before
    /// any authority wiring is allowed.
    pub(super) fn prepare_local_policy_binding_intent(
        &self,
    ) -> Result<ComputePluginLocalPolicyBindingIntent> {
        let state = self.state.lock().map_err(|_| {
            self.invalidate_policy_binding_intents_after_poison();
            anyhow::anyhow!("COMPUTE_PLUGIN_BOOTSTRAP_STATE_POISONED")
        })?;
        state.prepare_local_policy_binding_intent(&self.bootstrap_instance_id)
    }

    fn rejected_observation(
        &self,
        node_id: &str,
        owner_user_id: &str,
        error_code: &'static str,
    ) -> ComputePluginSharingPolicyObservedV1 {
        match self.state.lock() {
            Ok(state) => state.observation(node_id, owner_user_id, false, false, Some(error_code)),
            Err(_) => {
                self.invalidate_policy_binding_intents_after_poison();
                ComputePluginBootstrapState::poisoned_observation(node_id, owner_user_id)
            }
        }
    }

    /// A poisoned state mutex means a transition may have unwound between scalar updates. The
    /// generation source is duplicated outside that mutex solely so every previously minted
    /// intent can still be terminally revoked without trusting or recovering the poisoned state.
    fn invalidate_policy_binding_intents_after_poison(&self) {
        self.policy_binding_generation.invalidate();
    }
}

impl ComputePluginBootstrapState {
    fn advance_configuration_generation(&mut self) {
        self.invalidate_local_policy_binding_intents();
        self.last_install_plan_preparation = None;
        self.last_install_plan_planning_snapshot = None;
        match self.configuration_generation.checked_add(1) {
            Some(next) => self.configuration_generation = next,
            None => self.configuration_exhausted = true,
        }
    }

    fn invalidate_local_policy_binding_intents(&mut self) -> bool {
        let usable = self.policy_binding_generation.invalidate();
        if !usable {
            self.configuration_exhausted = true;
        }
        usable
    }

    fn status(&self) -> ComputePluginBootstrapStatus {
        let installation_identity_valid = self.installation.is_some();
        let node_data_root_bound = self.root.is_some();
        let node_instance_lock_live = self
            .instance_lock
            .as_ref()
            .is_some_and(NodeAgentInstanceLockWitness::is_live);
        let authority_path_derived = self.root.as_ref().is_some_and(|binding| {
            binding.node_data_paths.compute_plugins().as_path()
                == binding.compute_plugin_root.as_path()
                && binding.authority.path().parent() == Some(binding.compute_plugin_root.as_path())
        });
        let desired = self.desired_policy.as_ref();
        let authorization = desired.and_then(|current| current.snapshot.authorization.as_ref());
        let initialization_plan_prepared = desired.is_some_and(|current| {
            self.initialization_plan
                .as_ref()
                .is_some_and(|plan| plan.matches(current, self.cancellation_generation))
        });
        let install_plan_preparation_observed = desired.is_some_and(|current| {
            self.last_install_plan_preparation
                .as_ref()
                .is_some_and(|witness| {
                    let request = &witness.request;
                    self.sharing_requested
                        && request.node_id == current.snapshot.node_id
                        && request.owner_user_id == current.snapshot.owner_user_id
                        && self.installation.as_ref().is_some_and(|installation| {
                            request.installation_identity_digest == installation.digest()
                        })
                        && request.policy_revision == current.snapshot.policy_revision
                        && request.policy_digest == current.snapshot.policy_digest
                        && request.policy_snapshot_digest == current.snapshot_digest
                        && current.snapshot.authorization.as_ref() == Some(&request.authorization)
                })
        });
        let mut blocked_reasons = Vec::new();
        if !self.sharing_requested {
            blocked_reasons.push("compute_sharing_disabled");
        }
        if self.sharing_requested && !initialization_plan_prepared {
            blocked_reasons.push("compute_plugin_initialization_plan_unavailable");
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
            bootstrap_instance_id: String::new(),
            phase: if self.sharing_requested {
                BOOTSTRAP_PHASE_BLOCKED
            } else {
                BOOTSTRAP_PHASE_DISABLED
            },
            configuration_generation: self.configuration_generation,
            cancellation_generation: self.cancellation_generation,
            sharing_requested: self.sharing_requested,
            desired_policy_revision: desired.map(|current| current.snapshot.policy_revision),
            desired_policy_digest: desired.map(|current| current.snapshot.policy_digest.clone()),
            desired_snapshot_digest: desired.map(|current| current.snapshot_digest.clone()),
            authorization_revision: authorization.map(|binding| binding.revision),
            authorization_digest: authorization.map(|binding| binding.digest.clone()),
            initialization_plan_prepared,
            install_plan_preparation_observed,
            install_plan_context_ready: false,
            local_confirmation_available: false,
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
            new_work_admission_enabled: false,
            downloads_allowed: false,
            side_effects_started: false,
            restart_required: self.root_change_requires_restart,
            blocked_reasons,
        }
    }
}

impl DormantComputePluginInitializationPlan {
    fn matches(
        &self,
        desired: &AcceptedComputePluginSharingPolicy,
        cancellation_generation: u64,
    ) -> bool {
        desired.snapshot.plugin_runtime_requested
            && desired.snapshot.authorization.as_ref() == Some(&self.authorization)
            && desired.snapshot.policy_revision == self.policy_revision
            && desired.snapshot.policy_digest == self.policy_digest
            && desired.snapshot_digest == self.snapshot_digest
            && self.cancellation_generation == cancellation_generation
    }
}

impl ComputePluginBootstrapStatus {
    fn poisoned() -> Self {
        Self {
            schema: COMPUTE_PLUGIN_BOOTSTRAP_STATUS_SCHEMA,
            bootstrap_instance_id: String::new(),
            phase: BOOTSTRAP_PHASE_BLOCKED,
            configuration_generation: 0,
            cancellation_generation: 0,
            sharing_requested: false,
            desired_policy_revision: None,
            desired_policy_digest: None,
            desired_snapshot_digest: None,
            authorization_revision: None,
            authorization_digest: None,
            initialization_plan_prepared: false,
            install_plan_preparation_observed: false,
            install_plan_context_ready: false,
            local_confirmation_available: false,
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
            new_work_admission_enabled: false,
            downloads_allowed: false,
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
            node_data_paths: paths.clone(),
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

impl Drop for ComputePluginBootstrap {
    fn drop(&mut self) {
        // Intents retain their own Arc-backed guard and instance-lock lease, so dropping the owner
        // must explicitly revoke them before another account/runtime may construct a fresh
        // Bootstrap with an unrelated generation source.
        self.policy_binding_generation.invalidate();
    }
}
