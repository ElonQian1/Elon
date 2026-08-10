use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

use anyhow::{bail, Result};

use super::super::{
    ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
    OpenedComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    install_plan_admission_validation::is_identifier,
    keyring::ComputePluginBootstrapRootKeyResolver, manifest_validation::is_sha256,
    rollback_anchor::ComputePluginRollbackAnchorStartupPermitV2,
    trusted_time::ComputePluginTrustedTimeObservation,
};

const MAX_IJSON_INTEGER: u64 = 9_007_199_254_740_991;

/// One process-local fence for the future controller -> planning hand-off.
///
/// There is intentionally no constructor in A1. A later production controller must mint this
/// value while atomically transferring its root lock, Bootstrap generation and request witness;
/// copying the scalar fields below will never be sufficient.
struct ComputePluginPlanningControllerFence {
    live: Arc<AtomicBool>,
    bootstrap_instance_id: String,
    configuration_generation: u64,
    cancellation_generation: u64,
    planning_request_digest: String,
    source_preparation_id: String,
    node_id: String,
    owner_user_id: String,
    account_binding_digest: String,
    node_profile_digest: String,
    target_id: String,
    host_api_protocol_id: String,
    host_api_revision: u32,
}

impl ComputePluginPlanningControllerFence {
    fn ensure_current(&self) -> Result<()> {
        if !self.live.load(Ordering::Acquire)
            || !is_identifier(&self.bootstrap_instance_id)
            || !(1..=MAX_IJSON_INTEGER).contains(&self.configuration_generation)
            || !(1..=MAX_IJSON_INTEGER).contains(&self.cancellation_generation)
            || !is_sha256(&self.planning_request_digest)
            || !is_identifier(&self.source_preparation_id)
            || !is_identifier(&self.node_id)
            || !is_identifier(&self.owner_user_id)
            || !is_sha256(&self.account_binding_digest)
            || !is_sha256(&self.node_profile_digest)
            || !is_identifier(&self.target_id)
            || !is_identifier(&self.host_api_protocol_id)
            || self.host_api_revision == 0
        {
            bail!("COMPUTE_PLUGIN_PLANNING_CONTROLLER_FENCE_CHANGED");
        }
        Ok(())
    }
}

impl Drop for ComputePluginPlanningControllerFence {
    fn drop(&mut self) {
        self.live.store(false, Ordering::Release);
    }
}

/// V2 rollback permission with the local opened-authority/root custody that today's public
/// startup permit discards. It has no constructor until assessment transfers the guarded local
/// checkpoint instead of projecting only remote witness scalars.
struct ComputePluginPlanningRollbackCustodyV2 {
    permit: ComputePluginRollbackAnchorStartupPermitV2,
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    root_identity_digest: String,
}

/// Process-local liveness for the exact Bootstrap root set used by the resolver. A future
/// production provider must transfer this custody together with the resolver; a naked trait
/// object is never sufficient to construct planning authority.
struct ComputePluginPlanningBootstrapRootCustody<'a> {
    live: Arc<AtomicBool>,
    resolver: &'a dyn ComputePluginBootstrapRootKeyResolver,
    root_set_digest: String,
}

impl ComputePluginPlanningBootstrapRootCustody<'_> {
    fn ensure_current(&self) -> Result<()> {
        if !self.live.load(Ordering::Acquire) || !is_sha256(&self.root_set_digest) {
            bail!("COMPUTE_PLUGIN_PLANNING_BOOTSTRAP_ROOT_CHANGED");
        }
        Ok(())
    }
}

impl Drop for ComputePluginPlanningBootstrapRootCustody<'_> {
    fn drop(&mut self) {
        self.live.store(false, Ordering::Release);
    }
}

/// Linear, non-serializable prerequisites for one coherent A1 read.
///
/// Fields are private and A1 deliberately provides no constructor. In particular, a caller cannot
/// combine a legacy path-opened process fence, an arbitrary root resolver and a remote rollback
/// permit. The eventual constructor must live at the controller/VFS/rollback custody hand-off.
#[must_use = "dropping planning custody abandons the unopened coherent projection"]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPlanningSnapshotReadCustody<'a> {
    controller: ComputePluginPlanningControllerFence,
    process_fence: &'a ComputePluginFetchProcessFence,
    trusted_time: &'a ComputePluginTrustedTimeObservation,
    rollback: ComputePluginPlanningRollbackCustodyV2,
    bootstrap_roots: ComputePluginPlanningBootstrapRootCustody<'a>,
}

impl<'a> ComputePluginPlanningSnapshotReadCustody<'a> {
    pub(super) fn ensure_external_current(&self) -> Result<()> {
        self.controller.ensure_current()?;
        self.bootstrap_roots.ensure_current()?;
        self.process_fence.ensure_process_owner_current()?;
        self.trusted_time.ensure_live(Instant::now())
    }

    pub(super) fn ensure_for_opened(
        &self,
        opened: &OpenedComputePluginLocalAuthority,
    ) -> Result<()> {
        opened.ensure_current()?;
        self.ensure_external_current()?;
        if !opened
            .authority_instance_binding()
            .matches(self.process_fence.authority_instance_binding())
            || !opened
                .authority_instance_binding()
                .matches(&self.rollback.authority_instance_binding)
            || opened.installation_id_digest() != self.process_fence.installation_id_digest()
            || opened.installation_id_digest() != self.trusted_time.installation_id_digest()
            || opened.root_identity_digest() != self.rollback.root_identity_digest
            || self.process_fence.clock_epoch_digest() != self.trusted_time.clock_epoch_digest()
            || self.rollback.permit.verified_at() > Instant::now()
        {
            bail!("COMPUTE_PLUGIN_PLANNING_CUSTODY_CHANGED");
        }
        Ok(())
    }

    pub(super) fn process_fence(&self) -> &ComputePluginFetchProcessFence {
        self.process_fence
    }

    pub(super) fn trusted_time(&self) -> &ComputePluginTrustedTimeObservation {
        self.trusted_time
    }

    pub(super) fn rollback_permit(&self) -> &ComputePluginRollbackAnchorStartupPermitV2 {
        &self.rollback.permit
    }

    pub(super) fn bootstrap_roots(&self) -> &dyn ComputePluginBootstrapRootKeyResolver {
        self.bootstrap_roots.resolver
    }

    pub(super) fn bootstrap_root_set_digest(&self) -> &str {
        &self.bootstrap_roots.root_set_digest
    }

    pub(super) fn bootstrap_instance_id(&self) -> &str {
        &self.controller.bootstrap_instance_id
    }

    pub(super) fn configuration_generation(&self) -> u64 {
        self.controller.configuration_generation
    }

    pub(super) fn cancellation_generation(&self) -> u64 {
        self.controller.cancellation_generation
    }

    pub(super) fn planning_request_digest(&self) -> &str {
        &self.controller.planning_request_digest
    }

    pub(super) fn source_preparation_id(&self) -> &str {
        &self.controller.source_preparation_id
    }

    pub(super) fn node_id(&self) -> &str {
        &self.controller.node_id
    }

    pub(super) fn owner_user_id(&self) -> &str {
        &self.controller.owner_user_id
    }

    pub(super) fn account_binding_digest(&self) -> &str {
        &self.controller.account_binding_digest
    }

    pub(super) fn node_profile_digest(&self) -> &str {
        &self.controller.node_profile_digest
    }

    pub(super) fn target_id(&self) -> &str {
        &self.controller.target_id
    }

    pub(super) fn host_api_protocol_id(&self) -> &str {
        &self.controller.host_api_protocol_id
    }

    pub(super) fn host_api_revision(&self) -> u32 {
        self.controller.host_api_revision
    }
}
