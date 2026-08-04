use std::{
    fmt,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use anyhow::{bail, Result};

use crate::node_agent_compute_plugin_host::{
    local_authority::ComputePluginAuthorityInstanceBinding, manifest_validation::is_sha256,
};

const INITIAL_CANCELLATION_EPOCH: u64 = 1;
const EXHAUSTED_CANCELLATION_EPOCH: u64 = u64::MAX;

/// Process-local revocation source for one downloader authority domain. The future Host must
/// invalidate it whenever sharing, plan, publisher authority or candidate ownership changes.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginFetchCancellationSource {
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    process_owner_epoch: i64,
    epoch: Arc<AtomicU64>,
}

/// Snapshot captured before Store claim authorization. Cloning preserves the same observed epoch;
/// it never refreshes authority. The guard is carried by the authorized linear capability.
#[derive(Clone)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginFetchCancellationGuard {
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    process_owner_epoch: i64,
    epoch: Arc<AtomicU64>,
    observed_epoch: u64,
}

impl ComputePluginFetchCancellationSource {
    pub(in crate::node_agent_compute_plugin_host) fn bound(
        authority_instance_binding: ComputePluginAuthorityInstanceBinding,
        installation_id_digest: String,
        process_owner_epoch: i64,
    ) -> Result<Self> {
        if !is_sha256(&installation_id_digest) || process_owner_epoch <= 0 {
            bail!("COMPUTE_PLUGIN_FETCH_CANCELLATION_BINDING_INVALID");
        }
        Ok(Self {
            authority_instance_binding,
            installation_id_digest,
            process_owner_epoch,
            epoch: Arc::new(AtomicU64::new(INITIAL_CANCELLATION_EPOCH)),
        })
    }

    pub(in crate::node_agent_compute_plugin_host) fn snapshot(
        &self,
    ) -> Result<ComputePluginFetchCancellationGuard> {
        let observed_epoch = self.epoch.load(Ordering::Acquire);
        if observed_epoch == EXHAUSTED_CANCELLATION_EPOCH {
            bail!("COMPUTE_PLUGIN_FETCH_CANCELLATION_EPOCH_EXHAUSTED");
        }
        Ok(ComputePluginFetchCancellationGuard {
            authority_instance_binding: self.authority_instance_binding.clone(),
            installation_id_digest: self.installation_id_digest.clone(),
            process_owner_epoch: self.process_owner_epoch,
            epoch: Arc::clone(&self.epoch),
            observed_epoch,
        })
    }

    pub(in crate::node_agent_compute_plugin_host) fn invalidate(&self) -> Result<()> {
        self.epoch
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |epoch| {
                epoch.checked_add(1)
            })
            .map(|_| ())
            .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_CANCELLATION_EPOCH_EXHAUSTED"))
    }
}

impl ComputePluginFetchCancellationGuard {
    pub(in crate::node_agent_compute_plugin_host) fn validate_source(
        &self,
        source: &ComputePluginFetchCancellationSource,
    ) -> Result<()> {
        if !self
            .authority_instance_binding
            .matches(&source.authority_instance_binding)
            || self.installation_id_digest != source.installation_id_digest
            || self.process_owner_epoch != source.process_owner_epoch
            || !Arc::ptr_eq(&self.epoch, &source.epoch)
        {
            bail!("COMPUTE_PLUGIN_FETCH_CANCELLATION_SOURCE_CHANGED");
        }
        Ok(())
    }

    pub(in crate::node_agent_compute_plugin_host) fn ensure_current(&self) -> Result<()> {
        let current = self.epoch.load(Ordering::Acquire);
        if current == EXHAUSTED_CANCELLATION_EPOCH || current != self.observed_epoch {
            bail!("COMPUTE_PLUGIN_FETCH_CANCELED");
        }
        Ok(())
    }
}

impl fmt::Debug for ComputePluginFetchCancellationSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginFetchCancellationSource")
            .field("binding", &"<redacted>")
            .field("epoch", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for ComputePluginFetchCancellationGuard {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginFetchCancellationGuard")
            .field("binding", &"<redacted>")
            .field("epoch", &"<redacted>")
            .finish()
    }
}
