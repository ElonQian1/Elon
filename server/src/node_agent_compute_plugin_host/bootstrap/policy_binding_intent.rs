use std::{
    fmt,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use anyhow::{bail, Result};
use elon_pc_dev_runtime::NodeDataPaths;
use homecli_proto::{
    ComputePluginInstallPlanPreparationRequestV1, ComputePluginSharingAuthorizationBindingV1,
    ComputePluginSharingPolicySnapshotV1,
    COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_V1_SCHEMA,
};

use crate::{
    compute_plugin_sharing_directive::compute_plugin_sharing_policy_snapshot_digest,
    node_agent_instance_lock::NodeAgentInstanceLockLease,
};

use super::ComputePluginBootstrapState;

const GENERATION_EXHAUSTED: u64 = u64::MAX;

/// Shared only by Bootstrap and intents minted from it. Scalar generations are durable binding
/// facts; this process-local source is the non-forgeable revocation edge between a later policy or
/// root transition and every older in-flight intent.
#[derive(Clone)]
pub(super) struct ComputePluginLocalPolicyBindingGenerationSource {
    epoch: Arc<AtomicU64>,
    outstanding_epoch: Arc<AtomicU64>,
}

impl ComputePluginLocalPolicyBindingGenerationSource {
    pub(super) fn new() -> Self {
        Self {
            epoch: Arc::new(AtomicU64::new(1)),
            outstanding_epoch: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Returns false after moving to the terminal fail-closed epoch. No intent may be minted at or
    /// after that epoch, so integer wraparound can never make an old intent current again.
    pub(super) fn invalidate(&self) -> bool {
        let mut current = self.epoch.load(Ordering::Acquire);
        loop {
            if current >= GENERATION_EXHAUSTED - 1 {
                if current == GENERATION_EXHAUSTED {
                    return false;
                }
                match self.epoch.compare_exchange_weak(
                    current,
                    GENERATION_EXHAUSTED,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        self.release_outstanding(current);
                        return false;
                    }
                    Err(observed) => current = observed,
                }
            } else {
                match self.epoch.compare_exchange_weak(
                    current,
                    current + 1,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        self.release_outstanding(current);
                        return true;
                    }
                    Err(observed) => current = observed,
                }
            }
        }
    }

    fn capture(&self) -> Result<ComputePluginLocalPolicyBindingGenerationGuard> {
        let observed_epoch = self.epoch.load(Ordering::Acquire);
        if observed_epoch == GENERATION_EXHAUSTED {
            bail!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_GENERATION_EXHAUSTED");
        }
        self.outstanding_epoch
            .compare_exchange(0, observed_epoch, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                anyhow::anyhow!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_INTENT_ALREADY_OUTSTANDING")
            })?;
        if self.epoch.load(Ordering::Acquire) != observed_epoch {
            self.release_outstanding(observed_epoch);
            bail!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_INTENT_STALE");
        }
        Ok(ComputePluginLocalPolicyBindingGenerationGuard {
            source: Arc::clone(&self.epoch),
            outstanding_epoch: Arc::clone(&self.outstanding_epoch),
            observed_epoch,
        })
    }

    fn release_outstanding(&self, epoch: u64) {
        let _ =
            self.outstanding_epoch
                .compare_exchange(epoch, 0, Ordering::AcqRel, Ordering::Acquire);
    }
}

struct ComputePluginLocalPolicyBindingGenerationGuard {
    source: Arc<AtomicU64>,
    outstanding_epoch: Arc<AtomicU64>,
    observed_epoch: u64,
}

impl ComputePluginLocalPolicyBindingGenerationGuard {
    fn ensure_current(&self) -> Result<()> {
        if self.observed_epoch == GENERATION_EXHAUSTED
            || self.source.load(Ordering::Acquire) != self.observed_epoch
            || self.outstanding_epoch.load(Ordering::Acquire) != self.observed_epoch
        {
            bail!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_INTENT_STALE");
        }
        Ok(())
    }
}

impl Drop for ComputePluginLocalPolicyBindingGenerationGuard {
    fn drop(&mut self) {
        let _ = self.outstanding_epoch.compare_exchange(
            self.observed_epoch,
            0,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }
}

/// Linear, process-local custody for one future no-download authority policy-binding transition.
///
/// This type deliberately implements neither `Clone` nor serde traits. Its private constructor is
/// reachable only while Bootstrap holds the state lock and has revalidated the dormant policy,
/// installation, data-root and generation bindings, plus the initialization/preparation chain for
/// an enabled snapshot. The retained instance-lock lease is necessary process custody, but is not
/// a compute-plugin-root pin or SQLite authorization.
#[must_use = "dropping the local policy-binding intent releases its node instance-lock lease"]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginLocalPolicyBindingIntent {
    preparation_id: Option<String>,
    snapshot: ComputePluginSharingPolicySnapshotV1,
    snapshot_digest: String,
    node_data_paths: NodeDataPaths,
    compute_plugin_root: PathBuf,
    bootstrap_instance_id: String,
    configuration_generation: u64,
    cancellation_generation: u64,
    generation_guard: ComputePluginLocalPolicyBindingGenerationGuard,
    _instance_lock_lease: NodeAgentInstanceLockLease,
}

impl ComputePluginBootstrapState {
    pub(super) fn prepare_local_policy_binding_intent(
        &self,
        bootstrap_instance_id: &str,
    ) -> Result<ComputePluginLocalPolicyBindingIntent> {
        if self.configuration_exhausted {
            bail!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_CONFIGURATION_GENERATION_EXHAUSTED");
        }
        if self.root_change_requires_restart {
            bail!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_DATA_ROOT_RESTART_REQUIRED");
        }
        let installation = self.installation.as_ref().ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_INSTALLATION_UNAVAILABLE")
        })?;
        let root = self.root.as_ref().ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_DATA_ROOT_UNAVAILABLE")
        })?;
        if root.node_data_paths.compute_plugins() != root.compute_plugin_root
            || root.authority.path().parent() != Some(root.compute_plugin_root.as_path())
        {
            bail!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_DATA_ROOT_CHANGED");
        }
        let desired = self.desired_policy.as_ref().ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_POLICY_UNAVAILABLE")
        })?;
        let calculated_snapshot_digest =
            compute_plugin_sharing_policy_snapshot_digest(&desired.snapshot)
                .map_err(|error| anyhow::anyhow!(error.code()))?;
        if calculated_snapshot_digest != desired.snapshot_digest
            || self.sharing_requested != desired.snapshot.plugin_runtime_requested
            || desired.snapshot.installation_identity_digest != installation.digest()
        {
            bail!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_POLICY_CHANGED");
        }
        let preparation_id = if desired.snapshot.plugin_runtime_requested {
            let authorization = desired.snapshot.authorization.as_ref().ok_or_else(|| {
                anyhow::anyhow!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_AUTHORIZATION_UNAVAILABLE")
            })?;
            if authorization.revision != desired.snapshot.policy_revision
                || authorization.digest != desired.snapshot.policy_digest
            {
                bail!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_AUTHORIZATION_CHANGED");
            }
            let initialization_plan = self.initialization_plan.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_INITIALIZATION_PLAN_UNAVAILABLE"
                )
            })?;
            if !initialization_plan.matches(desired, self.cancellation_generation) {
                bail!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_INITIALIZATION_PLAN_CHANGED");
            }
            let preparation = self.last_install_plan_preparation.as_ref().ok_or_else(|| {
                anyhow::anyhow!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_PREPARATION_UNAVAILABLE")
            })?;
            if !preparation_matches_current(
                preparation,
                desired,
                installation.digest(),
                authorization,
            ) {
                bail!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_PREPARATION_CHANGED");
            }
            Some(preparation.preparation_id.clone())
        } else {
            if desired.snapshot.authorization.is_some()
                || self.initialization_plan.is_some()
                || self.last_install_plan_preparation.is_some()
            {
                bail!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_DISABLED_STATE_CHANGED");
            }
            None
        };
        let instance_lock_lease = self
            .instance_lock
            .as_ref()
            .and_then(|witness| witness.try_acquire_lease())
            .ok_or_else(|| {
                anyhow::anyhow!("COMPUTE_PLUGIN_LOCAL_POLICY_BINDING_INSTANCE_LOCK_UNAVAILABLE")
            })?;
        let generation_guard = self.policy_binding_generation.capture()?;

        Ok(ComputePluginLocalPolicyBindingIntent {
            preparation_id,
            snapshot: desired.snapshot.clone(),
            snapshot_digest: desired.snapshot_digest.clone(),
            node_data_paths: root.node_data_paths.clone(),
            compute_plugin_root: root.compute_plugin_root.clone(),
            bootstrap_instance_id: bootstrap_instance_id.to_string(),
            configuration_generation: self.configuration_generation,
            cancellation_generation: self.cancellation_generation,
            generation_guard,
            _instance_lock_lease: instance_lock_lease,
        })
    }
}

fn preparation_matches_current(
    preparation: &ComputePluginInstallPlanPreparationRequestV1,
    desired: &super::AcceptedComputePluginSharingPolicy,
    installation_identity_digest: &str,
    authorization: &ComputePluginSharingAuthorizationBindingV1,
) -> bool {
    preparation.schema == COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_V1_SCHEMA
        && preparation.node_id == desired.snapshot.node_id
        && preparation.owner_user_id == desired.snapshot.owner_user_id
        && preparation.installation_identity_digest == installation_identity_digest
        && preparation.policy_revision == desired.snapshot.policy_revision
        && preparation.policy_digest == desired.snapshot.policy_digest
        && preparation.policy_snapshot_digest == desired.snapshot_digest
        && preparation.authorization == *authorization
}

impl ComputePluginLocalPolicyBindingIntent {
    /// Must be checked immediately before every mutation boundary, again inside the Store
    /// transaction before write, and after exact readback. Scalar generation equality alone does
    /// not revoke an intent that survived a later policy or root transition.
    pub(in crate::node_agent_compute_plugin_host) fn ensure_current(&self) -> Result<()> {
        self.generation_guard.ensure_current()
    }

    pub(in crate::node_agent_compute_plugin_host) fn preparation_id(&self) -> Option<&str> {
        self.preparation_id.as_deref()
    }

    pub(in crate::node_agent_compute_plugin_host) fn snapshot(
        &self,
    ) -> &ComputePluginSharingPolicySnapshotV1 {
        &self.snapshot
    }

    pub(in crate::node_agent_compute_plugin_host) fn node_id(&self) -> &str {
        &self.snapshot.node_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn owner_user_id(&self) -> &str {
        &self.snapshot.owner_user_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn installation_identity_digest(&self) -> &str {
        &self.snapshot.installation_identity_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn node_data_paths(&self) -> &NodeDataPaths {
        &self.node_data_paths
    }

    pub(in crate::node_agent_compute_plugin_host) fn compute_plugin_root(&self) -> &Path {
        &self.compute_plugin_root
    }

    pub(in crate::node_agent_compute_plugin_host) fn policy_revision(&self) -> u64 {
        self.snapshot.policy_revision
    }

    pub(in crate::node_agent_compute_plugin_host) fn policy_digest(&self) -> &str {
        &self.snapshot.policy_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn snapshot_digest(&self) -> &str {
        &self.snapshot_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn authorization(
        &self,
    ) -> Option<&ComputePluginSharingAuthorizationBindingV1> {
        self.snapshot.authorization.as_ref()
    }

    pub(in crate::node_agent_compute_plugin_host) fn bootstrap_instance_id(&self) -> &str {
        &self.bootstrap_instance_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn configuration_generation(&self) -> u64 {
        self.configuration_generation
    }

    pub(in crate::node_agent_compute_plugin_host) fn cancellation_generation(&self) -> u64 {
        self.cancellation_generation
    }
}

impl fmt::Debug for ComputePluginLocalPolicyBindingIntent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginLocalPolicyBindingIntent")
            .field("preparation_id", &self.preparation_id)
            .field("node_id", &self.snapshot.node_id)
            .field(
                "installation_identity_digest",
                &self.snapshot.installation_identity_digest,
            )
            .field("policy_revision", &self.snapshot.policy_revision)
            .field("policy_digest", &self.snapshot.policy_digest)
            .field("snapshot_digest", &self.snapshot_digest)
            .field("bootstrap_instance_id", &self.bootstrap_instance_id)
            .field("configuration_generation", &self.configuration_generation)
            .field("cancellation_generation", &self.cancellation_generation)
            .field("instance_lock", &"<process-local-lease>")
            .finish()
    }
}
