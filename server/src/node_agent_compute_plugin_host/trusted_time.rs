use std::{fmt, time::Instant};

use chrono::{DateTime, Utc};

/// One authenticated observation emitted by the future node trusted-time kernel. This type has no
/// production constructor yet: ordinary wall clock, caller-provided milliseconds and arithmetic
/// over the persisted high-water are deliberately unable to mint it.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginTrustedTimeObservation {
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    installation_id_digest: String,
    clock_epoch_digest: String,
}

impl ComputePluginTrustedTimeObservation {
    pub(in crate::node_agent_compute_plugin_host) fn trusted_now(&self) -> &DateTime<Utc> {
        &self.trusted_now
    }

    pub(in crate::node_agent_compute_plugin_host) fn observed_at(&self) -> Instant {
        self.observed_at
    }

    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        &self.installation_id_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn clock_epoch_digest(&self) -> &str {
        &self.clock_epoch_digest
    }
}

impl fmt::Debug for ComputePluginTrustedTimeObservation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginTrustedTimeObservation")
            .field("trusted_now", &self.trusted_now)
            .field("observed_at", &"<monotonic>")
            .field("installation_id_digest", &"<redacted>")
            .field("clock_epoch_digest", &"<redacted>")
            .finish()
    }
}
