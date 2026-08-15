use base64::{engine::general_purpose::STANDARD, Engine as _};
use sha2::{Digest, Sha256};

use super::*;

const CHECKED_IN_PROFILE: &str = include_str!(
    "../../../../docs/distributed-compute/external-pool-adapter-runtime-compatibility-profile-v1.json"
);
const MODULE_ROOT: &str = include_str!("../external_pool_adapter_runtime_compatibility.rs");
const PROFILE_SOURCE: &str = include_str!("profile.rs");
const VALIDATION_SOURCE: &str = include_str!("validation.rs");
const SESSION_NO_WORK: &str =
    include_str!("../../../external-pool-adapter-session-core/src/no_work.rs");
const BROKER_NO_WORK: &str = include_str!("../external_pool_adapter_broker_tls/no_work.rs");

#[test]
fn v266_checked_in_profile_matches_current_policy_catalogs() {
    let expected = server_runtime_compatibility_profile_catalog().unwrap();
    let canonical = runtime_compatibility_profile_json(&expected).unwrap();
    assert_eq!(
        serde_json::from_str::<ExternalPoolAdapterRuntimeCompatibilityProfileEnvelope>(&canonical)
            .unwrap(),
        expected
    );
    let checked_in: ExternalPoolAdapterRuntimeCompatibilityProfileEnvelope =
        serde_json::from_str(CHECKED_IN_PROFILE).unwrap();
    assert_eq!(checked_in, expected);
    validate_runtime_compatibility_profile_envelope(&checked_in).unwrap();
}

#[test]
fn v266_challenge_and_unsigned_candidate_report_are_exact_but_non_authorizing() {
    let challenge = valid_challenge();
    let report = valid_report(&challenge);
    validate_runtime_compatibility_challenge(&challenge).unwrap();
    validate_runtime_compatibility_candidate_report(&report, &challenge).unwrap();
    assert_eq!(report.report.effects, no_effects());
    assert_eq!(
        report.report.candidate_status,
        RUNTIME_COMPATIBILITY_CANDIDATE_STATUS
    );
}

#[test]
fn v266_rejects_profile_challenge_lineage_and_report_tampering() {
    let mut profile = server_runtime_compatibility_profile_catalog().unwrap();
    profile.profile.elnw.max_request_bytes += 1;
    assert!(validate_runtime_compatibility_profile_envelope(&profile).is_err());

    let challenge = valid_challenge();
    let mut bad_challenge = challenge.clone();
    bad_challenge.challenge.challenge_nonce_digest = "f".repeat(64);
    assert!(validate_runtime_compatibility_challenge(&bad_challenge).is_err());

    let mut report = valid_report(&challenge);
    report.report.no_work.probe_root_sha256 = "e".repeat(64);
    refresh_report_digest(&mut report);
    assert!(validate_runtime_compatibility_candidate_report(&report, &challenge).is_err());

    let mut report = valid_report(&challenge);
    report.report.observations.swap(0, 1);
    refresh_report_digest(&mut report);
    assert!(validate_runtime_compatibility_candidate_report(&report, &challenge).is_err());

    let mut report = valid_report(&challenge);
    report.report.observations[0].policy_violation_count = 1;
    refresh_report_digest(&mut report);
    assert!(validate_runtime_compatibility_candidate_report(&report, &challenge).is_err());

    let mut report = valid_report(&challenge);
    report.report.effects.activation_effect = "activation_ready".into();
    refresh_report_digest(&mut report);
    assert!(validate_runtime_compatibility_candidate_report(&report, &challenge).is_err());
}

#[test]
fn v266_rejects_invalid_challenge_time_identity_and_digest_material() {
    let valid = valid_challenge();

    let mut material = valid.challenge.clone();
    material.adapter_id.clear();
    assert!(build_runtime_compatibility_challenge(material).is_err());

    let mut material = valid.challenge.clone();
    material.capability_set_digest = "A".repeat(64);
    assert!(build_runtime_compatibility_challenge(material).is_err());

    let mut material = valid.challenge.clone();
    material.challenge_nonce_base64 = STANDARD.encode([0_u8; 32]);
    material.challenge_nonce_digest = hex::encode(Sha256::digest([0_u8; 32]));
    assert!(build_runtime_compatibility_challenge(material).is_err());

    let mut material = valid.challenge;
    material.expires_at = "2026-08-15T00:10:00.000000001Z".into();
    assert!(build_runtime_compatibility_challenge(material).is_err());
}

#[test]
fn v266_rejects_report_time_bounds_isolation_and_digest_tampering() {
    let challenge = valid_challenge();

    let mut report = valid_report(&challenge);
    report.report.run_completed_at = "2026-08-15T00:00:32.000000000Z".into();
    refresh_report_digest(&mut report);
    assert!(validate_runtime_compatibility_candidate_report(&report, &challenge).is_err());

    let mut report = valid_report(&challenge);
    report.report.child_network_attempt_count = 1;
    refresh_report_digest(&mut report);
    assert!(validate_runtime_compatibility_candidate_report(&report, &challenge).is_err());

    let mut report = valid_report(&challenge);
    report.report.no_work.request_bytes = 0;
    refresh_report_digest(&mut report);
    assert!(validate_runtime_compatibility_candidate_report(&report, &challenge).is_err());

    let mut report = valid_report(&challenge);
    report.report_digest = "0".repeat(64);
    assert!(validate_runtime_compatibility_candidate_report(&report, &challenge).is_err());
}

#[test]
fn v266_profile_tracks_v265_wire_constants_and_has_no_runtime_capabilities() {
    for required in [
        "const PROBE_MAGIC: &[u8; 4] = b\"ELNW\"",
        "const PROBE_VERSION: u8 = 1",
        "const REQUEST_HEADER_BYTES: usize = 48",
        "const RESPONSE_HEADER_BYTES: usize = 44",
        "const RECEIPT_BYTES: usize = 136",
        "const MAX_REQUEST_BYTES: usize = 16_384",
        "const MAX_RESPONSE_BYTES: usize = 65_536",
        "const MAX_PROBE_TIMEOUT: Duration = Duration::from_millis(15_000)",
    ] {
        assert!(
            SESSION_NO_WORK.contains(required),
            "missing ELNW constant {required}"
        );
    }
    for required in [
        "server_linux_runtime_launch_policy_catalog",
        "server_upstream_transport_target_policy_catalog",
        "server_supervisor_session_policy_catalog",
        "single_bounded_write_exact_length_read_v1",
    ] {
        assert!(
            PROFILE_SOURCE.contains(required),
            "missing profile root {required}"
        );
    }
    assert!(BROKER_NO_WORK.contains("stream.write_all(request).await?"));
    assert!(BROKER_NO_WORK.contains("stream.read_exact(&mut response[..]).await?"));
    for forbidden in [
        "crate::store",
        "rusqlite",
        "tokio::net",
        "std::process",
        "secret_resolver",
        "activate_external_pool",
        "compute_usage",
        "compute_settlement",
        "sui_client",
        "axum",
        "mcp",
    ] {
        let combined = [MODULE_ROOT, PROFILE_SOURCE, VALIDATION_SOURCE].concat();
        assert!(
            !combined.contains(forbidden),
            "V266 owns forbidden runtime capability {forbidden}"
        );
    }
}

#[test]
#[ignore = "maintenance helper for intentional profile revision updates"]
fn dump_v266_profile_json() {
    let profile = server_runtime_compatibility_profile_catalog().unwrap();
    println!("{}", serde_json::to_string_pretty(&profile).unwrap());
}

fn valid_challenge() -> ExternalPoolAdapterRuntimeCompatibilityChallenge {
    let profile = server_runtime_compatibility_profile_catalog().unwrap();
    let nonce = [7_u8; 32];
    build_runtime_compatibility_challenge(
        ExternalPoolAdapterRuntimeCompatibilityChallengeMaterial {
            profile_id: profile.profile.profile_id,
            profile_revision: profile.profile.profile_revision,
            profile_digest: profile.profile_digest,
            adapter_id: "adapter.example".into(),
            release_version: "1.0.0".into(),
            implementation_sha256: "1".repeat(64),
            capability_set_digest: "2".repeat(64),
            runtime_image_digest: "3".repeat(64),
            challenge_nonce_base64: STANDARD.encode(nonce),
            challenge_nonce_digest: hex::encode(Sha256::digest(nonce)),
            issued_at: "2026-08-15T00:00:00.000000000Z".into(),
            expires_at: "2026-08-15T00:10:00.000000000Z".into(),
        },
    )
    .unwrap()
}

fn valid_report(
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallenge,
) -> ExternalPoolAdapterRuntimeCompatibilityCandidateReport {
    let probe_nonce = [9_u8; 32];
    let request = b"public compatibility request";
    let response = b"public no-work response";
    let request_digest: [u8; 32] = Sha256::digest(request).into();
    let response_digest: [u8; 32] = Sha256::digest(response).into();
    let no_work = ExternalPoolAdapterRuntimeCompatibilityNoWorkEvidence {
        probe_nonce_base64: STANDARD.encode(probe_nonce),
        probe_nonce_digest: hex::encode(Sha256::digest(probe_nonce)),
        request_bytes: request.len() as u64,
        response_bytes: response.len() as u64,
        request_sha256: hex::encode(request_digest),
        response_sha256: hex::encode(response_digest),
        probe_root_sha256: runtime_compatibility_elnw_root(
            &probe_nonce,
            request.len() as u32,
            response.len() as u32,
            &request_digest,
            &response_digest,
        ),
    };
    let observations = REQUIRED_RUNTIME_OBSERVATIONS
        .iter()
        .map(|id| ExternalPoolAdapterRuntimeCompatibilityObservation {
            observation_id: (*id).into(),
            observation_revision: 1,
            outcome: "passed".into(),
            evidence_digest: hex::encode(Sha256::digest(id.as_bytes())),
            duration_ms: 1,
            policy_violation_count: 0,
        })
        .collect();
    let material = ExternalPoolAdapterRuntimeCompatibilityCandidateMaterial {
        verifier_report_id: "verifier-report-example".into(),
        challenge_digest: challenge.challenge_digest.clone(),
        profile_digest: challenge.challenge.profile_digest.clone(),
        adapter_id: challenge.challenge.adapter_id.clone(),
        release_version: challenge.challenge.release_version.clone(),
        implementation_sha256: challenge.challenge.implementation_sha256.clone(),
        capability_set_digest: challenge.challenge.capability_set_digest.clone(),
        runtime_image_digest: challenge.challenge.runtime_image_digest.clone(),
        run_started_at: "2026-08-15T00:00:01.000000000Z".into(),
        run_completed_at: "2026-08-15T00:00:02.000000000Z".into(),
        child_network_attempt_count: 0,
        write_outside_ephemeral_count: 0,
        additional_process_attempt_count: 0,
        observations,
        no_work,
        evidence_scope: RUNTIME_COMPATIBILITY_EVIDENCE_SCOPE.into(),
        candidate_status: RUNTIME_COMPATIBILITY_CANDIDATE_STATUS.into(),
        effects: no_effects(),
    };
    ExternalPoolAdapterRuntimeCompatibilityCandidateReport {
        schema: RUNTIME_COMPATIBILITY_REPORT_SCHEMA.into(),
        canonicalization: RUNTIME_COMPATIBILITY_CANONICALIZATION.into(),
        digest_algorithm: RUNTIME_COMPATIBILITY_DIGEST_ALGORITHM.into(),
        report_digest: runtime_compatibility_candidate_report_digest(&material).unwrap(),
        report: material,
    }
}

fn refresh_report_digest(report: &mut ExternalPoolAdapterRuntimeCompatibilityCandidateReport) {
    report.report_digest = runtime_compatibility_candidate_report_digest(&report.report).unwrap();
}
