use std::{collections::BTreeSet, error::Error as StdError, fmt, time::Instant};

use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};

use super::{
    candidate_staging_contract::StagedComputePluginCandidateArchive,
    identity::{ComputePluginInstallationIdentity, ComputePluginReleaseRef},
    manifest_validation::ValidatedComputePluginManifest,
    trusted_time::ComputePluginTrustedTimeObservation,
};

mod validation;

pub(in crate::node_agent_compute_plugin_host) const CANDIDATE_HEALTH_OBSERVATION_SCHEMA: &str =
    "elon.compute_plugin.candidate_health_observation.v1";
pub(in crate::node_agent_compute_plugin_host) const HASHED_CANDIDATE_HEALTH_OBSERVATION_SCHEMA:
    &str = "elon.compute_plugin.hashed_candidate_health_observation.v1";
const CANDIDATE_HEALTHY: &str = "healthy";
const CANDIDATE_HEALTH_CANONICALIZATION: &str = "RFC8785-JCS";
const CANDIDATE_HEALTH_DIGEST_ALGORITHM: &str = "sha256";
const CANDIDATE_HEALTH_TRANSCRIPT_SCHEMA: &str =
    "elon.compute_plugin.candidate_health_transcript.v1";
const MAX_CANDIDATE_HEALTH_PROBES: i64 = 64;
const MAX_CANDIDATE_HEALTH_REASON_CODES: usize = 16;
const MAX_CANDIDATE_HEALTH_LIFETIME_SECONDS: i64 = 5 * 60;
const MAX_CANDIDATE_HEALTH_TIMEOUT_MS: i64 = 2 * 60 * 1_000;
const MAX_CANDIDATE_HEALTH_INTERVAL_MS: i64 = 5 * 60 * 1_000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(in crate::node_agent_compute_plugin_host) enum CandidateHealthProbeOutcome {
    Success,
    Failure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthProbeObservation {
    pub outcome: CandidateHealthProbeOutcome,
    pub latency_ms: i64,
    pub response_digest: String,
    pub reason_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthProgress {
    pub attempted_probes: i64,
    pub successful_probes: i64,
    pub consecutive_successes: i64,
    pub consecutive_failures: i64,
    pub healthy: bool,
    pub terminal_unhealthy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateHealthObservation {
    pub schema: String,
    pub evaluation_id: String,
    pub installation_id_digest: String,
    pub candidate_token_digest: String,
    pub staging_id: String,
    pub staging_receipt_digest: String,
    pub staging_run_digest: String,
    pub root_identity_digest: String,
    pub extraction_plan_digest: String,
    pub release: ComputePluginReleaseRef,
    pub entrypoint_relative_path: String,
    pub runner_digest: String,
    pub protocol: String,
    pub timeout_ms: i64,
    pub interval_ms: i64,
    pub required_consecutive_successes: i64,
    pub unhealthy_after_failures: i64,
    pub attempted_probes: i64,
    pub successful_probes: i64,
    pub consecutive_successes: i64,
    pub probe_transcript_digest: String,
    pub reason_codes: Vec<String>,
    pub status: String,
    pub observed_at: String,
    pub expires_at: String,
    pub clock_epoch_digest: String,
    pub process_owner_epoch: i64,
    pub authority_state_revision: i64,
    pub inventory_revision: i64,
    pub inventory_digest: String,
    pub authority_epoch: i64,
    pub time_authority_id: String,
    pub time_attestation_digest: String,
    pub time_attestation_sequence: i64,
    pub time_signing_key_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginCandidateHealthObservation {
    pub schema: String,
    pub observation: ComputePluginCandidateHealthObservation,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub observation_digest: String,
}

struct CandidateHealthBinding {
    installation_id_digest: String,
    candidate_token_digest: String,
    staging_id: String,
    staging_receipt_digest: String,
    staging_run_digest: String,
    root_identity_digest: String,
    extraction_plan_digest: String,
    release: ComputePluginReleaseRef,
    entrypoint_relative_path: String,
    runner_digest: String,
    protocol: String,
    timeout_ms: i64,
    interval_ms: i64,
    required_consecutive_successes: i64,
    unhealthy_after_failures: i64,
    clock_epoch_digest: String,
    process_owner_epoch: i64,
    authority_state_revision: i64,
    inventory_revision: i64,
    inventory_digest: String,
    authority_epoch: i64,
    staged_at_ms: i64,
}

/// Process-local evaluation state. It owns staged custody and cannot be serialized or cloned.
#[must_use = "candidate health evaluation must be finalized or returned for cleanup"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthEvaluation<'root> {
    staged: StagedComputePluginCandidateArchive<'root>,
    evaluation_id: String,
    binding: CandidateHealthBinding,
    attempted_probes: i64,
    successful_probes: i64,
    consecutive_successes: i64,
    consecutive_failures: i64,
    healthy: bool,
    terminal_unhealthy: bool,
    reason_codes: BTreeSet<String>,
    transcript_digest: String,
    started_at: Instant,
    last_probe_at: Option<Instant>,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthEvaluationStartFailure<'root> {
    error: Error,
    staged: StagedComputePluginCandidateArchive<'root>,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthFinalizationFailure<'root> {
    error: Error,
    evaluation: CandidateHealthEvaluation<'root>,
}

/// Validated input for a future health Store transaction. This is not a durable receipt or a
/// promotion permit and still owns the original staged file handles.
#[must_use = "validated candidate health must be persisted or returned for cleanup"]
pub(in crate::node_agent_compute_plugin_host) struct ValidatedCandidateHealthPublication<'root> {
    staged: StagedComputePluginCandidateArchive<'root>,
    observation: HashedComputePluginCandidateHealthObservation,
    trusted_time: ComputePluginTrustedTimeObservation,
}

pub(in crate::node_agent_compute_plugin_host) fn begin_candidate_health_evaluation<'root>(
    staged: StagedComputePluginCandidateArchive<'root>,
    manifest: &ValidatedComputePluginManifest,
    installation: &ComputePluginInstallationIdentity,
) -> std::result::Result<
    CandidateHealthEvaluation<'root>,
    CandidateHealthEvaluationStartFailure<'root>,
> {
    validation::begin_evaluation(staged, manifest, installation)
}

pub(in crate::node_agent_compute_plugin_host) fn record_candidate_health_probe(
    evaluation: &mut CandidateHealthEvaluation<'_>,
    observation: CandidateHealthProbeObservation,
) -> Result<CandidateHealthProgress> {
    validation::record_probe(evaluation, observation)
}

pub(in crate::node_agent_compute_plugin_host) fn finalize_candidate_health_evaluation<'root>(
    evaluation: CandidateHealthEvaluation<'root>,
    trusted_time: ComputePluginTrustedTimeObservation,
) -> std::result::Result<
    ValidatedCandidateHealthPublication<'root>,
    CandidateHealthFinalizationFailure<'root>,
> {
    validation::finalize_evaluation(evaluation, trusted_time)
}

impl CandidateHealthEvaluation<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn progress(&self) -> CandidateHealthProgress {
        CandidateHealthProgress {
            attempted_probes: self.attempted_probes,
            successful_probes: self.successful_probes,
            consecutive_successes: self.consecutive_successes,
            consecutive_failures: self.consecutive_failures,
            healthy: self.healthy,
            terminal_unhealthy: self.terminal_unhealthy,
        }
    }
}

impl<'root> CandidateHealthEvaluation<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_staged(
        self,
    ) -> StagedComputePluginCandidateArchive<'root> {
        self.staged
    }
}

impl<'root> CandidateHealthEvaluationStartFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, StagedComputePluginCandidateArchive<'root>) {
        (self.error, self.staged)
    }
}

impl<'root> CandidateHealthFinalizationFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateHealthEvaluation<'root>) {
        (self.error, self.evaluation)
    }
}

impl ValidatedCandidateHealthPublication<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn observation(
        &self,
    ) -> &HashedComputePluginCandidateHealthObservation {
        &self.observation
    }

    pub(in crate::node_agent_compute_plugin_host) fn trusted_time(
        &self,
    ) -> &ComputePluginTrustedTimeObservation {
        &self.trusted_time
    }
}

impl<'root> ValidatedCandidateHealthPublication<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        StagedComputePluginCandidateArchive<'root>,
        HashedComputePluginCandidateHealthObservation,
        ComputePluginTrustedTimeObservation,
    ) {
        (self.staged, self.observation, self.trusted_time)
    }
}

macro_rules! impl_failure {
    ($failure:ident) => {
        impl fmt::Display for $failure<'_> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{:#}", self.error)
            }
        }

        impl fmt::Debug for $failure<'_> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_struct(stringify!($failure))
                    .field("error", &self.error)
                    .finish_non_exhaustive()
            }
        }

        impl StdError for $failure<'_> {}
    };
}

impl_failure!(CandidateHealthEvaluationStartFailure);
impl_failure!(CandidateHealthFinalizationFailure);

impl fmt::Debug for CandidateHealthEvaluation<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateHealthEvaluation")
            .field("evaluation_id", &self.evaluation_id)
            .field("progress", &self.progress())
            .field("transcript_digest", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for ValidatedCandidateHealthPublication<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedCandidateHealthPublication")
            .field("observation_digest", &"<redacted>")
            .field("trusted_time", &"<authenticated>")
            .finish()
    }
}
