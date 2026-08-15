use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Duration, SecondsFormat};
use sha2::{Digest, Sha256};

use super::*;

pub(crate) fn validate_runtime_compatibility_profile_envelope(
    envelope: &ExternalPoolAdapterRuntimeCompatibilityProfileEnvelope,
) -> Result<()> {
    if envelope.schema != RUNTIME_COMPATIBILITY_PROFILE_ENVELOPE_SCHEMA
        || envelope.canonicalization != RUNTIME_COMPATIBILITY_CANONICALIZATION
        || envelope.digest_algorithm != RUNTIME_COMPATIBILITY_DIGEST_ALGORITHM
    {
        bail!("runtime compatibility profile envelope metadata is unsupported");
    }
    digest(
        &envelope.profile_digest,
        "runtime compatibility profile digest",
    )?;
    validate_runtime_compatibility_profile(&envelope.profile)?;
    if runtime_compatibility_profile_digest(&envelope.profile)? != envelope.profile_digest {
        bail!("runtime compatibility profile digest is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_runtime_compatibility_profile(
    profile: &ExternalPoolAdapterRuntimeCompatibilityProfile,
) -> Result<()> {
    let expected = super::profile::profile_for_validation()?;
    if profile != &expected {
        bail!("runtime compatibility profile does not match current server policy catalogs");
    }
    Ok(())
}

pub(crate) fn validate_runtime_compatibility_challenge(
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallenge,
) -> Result<()> {
    if challenge.schema != RUNTIME_COMPATIBILITY_CHALLENGE_SCHEMA
        || challenge.canonicalization != RUNTIME_COMPATIBILITY_CANONICALIZATION
        || challenge.digest_algorithm != RUNTIME_COMPATIBILITY_DIGEST_ALGORITHM
    {
        bail!("runtime compatibility challenge metadata is unsupported");
    }
    digest(
        &challenge.challenge_digest,
        "runtime compatibility challenge digest",
    )?;
    validate_runtime_compatibility_challenge_material(&challenge.challenge)?;
    if runtime_compatibility_challenge_digest(&challenge.challenge)? != challenge.challenge_digest {
        bail!("runtime compatibility challenge digest is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_runtime_compatibility_challenge_material(
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallengeMaterial,
) -> Result<()> {
    let profile = server_runtime_compatibility_profile_catalog()?;
    if challenge.profile_id != profile.profile.profile_id
        || challenge.profile_revision != profile.profile.profile_revision
        || challenge.profile_digest != profile.profile_digest
    {
        bail!("runtime compatibility challenge profile binding is not current");
    }
    identifier(&challenge.adapter_id, "Adapter ID", 160)?;
    identifier(&challenge.release_version, "release version", 80)?;
    for (value, label) in [
        (&challenge.implementation_sha256, "implementation digest"),
        (&challenge.capability_set_digest, "capability-set digest"),
        (&challenge.runtime_image_digest, "runtime image digest"),
        (&challenge.challenge_nonce_digest, "challenge nonce digest"),
    ] {
        digest(value, label)?;
    }
    let _ = exact_nonce(
        &challenge.challenge_nonce_base64,
        &challenge.challenge_nonce_digest,
        "challenge nonce",
    )?;
    let issued = canonical_nanos(&challenge.issued_at, "challenge issued_at")?;
    let expires = canonical_nanos(&challenge.expires_at, "challenge expires_at")?;
    let validity = expires - issued;
    if validity <= Duration::zero()
        || validity > Duration::minutes(MAX_COMPATIBILITY_CHALLENGE_MINUTES)
    {
        bail!("runtime compatibility challenge validity is invalid");
    }
    Ok(())
}

pub(crate) fn validate_runtime_compatibility_candidate_report(
    report: &ExternalPoolAdapterRuntimeCompatibilityCandidateReport,
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallenge,
) -> Result<()> {
    validate_runtime_compatibility_challenge(challenge)?;
    if report.schema != RUNTIME_COMPATIBILITY_REPORT_SCHEMA
        || report.canonicalization != RUNTIME_COMPATIBILITY_CANONICALIZATION
        || report.digest_algorithm != RUNTIME_COMPATIBILITY_DIGEST_ALGORITHM
    {
        bail!("runtime compatibility candidate report metadata is unsupported");
    }
    digest(&report.report_digest, "candidate report digest")?;
    validate_candidate_material(&report.report, challenge)?;
    if runtime_compatibility_candidate_report_digest(&report.report)? != report.report_digest {
        bail!("runtime compatibility candidate report digest is not canonical");
    }
    Ok(())
}

fn validate_candidate_material(
    report: &ExternalPoolAdapterRuntimeCompatibilityCandidateMaterial,
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallenge,
) -> Result<()> {
    let expected = &challenge.challenge;
    identifier(&report.verifier_report_id, "verifier report ID", 200)?;
    if report.challenge_digest != challenge.challenge_digest
        || report.profile_digest != expected.profile_digest
        || report.adapter_id != expected.adapter_id
        || report.release_version != expected.release_version
        || report.implementation_sha256 != expected.implementation_sha256
        || report.capability_set_digest != expected.capability_set_digest
        || report.runtime_image_digest != expected.runtime_image_digest
    {
        bail!("runtime compatibility candidate report lineage is not exact");
    }
    let started = canonical_nanos(&report.run_started_at, "run_started_at")?;
    let completed = canonical_nanos(&report.run_completed_at, "run_completed_at")?;
    let issued = canonical_nanos(&expected.issued_at, "challenge issued_at")?;
    let expires = canonical_nanos(&expected.expires_at, "challenge expires_at")?;
    if started < issued
        || completed < started
        || completed >= expires
        || completed - started > Duration::seconds(MAX_COMPATIBILITY_RUN_SECONDS)
    {
        bail!("runtime compatibility candidate report time window is invalid");
    }
    if report.child_network_attempt_count != 0
        || report.write_outside_ephemeral_count != 0
        || report.additional_process_attempt_count != 0
    {
        bail!("runtime compatibility candidate report violates isolation policy");
    }
    validate_observations(&report.observations)?;
    validate_no_work(&report.no_work)?;
    if report.evidence_scope != RUNTIME_COMPATIBILITY_EVIDENCE_SCOPE
        || report.candidate_status != RUNTIME_COMPATIBILITY_CANDIDATE_STATUS
        || report.effects != super::profile::no_effects()
    {
        bail!("runtime compatibility candidate report authority boundary is invalid");
    }
    Ok(())
}

fn validate_observations(
    observations: &[ExternalPoolAdapterRuntimeCompatibilityObservation],
) -> Result<()> {
    if observations.len() != REQUIRED_RUNTIME_OBSERVATIONS.len() {
        bail!("runtime compatibility observation inventory is incomplete");
    }
    for (observation, expected_id) in observations.iter().zip(REQUIRED_RUNTIME_OBSERVATIONS) {
        if observation.observation_id != expected_id
            || observation.observation_revision != 1
            || observation.outcome != "passed"
            || observation.duration_ms == 0
            || observation.duration_ms > MAX_COMPATIBILITY_PROBE_TIMEOUT_MS
            || observation.policy_violation_count != 0
        {
            bail!("runtime compatibility observation is invalid");
        }
        digest(&observation.evidence_digest, "observation evidence digest")?;
    }
    Ok(())
}

fn validate_no_work(
    evidence: &ExternalPoolAdapterRuntimeCompatibilityNoWorkEvidence,
) -> Result<()> {
    if evidence.request_bytes == 0
        || evidence.request_bytes > MAX_COMPATIBILITY_REQUEST_BYTES
        || evidence.response_bytes == 0
        || evidence.response_bytes > MAX_COMPATIBILITY_RESPONSE_BYTES
    {
        bail!("runtime compatibility no-work byte bounds are invalid");
    }
    for (value, label) in [
        (&evidence.request_sha256, "request digest"),
        (&evidence.response_sha256, "response digest"),
        (&evidence.probe_root_sha256, "probe root digest"),
        (&evidence.probe_nonce_digest, "probe nonce digest"),
    ] {
        digest(value, label)?;
    }
    let nonce = exact_nonce(
        &evidence.probe_nonce_base64,
        &evidence.probe_nonce_digest,
        "probe nonce",
    )?;
    let request_digest: [u8; 32] = hex::decode(&evidence.request_sha256)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("request digest length is invalid"))?;
    let response_digest: [u8; 32] = hex::decode(&evidence.response_sha256)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("response digest length is invalid"))?;
    let expected_root = runtime_compatibility_elnw_root(
        &nonce,
        evidence.request_bytes as u32,
        evidence.response_bytes as u32,
        &request_digest,
        &response_digest,
    );
    if evidence.probe_root_sha256 != expected_root {
        bail!("runtime compatibility ELNW probe root is invalid");
    }
    Ok(())
}

fn exact_nonce(value: &str, expected_digest: &str, label: &str) -> Result<[u8; 32]> {
    let bytes = STANDARD
        .decode(value)
        .map_err(|_| anyhow::anyhow!("{label} is not canonical base64"))?;
    let nonce: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("{label} must contain exactly 32 bytes"))?;
    if nonce.iter().all(|byte| *byte == 0)
        || STANDARD.encode(nonce) != value
        || hex::encode(Sha256::digest(nonce)) != expected_digest
    {
        bail!("{label} is invalid");
    }
    Ok(nonce)
}

fn identifier(value: &str, label: &str, max_chars: usize) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > max_chars
        || value.chars().any(char::is_control)
    {
        bail!("{label} is invalid");
    }
    Ok(())
}

fn digest(value: &str, label: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("{label} must be a lowercase SHA-256 digest");
    }
    Ok(())
}

fn canonical_nanos(value: &str, label: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| anyhow::anyhow!("{label} is not RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("{label} must use canonical UTC nanoseconds");
    }
    Ok(parsed)
}
