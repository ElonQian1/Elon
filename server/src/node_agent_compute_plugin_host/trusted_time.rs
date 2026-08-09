use std::{
    fmt,
    time::{Duration, Instant},
};

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

mod attestation;

const TRUSTED_TIME_OBSERVATION_LIFETIME: Duration = Duration::from_secs(60);

pub(in crate::node_agent_compute_plugin_host) use attestation::{
    begin_trusted_time_challenge, create_trusted_time_clock_epoch, verify_trusted_time_attestation,
    ComputePluginSignedTrustedTimeAttestation, ComputePluginTrustedTimeAttestation,
    ComputePluginTrustedTimeChallenge, ComputePluginTrustedTimeChallengePayload,
    ComputePluginTrustedTimeChallengeRequest, ComputePluginTrustedTimeClockEpoch,
    ComputePluginTrustedTimeKeyResolver,
};

/// One authenticated observation emitted after a challenge-bound time attestation is verified.
/// Ordinary wall clock, caller-provided milliseconds and persisted high-water arithmetic cannot
/// mint this type.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginTrustedTimeObservation {
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    expires_at: Instant,
    installation_id_digest: String,
    clock_epoch_digest: String,
    time_authority_id: String,
    attestation_digest: String,
    attestation_sequence: i64,
    signing_key_fingerprint: String,
}

impl ComputePluginTrustedTimeObservation {
    #[allow(clippy::too_many_arguments)]
    fn from_verified_attestation(
        trusted_now: DateTime<Utc>,
        observed_at: Instant,
        expires_at: Instant,
        installation_id_digest: String,
        clock_epoch_digest: String,
        time_authority_id: String,
        attestation_digest: String,
        attestation_sequence: i64,
        signing_key_fingerprint: String,
    ) -> Self {
        Self {
            trusted_now,
            observed_at,
            expires_at,
            installation_id_digest,
            clock_epoch_digest,
            time_authority_id,
            attestation_digest,
            attestation_sequence,
            signing_key_fingerprint,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn trusted_now(&self) -> &DateTime<Utc> {
        &self.trusted_now
    }

    pub(in crate::node_agent_compute_plugin_host) fn observed_at(&self) -> Instant {
        self.observed_at
    }

    pub(in crate::node_agent_compute_plugin_host) fn ensure_live(
        &self,
        now: Instant,
    ) -> Result<()> {
        if now >= self.expires_at {
            bail!("COMPUTE_PLUGIN_TRUSTED_TIME_OBSERVATION_EXPIRED");
        }
        Ok(())
    }

    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        &self.installation_id_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn clock_epoch_digest(&self) -> &str {
        &self.clock_epoch_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn time_authority_id(&self) -> &str {
        &self.time_authority_id
    }

    pub(in crate::node_agent_compute_plugin_host) fn attestation_digest(&self) -> &str {
        &self.attestation_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn attestation_sequence(&self) -> i64 {
        self.attestation_sequence
    }

    pub(in crate::node_agent_compute_plugin_host) fn signing_key_fingerprint(&self) -> &str {
        &self.signing_key_fingerprint
    }
}

impl fmt::Debug for ComputePluginTrustedTimeObservation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginTrustedTimeObservation")
            .field("trusted_now", &self.trusted_now)
            .field("observed_at", &"<monotonic>")
            .field("expires_at", &"<monotonic>")
            .field("installation_id_digest", &"<redacted>")
            .field("clock_epoch_digest", &"<redacted>")
            .field("time_authority_id", &self.time_authority_id)
            .field("attestation_digest", &"<redacted>")
            .field("attestation_sequence", &self.attestation_sequence)
            .field("signing_key_fingerprint", &"<redacted>")
            .finish()
    }
}
