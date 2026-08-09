use std::{
    fmt,
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::ComputePluginTrustedTimeObservation;
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginInstallationIdentity,
    manifest_validation::is_sha256,
    plugin_manifest::{
        ComputePluginSignature, COMPUTE_PLUGIN_DIGEST_ALGORITHM,
        COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION, COMPUTE_PLUGIN_SIGNATURE_ALGORITHM,
    },
    signed_artifact_verification::{
        jcs_sha256_hex, verify_jcs_ed25519, ComputePluginEd25519PublicKey,
    },
};

const TRUSTED_TIME_CHALLENGE_PAYLOAD_SCHEMA: &str = "elon.compute_plugin.trusted_time_challenge.v1";
const TRUSTED_TIME_CHALLENGE_REQUEST_SCHEMA: &str =
    "elon.compute_plugin.trusted_time_challenge_request.v1";
const TRUSTED_TIME_ATTESTATION_SCHEMA: &str = "elon.compute_plugin.trusted_time_attestation.v1";
const SIGNED_TRUSTED_TIME_ATTESTATION_SCHEMA: &str =
    "elon.compute_plugin.signed_trusted_time_attestation.v1";
const TRUSTED_TIME_SIGNATURE_DOMAIN: &str = "ELON-COMPUTE-PLUGIN-TRUSTED-TIME-V1";
const TRUSTED_TIME_CHALLENGE_LIFETIME: Duration = Duration::from_secs(60);
const TRUSTED_TIME_CLOCK_EPOCH_DOMAIN: &[u8] = b"ELON_COMPUTE_PLUGIN_CLOCK_EPOCH_V1";
const RANDOM_CLOCK_EPOCH_BYTES: usize = 32;
const RANDOM_CHALLENGE_ID_BYTES: usize = 16;
const RANDOM_CHALLENGE_NONCE_BYTES: usize = 32;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginTrustedTimeChallengePayload {
    pub schema: String,
    pub challenge_id: String,
    pub challenge_nonce: String,
    pub installation_id_digest: String,
    pub clock_epoch_digest: String,
    pub time_authority_id: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginTrustedTimeChallengeRequest {
    pub schema: String,
    pub challenge: ComputePluginTrustedTimeChallengePayload,
    pub canonicalization: String,
    pub challenge_digest_algorithm: String,
    pub challenge_digest: String,
}

/// Linear local custody for one request. Only the request is serializable; the monotonic deadline
/// and consumption right cannot cross a process boundary or be recreated from response bytes.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginTrustedTimeChallenge {
    request: ComputePluginTrustedTimeChallengeRequest,
    expires_at: Instant,
}

/// One process-local clock epoch. Its random seed is never exposed, serialized or cloned; only a
/// domain-separated digest is carried by challenges and signed attestations.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginTrustedTimeClockEpoch {
    digest: String,
}

impl fmt::Debug for ComputePluginTrustedTimeClockEpoch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginTrustedTimeClockEpoch")
            .field("digest", &"<redacted>")
            .finish()
    }
}

impl ComputePluginTrustedTimeChallenge {
    pub(in crate::node_agent_compute_plugin_host) fn request(
        &self,
    ) -> &ComputePluginTrustedTimeChallengeRequest {
        &self.request
    }

    fn ensure_live(&self, now: Instant) -> Result<()> {
        if now >= self.expires_at {
            bail!("COMPUTE_PLUGIN_TRUSTED_TIME_CHALLENGE_EXPIRED");
        }
        Ok(())
    }
}

impl fmt::Debug for ComputePluginTrustedTimeChallenge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginTrustedTimeChallenge")
            .field("challenge_id", &self.request.challenge.challenge_id)
            .field("installation_id_digest", &"<redacted>")
            .field("challenge_nonce", &"<redacted>")
            .field("expires_at", &"<monotonic>")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginTrustedTimeAttestation {
    pub schema: String,
    pub attestation_id: String,
    pub attestation_sequence: i64,
    pub challenge_id: String,
    pub challenge_digest: String,
    pub installation_id_digest: String,
    pub time_authority_id: String,
    pub clock_epoch_digest: String,
    pub trusted_now: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginSignedTrustedTimeAttestation {
    pub schema: String,
    pub attestation: ComputePluginTrustedTimeAttestation,
    pub canonicalization: String,
    pub attestation_digest_algorithm: String,
    pub attestation_digest: String,
    pub signature: ComputePluginSignature,
}

/// Implementations resolve only immutable Ed25519 verification keys pinned into a trusted node
/// release for this exact authority. AppData files and environment variables are never fallbacks.
pub(in crate::node_agent_compute_plugin_host) trait ComputePluginTrustedTimeKeyResolver {
    fn resolve_trusted_time_key(
        &self,
        time_authority_id: &str,
        signing_key_id: &str,
    ) -> Result<Option<ComputePluginEd25519PublicKey>>;
}

pub(in crate::node_agent_compute_plugin_host) fn create_trusted_time_clock_epoch(
) -> Result<ComputePluginTrustedTimeClockEpoch> {
    let random = SystemRandom::new();
    let mut seed = [0_u8; RANDOM_CLOCK_EPOCH_BYTES];
    random
        .fill(&mut seed)
        .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_TRUSTED_TIME_CLOCK_EPOCH_RANDOM"))?;
    let mut digest = Sha256::new();
    digest.update(TRUSTED_TIME_CLOCK_EPOCH_DOMAIN);
    digest.update([0]);
    digest.update(seed);
    Ok(ComputePluginTrustedTimeClockEpoch {
        digest: hex::encode(digest.finalize()),
    })
}

pub(in crate::node_agent_compute_plugin_host) fn begin_trusted_time_challenge(
    installation_identity: &ComputePluginInstallationIdentity,
    clock_epoch: &ComputePluginTrustedTimeClockEpoch,
    time_authority_id: &str,
) -> Result<ComputePluginTrustedTimeChallenge> {
    validate_opaque_identifier(
        "COMPUTE_PLUGIN_TRUSTED_TIME_AUTHORITY_ID",
        time_authority_id,
        160,
    )?;
    if !is_sha256(installation_identity.digest()) {
        bail!("COMPUTE_PLUGIN_TRUSTED_TIME_INSTALLATION_IDENTITY_INVALID");
    }
    if !is_sha256(&clock_epoch.digest) {
        bail!("COMPUTE_PLUGIN_TRUSTED_TIME_CLOCK_EPOCH_INVALID");
    }

    let random = SystemRandom::new();
    let mut challenge_id = [0_u8; RANDOM_CHALLENGE_ID_BYTES];
    let mut challenge_nonce = [0_u8; RANDOM_CHALLENGE_NONCE_BYTES];
    random
        .fill(&mut challenge_id)
        .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_TRUSTED_TIME_CHALLENGE_RANDOM"))?;
    random
        .fill(&mut challenge_nonce)
        .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_TRUSTED_TIME_NONCE_RANDOM"))?;

    let challenge = ComputePluginTrustedTimeChallengePayload {
        schema: TRUSTED_TIME_CHALLENGE_PAYLOAD_SCHEMA.to_string(),
        challenge_id: hex::encode(challenge_id),
        challenge_nonce: hex::encode(challenge_nonce),
        installation_id_digest: installation_identity.digest().to_string(),
        clock_epoch_digest: clock_epoch.digest.clone(),
        time_authority_id: time_authority_id.to_string(),
    };
    let challenge_digest = jcs_sha256_hex(&challenge)?;
    let now = Instant::now();
    let expires_at = now
        .checked_add(TRUSTED_TIME_CHALLENGE_LIFETIME)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_TRUSTED_TIME_DEADLINE_OVERFLOW"))?;
    Ok(ComputePluginTrustedTimeChallenge {
        request: ComputePluginTrustedTimeChallengeRequest {
            schema: TRUSTED_TIME_CHALLENGE_REQUEST_SCHEMA.to_string(),
            challenge,
            canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
            challenge_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
            challenge_digest,
        },
        expires_at,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn verify_trusted_time_attestation(
    challenge: ComputePluginTrustedTimeChallenge,
    signed: ComputePluginSignedTrustedTimeAttestation,
    resolver: &dyn ComputePluginTrustedTimeKeyResolver,
) -> Result<ComputePluginTrustedTimeObservation> {
    challenge.ensure_live(Instant::now())?;
    validate_challenge_request(&challenge.request)?;
    validate_attestation_binding(&challenge.request, &signed)?;
    let trusted_now = parse_canonical_trusted_time(&signed.attestation.trusted_now)?;

    let key = resolver
        .resolve_trusted_time_key(
            &signed.attestation.time_authority_id,
            &signed.signature.signing_key_id,
        )?
        .ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_TRUSTED_TIME_KEY_UNTRUSTED: verification key missing")
        })?;
    challenge.ensure_live(Instant::now())?;
    verify_jcs_ed25519(
        &signed.attestation,
        &signed.canonicalization,
        &signed.attestation_digest_algorithm,
        &signed.attestation_digest,
        &signed.signature,
        TRUSTED_TIME_SIGNATURE_DOMAIN,
        &key,
    )?;
    let observed_at = Instant::now();
    challenge.ensure_live(observed_at)?;
    let expires_at = observed_at
        .checked_add(super::TRUSTED_TIME_OBSERVATION_LIFETIME)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_TRUSTED_TIME_DEADLINE_OVERFLOW"))?;

    Ok(
        ComputePluginTrustedTimeObservation::from_verified_attestation(
            trusted_now,
            observed_at,
            expires_at,
            signed.attestation.installation_id_digest,
            signed.attestation.clock_epoch_digest,
            signed.attestation.time_authority_id,
            signed.attestation_digest,
            signed.attestation.attestation_sequence,
            key.fingerprint(),
        ),
    )
}

fn validate_challenge_request(request: &ComputePluginTrustedTimeChallengeRequest) -> Result<()> {
    if request.schema != TRUSTED_TIME_CHALLENGE_REQUEST_SCHEMA
        || request.challenge.schema != TRUSTED_TIME_CHALLENGE_PAYLOAD_SCHEMA
        || request.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || request.challenge_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || !is_lower_hex(
            &request.challenge.challenge_id,
            RANDOM_CHALLENGE_ID_BYTES * 2,
        )
        || !is_lower_hex(
            &request.challenge.challenge_nonce,
            RANDOM_CHALLENGE_NONCE_BYTES * 2,
        )
        || !is_sha256(&request.challenge.installation_id_digest)
        || !is_sha256(&request.challenge.clock_epoch_digest)
    {
        bail!("COMPUTE_PLUGIN_TRUSTED_TIME_CHALLENGE_INVALID");
    }
    validate_opaque_identifier(
        "COMPUTE_PLUGIN_TRUSTED_TIME_AUTHORITY_ID",
        &request.challenge.time_authority_id,
        160,
    )?;
    if jcs_sha256_hex(&request.challenge)? != request.challenge_digest {
        bail!("COMPUTE_PLUGIN_TRUSTED_TIME_CHALLENGE_DIGEST_MISMATCH");
    }
    Ok(())
}

fn validate_attestation_binding(
    request: &ComputePluginTrustedTimeChallengeRequest,
    signed: &ComputePluginSignedTrustedTimeAttestation,
) -> Result<()> {
    let attestation = &signed.attestation;
    if signed.schema != SIGNED_TRUSTED_TIME_ATTESTATION_SCHEMA
        || attestation.schema != TRUSTED_TIME_ATTESTATION_SCHEMA
        || signed.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || signed.attestation_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || signed.signature.algorithm != COMPUTE_PLUGIN_SIGNATURE_ALGORITHM
        || !is_sha256(&signed.attestation_digest)
        || attestation.attestation_sequence <= 0
        || attestation.challenge_id != request.challenge.challenge_id
        || attestation.challenge_digest != request.challenge_digest
        || attestation.installation_id_digest != request.challenge.installation_id_digest
        || attestation.clock_epoch_digest != request.challenge.clock_epoch_digest
        || attestation.time_authority_id != request.challenge.time_authority_id
        || !is_sha256(&attestation.clock_epoch_digest)
    {
        bail!("COMPUTE_PLUGIN_TRUSTED_TIME_ATTESTATION_BINDING_INVALID");
    }
    validate_opaque_identifier(
        "COMPUTE_PLUGIN_TRUSTED_TIME_ATTESTATION_ID",
        &attestation.attestation_id,
        160,
    )?;
    validate_opaque_identifier(
        "COMPUTE_PLUGIN_TRUSTED_TIME_SIGNING_KEY_ID",
        &signed.signature.signing_key_id,
        160,
    )
}

fn parse_canonical_trusted_time(value: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .context("COMPUTE_PLUGIN_TRUSTED_TIME_TIMESTAMP_PARSE")?
        .with_timezone(&Utc);
    if parsed.timestamp_millis() < 0 || parsed.to_rfc3339_opts(SecondsFormat::Millis, true) != value
    {
        bail!("COMPUTE_PLUGIN_TRUSTED_TIME_TIMESTAMP_NON_CANONICAL");
    }
    Ok(parsed)
}

fn validate_opaque_identifier(code: &str, value: &str, maximum_bytes: usize) -> Result<()> {
    if value.is_empty()
        || value.len() > maximum_bytes
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        bail!("{code}: identifier is empty, oversized or non-canonical");
    }
    Ok(())
}

fn is_lower_hex(value: &str, expected_characters: usize) -> bool {
    value.len() == expected_characters
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
