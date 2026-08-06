use std::collections::BTreeSet;

use anyhow::{bail, Context, Result};
use serde::Serialize;

use super::{
    CandidateHealthProbeObservation, CandidateHealthProbeOutcome, CandidateHealthProgress,
    CANDIDATE_HEALTH_TRANSCRIPT_SCHEMA, MAX_CANDIDATE_HEALTH_PROBES,
    MAX_CANDIDATE_HEALTH_REASON_CODES,
};
use crate::node_agent_compute_plugin_host::{
    install_plan_admission_validation::is_identifier, manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

#[derive(Serialize)]
struct CandidateHealthTranscriptLink<'a> {
    schema: &'static str,
    evaluation_id: &'a str,
    previous_digest: &'a str,
    sequence: i64,
    outcome: CandidateHealthProbeOutcome,
    latency_ms: i64,
    response_digest: &'a str,
    reason_code: Option<&'a str>,
}

pub(super) struct CandidateHealthProbeState {
    attempted_probes: i64,
    successful_probes: i64,
    consecutive_successes: i64,
    consecutive_failures: i64,
    healthy: bool,
    terminal_unhealthy: bool,
    reason_codes: BTreeSet<String>,
    transcript_digest: String,
}

impl CandidateHealthProbeState {
    pub(super) fn new(transcript_digest: String) -> Self {
        Self {
            attempted_probes: 0,
            successful_probes: 0,
            consecutive_successes: 0,
            consecutive_failures: 0,
            healthy: false,
            terminal_unhealthy: false,
            reason_codes: BTreeSet::new(),
            transcript_digest,
        }
    }

    pub(super) fn progress(&self) -> CandidateHealthProgress {
        CandidateHealthProgress {
            attempted_probes: self.attempted_probes,
            successful_probes: self.successful_probes,
            consecutive_successes: self.consecutive_successes,
            consecutive_failures: self.consecutive_failures,
            healthy: self.healthy,
            terminal_unhealthy: self.terminal_unhealthy,
        }
    }

    pub(super) fn transcript_digest(&self) -> &str {
        &self.transcript_digest
    }

    pub(super) fn reason_codes(&self) -> Vec<String> {
        self.reason_codes.iter().cloned().collect()
    }

    pub(super) fn record(
        &mut self,
        evaluation_id: &str,
        timeout_ms: i64,
        required_consecutive_successes: i64,
        unhealthy_after_failures: i64,
        observation: CandidateHealthProbeObservation,
    ) -> Result<CandidateHealthProgress> {
        if self.healthy || self.terminal_unhealthy {
            bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_EVALUATION_TERMINAL");
        }
        let sequence = self
            .attempted_probes
            .checked_add(1)
            .context("COMPUTE_PLUGIN_CANDIDATE_HEALTH_PROBE_COUNT_OVERFLOW")?;
        self.validate_observation(sequence, timeout_ms, &observation)?;

        let next_transcript_digest = jcs_sha256_hex(&CandidateHealthTranscriptLink {
            schema: CANDIDATE_HEALTH_TRANSCRIPT_SCHEMA,
            evaluation_id,
            previous_digest: &self.transcript_digest,
            sequence,
            outcome: observation.outcome,
            latency_ms: observation.latency_ms,
            response_digest: &observation.response_digest,
            reason_code: observation.reason_code.as_deref(),
        })?;

        self.attempted_probes = sequence;
        self.transcript_digest = next_transcript_digest;
        match observation.outcome {
            CandidateHealthProbeOutcome::Success => {
                self.successful_probes = self
                    .successful_probes
                    .checked_add(1)
                    .context("COMPUTE_PLUGIN_CANDIDATE_HEALTH_SUCCESS_COUNT_OVERFLOW")?;
                self.consecutive_successes = self
                    .consecutive_successes
                    .checked_add(1)
                    .context("COMPUTE_PLUGIN_CANDIDATE_HEALTH_SUCCESS_COUNT_OVERFLOW")?;
                self.consecutive_failures = 0;
                self.healthy = self.consecutive_successes >= required_consecutive_successes;
            }
            CandidateHealthProbeOutcome::Failure => {
                self.consecutive_successes = 0;
                self.consecutive_failures = self
                    .consecutive_failures
                    .checked_add(1)
                    .context("COMPUTE_PLUGIN_CANDIDATE_HEALTH_FAILURE_COUNT_OVERFLOW")?;
                if let Some(reason) = observation.reason_code {
                    self.reason_codes.insert(reason);
                }
                self.terminal_unhealthy = self.consecutive_failures >= unhealthy_after_failures;
            }
        }
        Ok(self.progress())
    }

    fn validate_observation(
        &self,
        sequence: i64,
        timeout_ms: i64,
        observation: &CandidateHealthProbeObservation,
    ) -> Result<()> {
        if sequence > MAX_CANDIDATE_HEALTH_PROBES
            || observation.latency_ms < 0
            || observation.latency_ms > timeout_ms
            || !is_sha256(&observation.response_digest)
        {
            bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_PROBE_INVALID");
        }
        match observation.outcome {
            CandidateHealthProbeOutcome::Success if observation.reason_code.is_some() => {
                bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_SUCCESS_REASON_FORBIDDEN")
            }
            CandidateHealthProbeOutcome::Failure => {
                let reason = observation.reason_code.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_FAILURE_REASON_MISSING")
                })?;
                if !is_identifier(reason)
                    || (!self.reason_codes.contains(reason)
                        && self.reason_codes.len() >= MAX_CANDIDATE_HEALTH_REASON_CODES)
                {
                    bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_FAILURE_REASON_INVALID");
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RESPONSE_DIGEST: &str =
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const INITIAL_DIGEST: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn success() -> CandidateHealthProbeObservation {
        CandidateHealthProbeObservation {
            outcome: CandidateHealthProbeOutcome::Success,
            latency_ms: 25,
            response_digest: RESPONSE_DIGEST.to_string(),
            reason_code: None,
        }
    }

    fn failure(reason: &str) -> CandidateHealthProbeObservation {
        CandidateHealthProbeObservation {
            outcome: CandidateHealthProbeOutcome::Failure,
            latency_ms: 30,
            response_digest: RESPONSE_DIGEST.to_string(),
            reason_code: Some(reason.to_string()),
        }
    }

    #[test]
    fn consecutive_successes_reach_healthy_terminal() {
        let mut state = CandidateHealthProbeState::new(INITIAL_DIGEST.to_string());

        let first = state.record("che_test", 100, 2, 3, success()).unwrap();
        assert_eq!(first.attempted_probes, 1);
        assert_eq!(first.consecutive_successes, 1);
        assert!(!first.healthy);

        let second = state.record("che_test", 100, 2, 3, success()).unwrap();
        assert_eq!(second.successful_probes, 2);
        assert!(second.healthy);
        assert!(state
            .record("che_test", 100, 2, 3, success())
            .unwrap_err()
            .to_string()
            .contains("EVALUATION_TERMINAL"));
    }

    #[test]
    fn failure_resets_success_streak_and_locks_terminal_unhealthy() {
        let mut state = CandidateHealthProbeState::new(INITIAL_DIGEST.to_string());

        state.record("che_test", 100, 3, 2, success()).unwrap();
        let first_failure = state
            .record("che_test", 100, 3, 2, failure("probe_timeout"))
            .unwrap();
        assert_eq!(first_failure.consecutive_successes, 0);
        assert_eq!(first_failure.consecutive_failures, 1);

        let terminal = state
            .record("che_test", 100, 3, 2, failure("sidecar_unavailable"))
            .unwrap();
        assert!(terminal.terminal_unhealthy);
        assert_eq!(
            state.reason_codes(),
            vec![
                "probe_timeout".to_string(),
                "sidecar_unavailable".to_string()
            ]
        );
        assert!(state
            .record("che_test", 100, 3, 2, success())
            .unwrap_err()
            .to_string()
            .contains("EVALUATION_TERMINAL"));
    }

    #[test]
    fn invalid_probe_does_not_mutate_progress_or_transcript() {
        let mut state = CandidateHealthProbeState::new(INITIAL_DIGEST.to_string());
        let before = state.transcript_digest().to_string();

        let error = state
            .record(
                "che_test",
                10,
                2,
                2,
                CandidateHealthProbeObservation {
                    outcome: CandidateHealthProbeOutcome::Failure,
                    latency_ms: 11,
                    response_digest: RESPONSE_DIGEST.to_string(),
                    reason_code: Some("probe_timeout".to_string()),
                },
            )
            .unwrap_err();

        assert!(error.to_string().contains("PROBE_INVALID"));
        assert_eq!(state.progress().attempted_probes, 0);
        assert_eq!(state.transcript_digest(), before);
        assert!(state.reason_codes().is_empty());
    }

    #[test]
    fn transcript_chain_is_deterministic_and_order_sensitive() {
        let mut left = CandidateHealthProbeState::new(INITIAL_DIGEST.to_string());
        let mut right = CandidateHealthProbeState::new(INITIAL_DIGEST.to_string());
        let mut reversed = CandidateHealthProbeState::new(INITIAL_DIGEST.to_string());

        left.record("che_test", 100, 3, 3, success()).unwrap();
        left.record("che_test", 100, 3, 3, failure("probe_timeout"))
            .unwrap();
        right.record("che_test", 100, 3, 3, success()).unwrap();
        right
            .record("che_test", 100, 3, 3, failure("probe_timeout"))
            .unwrap();
        reversed
            .record("che_test", 100, 3, 3, failure("probe_timeout"))
            .unwrap();
        reversed.record("che_test", 100, 3, 3, success()).unwrap();

        assert_eq!(left.transcript_digest(), right.transcript_digest());
        assert_ne!(left.transcript_digest(), reversed.transcript_digest());
    }

    #[test]
    fn reason_code_cardinality_is_bounded_without_partial_mutation() {
        let mut state = CandidateHealthProbeState::new(INITIAL_DIGEST.to_string());
        for index in 0..MAX_CANDIDATE_HEALTH_REASON_CODES {
            state
                .record(
                    "che_test",
                    100,
                    MAX_CANDIDATE_HEALTH_PROBES,
                    MAX_CANDIDATE_HEALTH_PROBES,
                    failure(&format!("reason_{index}")),
                )
                .unwrap();
        }
        let before = state.transcript_digest().to_string();
        let before_progress = state.progress();

        let error = state
            .record(
                "che_test",
                100,
                MAX_CANDIDATE_HEALTH_PROBES,
                MAX_CANDIDATE_HEALTH_PROBES,
                failure("reason_overflow"),
            )
            .unwrap_err();

        assert!(error.to_string().contains("FAILURE_REASON_INVALID"));
        assert_eq!(state.progress(), before_progress);
        assert_eq!(state.transcript_digest(), before);
        assert_eq!(
            state.reason_codes().len(),
            MAX_CANDIDATE_HEALTH_REASON_CODES
        );
    }
}
