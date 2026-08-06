use super::*;
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef, signed_artifact_verification::jcs_sha256_hex,
};

const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

fn valid_observation() -> HashedComputePluginCandidateHealthObservation {
    let observation = ComputePluginCandidateHealthObservation {
        schema: CANDIDATE_HEALTH_OBSERVATION_SCHEMA.to_string(),
        evaluation_id: "che_test".to_string(),
        installation_id_digest: DIGEST_A.to_string(),
        candidate_token_digest: DIGEST_B.to_string(),
        staging_id: "cst_test".to_string(),
        staging_receipt_digest: DIGEST_C.to_string(),
        staging_run_digest: DIGEST_A.to_string(),
        root_identity_digest: DIGEST_B.to_string(),
        extraction_plan_digest: DIGEST_C.to_string(),
        release: ComputePluginReleaseRef {
            plugin_id: "llm_runner".to_string(),
            plugin_version: "1.0.0".to_string(),
            target_id: "windows_x86_64".to_string(),
            manifest_digest: DIGEST_A.to_string(),
            package_digest: DIGEST_B.to_string(),
        },
        entrypoint_relative_path: "bin/runner.exe".to_string(),
        runner_digest: DIGEST_C.to_string(),
        protocol: "stdio_v1".to_string(),
        timeout_ms: 1_000,
        interval_ms: 2_000,
        required_consecutive_successes: 2,
        unhealthy_after_failures: 3,
        attempted_probes: 3,
        successful_probes: 2,
        consecutive_successes: 2,
        probe_transcript_digest: DIGEST_A.to_string(),
        reason_codes: vec!["cold_start".to_string()],
        status: CANDIDATE_HEALTHY.to_string(),
        observed_at: "2026-08-07T00:00:00.000Z".to_string(),
        expires_at: "2026-08-07T00:05:00.000Z".to_string(),
        clock_epoch_digest: DIGEST_B.to_string(),
        process_owner_epoch: 2,
        authority_state_revision: 10,
        inventory_revision: 8,
        inventory_digest: DIGEST_C.to_string(),
        authority_epoch: 7,
        time_authority_id: "primary_time".to_string(),
        time_attestation_digest: DIGEST_A.to_string(),
        time_attestation_sequence: 4,
        time_signing_key_fingerprint: DIGEST_B.to_string(),
    };
    let observation_digest = jcs_sha256_hex(&observation).unwrap();
    HashedComputePluginCandidateHealthObservation {
        schema: HASHED_CANDIDATE_HEALTH_OBSERVATION_SCHEMA.to_string(),
        observation,
        canonicalization: CANDIDATE_HEALTH_CANONICALIZATION.to_string(),
        digest_algorithm: CANDIDATE_HEALTH_DIGEST_ALGORITHM.to_string(),
        observation_digest,
    }
}

#[test]
fn validated_health_observation_round_trips() {
    validate_hashed_candidate_health_observation(&valid_observation()).unwrap();
}

#[test]
fn equivalent_noncanonical_timestamp_is_rejected() {
    let mut hashed = valid_observation();
    hashed.observation.observed_at = "2026-08-07T00:00:00.000+00:00".to_string();
    hashed.observation_digest = jcs_sha256_hex(&hashed.observation).unwrap();

    assert!(validate_hashed_candidate_health_observation(&hashed)
        .unwrap_err()
        .to_string()
        .contains("OBSERVATION_INVALID"));
}

#[test]
fn reordered_reason_codes_are_rejected_even_with_matching_digest() {
    let mut hashed = valid_observation();
    hashed.observation.reason_codes = vec!["probe_timeout".to_string(), "cold_start".to_string()];
    hashed.observation_digest = jcs_sha256_hex(&hashed.observation).unwrap();

    assert!(validate_hashed_candidate_health_observation(&hashed)
        .unwrap_err()
        .to_string()
        .contains("OBSERVATION_INVALID"));
}
