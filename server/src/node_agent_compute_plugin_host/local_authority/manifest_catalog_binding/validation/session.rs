use std::time::Instant;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use crate::node_agent_compute_plugin_host::{
    local_authority::{
        opened_authority::OpenedComputePluginLocalAuthority,
        process_ownership::ComputePluginFetchProcessFence,
    },
    manifest_validation::is_sha256,
    trusted_time::ComputePluginTrustedTimeObservation,
};

pub(super) struct ManifestCatalogBindingSession {
    pub trusted_now: DateTime<Utc>,
    pub prepared_at: Instant,
    pub clock_epoch_digest: String,
}

pub(super) fn validate_session(
    authority: &OpenedComputePluginLocalAuthority,
    process_fence: &ComputePluginFetchProcessFence,
    observation: &ComputePluginTrustedTimeObservation,
) -> Result<ManifestCatalogBindingSession> {
    observation.ensure_live(Instant::now())?;
    if !authority
        .authority_instance_binding()
        .matches(process_fence.authority_instance_binding())
        || authority.installation_id_digest() != process_fence.installation_id_digest()
        || authority.installation_id_digest() != observation.installation_id_digest()
        || !is_sha256(authority.installation_id_digest())
        || !is_sha256(authority.root_identity_digest())
        || observation.clock_epoch_digest() != process_fence.clock_epoch_digest()
        || observation.observed_at() <= process_fence.acquired_observed_at()
        || observation.trusted_now().timestamp_millis() <= process_fence.acquired_at_ms()
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_AUTHORITY_SESSION_CHANGED");
    }
    Ok(ManifestCatalogBindingSession {
        trusted_now: observation.trusted_now().to_owned(),
        prepared_at: observation.observed_at(),
        clock_epoch_digest: observation.clock_epoch_digest().to_string(),
    })
}
