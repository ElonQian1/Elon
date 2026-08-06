use std::{error::Error as StdError, fmt};

use anyhow::{bail, Context, Error, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use super::{
    CandidateHealthEvaluation, StagedComputePluginCandidateArchive,
    CANDIDATE_HEALTH_CANONICALIZATION, CANDIDATE_HEALTH_DIGEST_ALGORITHM,
    MAX_CANDIDATE_HEALTH_INTERVAL_MS, MAX_CANDIDATE_HEALTH_PROBES,
    MAX_CANDIDATE_HEALTH_REASON_CODES, MAX_CANDIDATE_HEALTH_TIMEOUT_MS,
};
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef, install_plan_admission_validation::is_identifier,
    manifest_validation::is_sha256, signed_artifact_verification::jcs_sha256_hex,
    trusted_time::ComputePluginTrustedTimeObservation,
};

pub(super) const CANDIDATE_HEALTH_FAILURE_OBSERVATION_SCHEMA: &str =
    "elon.compute_plugin.candidate_health_failure_observation.v1";
pub(super) const HASHED_CANDIDATE_HEALTH_FAILURE_OBSERVATION_SCHEMA: &str =
    "elon.compute_plugin.hashed_candidate_health_failure_observation.v1";
const CANDIDATE_TERMINAL_UNHEALTHY: &str = "terminal_unhealthy";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateHealthFailureObservation
{
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
    pub consecutive_failures: i64,
    pub probe_transcript_digest: String,
    pub reason_codes: Vec<String>,
    pub status: String,
    pub failed_at: String,
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
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginCandidateHealthFailureObservation
{
    pub schema: String,
    pub observation: ComputePluginCandidateHealthFailureObservation,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub observation_digest: String,
}

#[must_use = "validated candidate health failure must be quarantined or returned for cleanup"]
pub(in crate::node_agent_compute_plugin_host) struct ValidatedCandidateHealthFailurePublication<
    'root,
> {
    staged: StagedComputePluginCandidateArchive<'root>,
    observation: HashedComputePluginCandidateHealthFailureObservation,
    trusted_time: ComputePluginTrustedTimeObservation,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthFailureFinalizationFailure<
    'root,
> {
    error: Error,
    evaluation: CandidateHealthEvaluation<'root>,
}

pub(in crate::node_agent_compute_plugin_host) fn finalize_candidate_health_failure<'root>(
    evaluation: CandidateHealthEvaluation<'root>,
    trusted_time: ComputePluginTrustedTimeObservation,
) -> std::result::Result<
    ValidatedCandidateHealthFailurePublication<'root>,
    CandidateHealthFailureFinalizationFailure<'root>,
> {
    match build_failure_observation(&evaluation, &trusted_time) {
        Ok(observation) => Ok(ValidatedCandidateHealthFailurePublication {
            staged: evaluation.staged,
            observation,
            trusted_time,
        }),
        Err(error) => Err(CandidateHealthFailureFinalizationFailure { error, evaluation }),
    }
}

pub(in crate::node_agent_compute_plugin_host) fn validate_hashed_candidate_health_failure_observation(
    hashed: &HashedComputePluginCandidateHealthFailureObservation,
) -> Result<()> {
    let observation = &hashed.observation;
    let failed_at = DateTime::parse_from_rfc3339(&observation.failed_at)
        .context("COMPUTE_PLUGIN_CANDIDATE_HEALTH_FAILURE_TIME_INVALID")?
        .with_timezone(&Utc);
    let mut normalized_reasons = observation.reason_codes.clone();
    normalized_reasons.sort();
    normalized_reasons.dedup();
    if hashed.schema != HASHED_CANDIDATE_HEALTH_FAILURE_OBSERVATION_SCHEMA
        || hashed.canonicalization != CANDIDATE_HEALTH_CANONICALIZATION
        || hashed.digest_algorithm != CANDIDATE_HEALTH_DIGEST_ALGORITHM
        || observation.schema != CANDIDATE_HEALTH_FAILURE_OBSERVATION_SCHEMA
        || observation.status != CANDIDATE_TERMINAL_UNHEALTHY
        || !is_identifier(&observation.evaluation_id)
        || !is_sha256(&observation.installation_id_digest)
        || !is_sha256(&observation.candidate_token_digest)
        || !is_identifier(&observation.staging_id)
        || !is_sha256(&observation.staging_receipt_digest)
        || !is_sha256(&observation.staging_run_digest)
        || !is_sha256(&observation.root_identity_digest)
        || !is_sha256(&observation.extraction_plan_digest)
        || observation.entrypoint_relative_path.is_empty()
        || !is_sha256(&observation.runner_digest)
        || !is_identifier(&observation.protocol)
        || observation.timeout_ms <= 0
        || observation.timeout_ms > MAX_CANDIDATE_HEALTH_TIMEOUT_MS
        || observation.interval_ms <= 0
        || observation.interval_ms > MAX_CANDIDATE_HEALTH_INTERVAL_MS
        || observation.required_consecutive_successes <= 0
        || observation.required_consecutive_successes > MAX_CANDIDATE_HEALTH_PROBES
        || observation.unhealthy_after_failures <= 0
        || observation.unhealthy_after_failures > MAX_CANDIDATE_HEALTH_PROBES
        || observation.attempted_probes <= 0
        || observation.attempted_probes > MAX_CANDIDATE_HEALTH_PROBES
        || observation.successful_probes < 0
        || observation.successful_probes > observation.attempted_probes
        || observation.consecutive_successes < 0
        || observation.consecutive_successes > observation.successful_probes
        || observation.consecutive_failures < observation.unhealthy_after_failures
        || observation.consecutive_failures > observation.attempted_probes
        || observation.reason_codes.is_empty()
        || observation.reason_codes.len() > MAX_CANDIDATE_HEALTH_REASON_CODES
        || normalized_reasons != observation.reason_codes
        || !observation
            .reason_codes
            .iter()
            .all(|code| is_identifier(code))
        || !is_sha256(&observation.probe_transcript_digest)
        || observation.failed_at != failed_at.to_rfc3339_opts(SecondsFormat::Millis, true)
        || !is_sha256(&observation.clock_epoch_digest)
        || observation.process_owner_epoch <= 0
        || observation.authority_state_revision <= 0
        || observation.inventory_revision <= 0
        || !is_sha256(&observation.inventory_digest)
        || observation.authority_epoch <= 0
        || !is_identifier(&observation.time_authority_id)
        || !is_sha256(&observation.time_attestation_digest)
        || observation.time_attestation_sequence <= 0
        || !is_sha256(&observation.time_signing_key_fingerprint)
        || !is_sha256(&hashed.observation_digest)
        || jcs_sha256_hex(observation)? != hashed.observation_digest
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_FAILURE_OBSERVATION_INVALID");
    }
    Ok(())
}

fn build_failure_observation(
    evaluation: &CandidateHealthEvaluation<'_>,
    trusted_time: &ComputePluginTrustedTimeObservation,
) -> Result<HashedComputePluginCandidateHealthFailureObservation> {
    let last_probe_at = evaluation
        .last_probe_at
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_FAILURE_PROBE_MISSING"))?;
    let progress = evaluation.probes.progress();
    if progress.healthy
        || !progress.terminal_unhealthy
        || progress.consecutive_failures < evaluation.binding.unhealthy_after_failures
        || evaluation.probes.reason_codes().is_empty()
        || trusted_time.installation_id_digest() != evaluation.binding.installation_id_digest
        || trusted_time.clock_epoch_digest() != evaluation.binding.clock_epoch_digest
        || trusted_time.trusted_now().timestamp_millis() <= evaluation.binding.staged_at_ms
        || trusted_time.observed_at() <= last_probe_at
        || last_probe_at < evaluation.started_at
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_FAILURE_NOT_FINALIZABLE");
    }
    evaluation
        .staged
        .archive()
        .snapshot_cancellation_guard()
        .ensure_current()?;

    let binding = &evaluation.binding;
    let observation = ComputePluginCandidateHealthFailureObservation {
        schema: CANDIDATE_HEALTH_FAILURE_OBSERVATION_SCHEMA.to_string(),
        evaluation_id: evaluation.evaluation_id.clone(),
        installation_id_digest: binding.installation_id_digest.clone(),
        candidate_token_digest: binding.candidate_token_digest.clone(),
        staging_id: binding.staging_id.clone(),
        staging_receipt_digest: binding.staging_receipt_digest.clone(),
        staging_run_digest: binding.staging_run_digest.clone(),
        root_identity_digest: binding.root_identity_digest.clone(),
        extraction_plan_digest: binding.extraction_plan_digest.clone(),
        release: binding.release.clone(),
        entrypoint_relative_path: binding.entrypoint_relative_path.clone(),
        runner_digest: binding.runner_digest.clone(),
        protocol: binding.protocol.clone(),
        timeout_ms: binding.timeout_ms,
        interval_ms: binding.interval_ms,
        required_consecutive_successes: binding.required_consecutive_successes,
        unhealthy_after_failures: binding.unhealthy_after_failures,
        attempted_probes: progress.attempted_probes,
        successful_probes: progress.successful_probes,
        consecutive_successes: progress.consecutive_successes,
        consecutive_failures: progress.consecutive_failures,
        probe_transcript_digest: evaluation.probes.transcript_digest().to_string(),
        reason_codes: evaluation.probes.reason_codes(),
        status: CANDIDATE_TERMINAL_UNHEALTHY.to_string(),
        failed_at: trusted_time
            .trusted_now()
            .to_rfc3339_opts(SecondsFormat::Millis, true),
        clock_epoch_digest: trusted_time.clock_epoch_digest().to_string(),
        process_owner_epoch: binding.process_owner_epoch,
        authority_state_revision: binding.authority_state_revision,
        inventory_revision: binding.inventory_revision,
        inventory_digest: binding.inventory_digest.clone(),
        authority_epoch: binding.authority_epoch,
        time_authority_id: trusted_time.time_authority_id().to_string(),
        time_attestation_digest: trusted_time.attestation_digest().to_string(),
        time_attestation_sequence: trusted_time.attestation_sequence(),
        time_signing_key_fingerprint: trusted_time.signing_key_fingerprint().to_string(),
    };
    let hashed = HashedComputePluginCandidateHealthFailureObservation {
        schema: HASHED_CANDIDATE_HEALTH_FAILURE_OBSERVATION_SCHEMA.to_string(),
        observation_digest: jcs_sha256_hex(&observation)?,
        observation,
        canonicalization: CANDIDATE_HEALTH_CANONICALIZATION.to_string(),
        digest_algorithm: CANDIDATE_HEALTH_DIGEST_ALGORITHM.to_string(),
    };
    validate_hashed_candidate_health_failure_observation(&hashed)?;
    Ok(hashed)
}

impl ValidatedCandidateHealthFailurePublication<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn staged(
        &self,
    ) -> &StagedComputePluginCandidateArchive<'_> {
        &self.staged
    }

    pub(in crate::node_agent_compute_plugin_host) fn observation(
        &self,
    ) -> &HashedComputePluginCandidateHealthFailureObservation {
        &self.observation
    }

    pub(in crate::node_agent_compute_plugin_host) fn trusted_time(
        &self,
    ) -> &ComputePluginTrustedTimeObservation {
        &self.trusted_time
    }
}

impl<'root> ValidatedCandidateHealthFailurePublication<'root> {
    pub(super) fn staged_mut(&mut self) -> &mut StagedComputePluginCandidateArchive<'root> {
        &mut self.staged
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        StagedComputePluginCandidateArchive<'root>,
        HashedComputePluginCandidateHealthFailureObservation,
        ComputePluginTrustedTimeObservation,
    ) {
        (self.staged, self.observation, self.trusted_time)
    }
}

impl<'root> CandidateHealthFailureFinalizationFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateHealthEvaluation<'root>) {
        (self.error, self.evaluation)
    }
}

impl fmt::Display for CandidateHealthFailureFinalizationFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl fmt::Debug for CandidateHealthFailureFinalizationFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateHealthFailureFinalizationFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl StdError for CandidateHealthFailureFinalizationFailure<'_> {}

impl fmt::Debug for ValidatedCandidateHealthFailurePublication<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedCandidateHealthFailurePublication")
            .field("observation_digest", &"<redacted>")
            .field("trusted_time", &"<authenticated>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_failure() -> HashedComputePluginCandidateHealthFailureObservation {
        let observation = ComputePluginCandidateHealthFailureObservation {
            schema: CANDIDATE_HEALTH_FAILURE_OBSERVATION_SCHEMA.to_string(),
            evaluation_id: "che_test".to_string(),
            installation_id_digest: "a".repeat(64),
            candidate_token_digest: "b".repeat(64),
            staging_id: "staging_test".to_string(),
            staging_receipt_digest: "c".repeat(64),
            staging_run_digest: "d".repeat(64),
            root_identity_digest: "e".repeat(64),
            extraction_plan_digest: "f".repeat(64),
            release: ComputePluginReleaseRef {
                plugin_id: "plugin_test".to_string(),
                plugin_version: "1.0.0".to_string(),
                target_id: "windows_x86_64".to_string(),
                manifest_digest: "1".repeat(64),
                package_digest: "2".repeat(64),
            },
            entrypoint_relative_path: "bin/plugin.exe".to_string(),
            runner_digest: "3".repeat(64),
            protocol: "stdio".to_string(),
            timeout_ms: 1_000,
            interval_ms: 1_000,
            required_consecutive_successes: 2,
            unhealthy_after_failures: 2,
            attempted_probes: 3,
            successful_probes: 1,
            consecutive_successes: 0,
            consecutive_failures: 2,
            probe_transcript_digest: "4".repeat(64),
            reason_codes: vec!["probe_timeout".to_string()],
            status: CANDIDATE_TERMINAL_UNHEALTHY.to_string(),
            failed_at: "2026-08-07T00:00:00.000Z".to_string(),
            clock_epoch_digest: "5".repeat(64),
            process_owner_epoch: 2,
            authority_state_revision: 3,
            inventory_revision: 4,
            inventory_digest: "6".repeat(64),
            authority_epoch: 5,
            time_authority_id: "trusted_time".to_string(),
            time_attestation_digest: "7".repeat(64),
            time_attestation_sequence: 6,
            time_signing_key_fingerprint: "8".repeat(64),
        };
        HashedComputePluginCandidateHealthFailureObservation {
            schema: HASHED_CANDIDATE_HEALTH_FAILURE_OBSERVATION_SCHEMA.to_string(),
            observation_digest: jcs_sha256_hex(&observation).unwrap(),
            observation,
            canonicalization: CANDIDATE_HEALTH_CANONICALIZATION.to_string(),
            digest_algorithm: CANDIDATE_HEALTH_DIGEST_ALGORITHM.to_string(),
        }
    }

    #[test]
    fn terminal_failure_observation_is_canonical() {
        validate_hashed_candidate_health_failure_observation(&valid_failure()).unwrap();
    }

    #[test]
    fn failure_observation_requires_threshold_and_reason() {
        let mut failure = valid_failure();
        failure.observation.consecutive_failures = 1;
        failure.observation.reason_codes.clear();
        failure.observation_digest = jcs_sha256_hex(&failure.observation).unwrap();
        assert!(validate_hashed_candidate_health_failure_observation(&failure).is_err());
    }

    #[test]
    fn failure_observation_rejects_digest_tampering() {
        let mut failure = valid_failure();
        failure.observation.status = "healthy".to_string();
        assert!(validate_hashed_candidate_health_failure_observation(&failure).is_err());
    }
}
