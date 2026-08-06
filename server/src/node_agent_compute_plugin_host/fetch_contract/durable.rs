use anyhow::{bail, Result};

use super::{
    resolution::{commit_download_segment, ComputePluginFetchCommitResult},
    types::DurablyWrittenComputePluginSegment,
};
use crate::node_agent_compute_plugin_host::{
    fetch_file::SyncedComputePluginPartFile,
    install_plan_admission::AdmittedComputePluginInstallPlan,
    keyring::ComputePluginBootstrapRootKeyResolver,
    local_authority::{ComputePluginFetchProcessFence, ComputePluginLocalAuthority},
    manifest_validation::is_sha256,
    trusted_time::ComputePluginTrustedTimeObservation,
};

#[path = "durable/types.rs"]
mod types;

pub(in crate::node_agent_compute_plugin_host) use types::{
    ComputePluginDurableBindPermit, ComputePluginPostSyncBindingFailure,
};

/// Binds same-handle fsync evidence to a trusted observation acquired strictly afterward. The
/// trusted-time type intentionally has no production constructor until its authenticated kernel
/// lands, so ordinary wall-clock values cannot reach Store commit through this seam.
pub(in crate::node_agent_compute_plugin_host) fn bind_durable_download_segment<'authority>(
    synced: SyncedComputePluginPartFile,
    observation: ComputePluginTrustedTimeObservation,
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    roots: &'authority dyn ComputePluginBootstrapRootKeyResolver,
) -> std::result::Result<
    DurablyWrittenComputePluginSegment<'authority>,
    ComputePluginPostSyncBindingFailure,
> {
    let (authorized, file, root_lock_lease, sync_completed_at) =
        synced.into_parts(ComputePluginDurableBindPermit::new());
    let binding = || -> Result<()> {
        if !is_sha256(authorized.installation_id_digest())
            || authorized.installation_id_digest() != observation.installation_id_digest()
            || observation.observed_at() <= sync_completed_at
        {
            bail!("COMPUTE_PLUGIN_POST_SYNC_OBSERVATION_BINDING_CHANGED");
        }
        authorized.ensure_not_canceled()?;
        Ok(())
    };
    if let Err(error) = binding() {
        return Err(ComputePluginPostSyncBindingFailure::new(
            error,
            authorized.into_recovery_key(),
            file,
            root_lock_lease,
        ));
    }
    let resolution_session =
        match authority.bind_post_sync_fetch_authority_session(process_fence, observation, roots) {
            Ok(session) => session,
            Err(error) => {
                return Err(ComputePluginPostSyncBindingFailure::new(
                    error,
                    authorized.into_recovery_key(),
                    file,
                    root_lock_lease,
                ));
            }
        };
    if let Err(error) = authorized
        .validate_recovery_session(resolution_session.authority_session())
        .and_then(|_| authorized.ensure_not_canceled())
        .and_then(|_| {
            if !resolution_session.was_observed_strictly_after(sync_completed_at)
                || resolution_session.trusted_now_ms() <= authorized.prepared_at_ms()
            {
                bail!("COMPUTE_PLUGIN_POST_SYNC_AUTHORITY_TIME_STALE");
            }
            Ok(())
        })
    {
        return Err(ComputePluginPostSyncBindingFailure::new(
            error,
            authorized.into_recovery_key(),
            file,
            root_lock_lease,
        ));
    }
    Ok(DurablyWrittenComputePluginSegment {
        authorized,
        file,
        root_lock_lease,
        resolution_session,
        sync_completed_at,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn commit_durable_download_segment(
    admitted: &AdmittedComputePluginInstallPlan,
    durable: DurablyWrittenComputePluginSegment<'_>,
) -> ComputePluginFetchCommitResult {
    commit_download_segment(admitted, durable)
}
