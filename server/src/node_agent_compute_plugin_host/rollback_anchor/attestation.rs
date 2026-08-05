use std::{
    fmt,
    time::{Duration, Instant},
};

use anyhow::{bail, Result};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};

use super::{validate_checkpoint_envelope, validate_opaque_identifier};
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginInstallationIdentity,
    local_authority::HashedComputePluginAuthorityRollbackCheckpoint,
    manifest_validation::is_sha256,
    plugin_manifest::{
        ComputePluginSignature, COMPUTE_PLUGIN_DIGEST_ALGORITHM,
        COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION, COMPUTE_PLUGIN_SIGNATURE_ALGORITHM,
    },
    signed_artifact_verification::{
        jcs_sha256_hex, verify_jcs_ed25519, ComputePluginEd25519PublicKey,
    },
};

const ROLLBACK_ANCHOR_CHALLENGE_PAYLOAD_SCHEMA: &str =
    "elon.compute_plugin.rollback_anchor_challenge.v1";
const ROLLBACK_ANCHOR_CHALLENGE_REQUEST_SCHEMA: &str =
    "elon.compute_plugin.rollback_anchor_challenge_request.v1";
const ROLLBACK_ANCHOR_ATTESTATION_SCHEMA: &str =
    "elon.compute_plugin.rollback_anchor_attestation.v1";
const SIGNED_ROLLBACK_ANCHOR_ATTESTATION_SCHEMA: &str =
    "elon.compute_plugin.signed_rollback_anchor_attestation.v1";
const ROLLBACK_ANCHOR_SIGNATURE_DOMAIN: &str = "ELON-COMPUTE-PLUGIN-ROLLBACK-ANCHOR-V1";
const ROLLBACK_ANCHOR_CHALLENGE_LIFETIME: Duration = Duration::from_secs(60);
const RANDOM_CHALLENGE_ID_BYTES: usize = 16;
const RANDOM_CHALLENGE_NONCE_BYTES: usize = 32;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorChallengePayload {
    pub schema: String,
    pub challenge_id: String,
    pub challenge_nonce: String,
    pub installation_id_digest: String,
    pub anchor_id: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorChallengeRequest {
    pub schema: String,
    pub challenge: ComputePluginRollbackAnchorChallengePayload,
    pub canonicalization: String,
    pub challenge_digest_algorithm: String,
    pub challenge_digest: String,
}

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorChallenge {
    request: ComputePluginRollbackAnchorChallengeRequest,
    expires_at: Instant,
}

impl ComputePluginRollbackAnchorChallenge {
    pub(in crate::node_agent_compute_plugin_host) fn request(
        &self,
    ) -> &ComputePluginRollbackAnchorChallengeRequest {
        &self.request
    }

    pub(super) fn expires_at(&self) -> Instant {
        self.expires_at
    }

    pub(super) fn ensure_live(&self, now: Instant) -> Result<()> {
        if now >= self.expires_at {
            bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_CHALLENGE_EXPIRED");
        }
        Ok(())
    }
}

impl fmt::Debug for ComputePluginRollbackAnchorChallenge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginRollbackAnchorChallenge")
            .field("challenge_id", &self.request.challenge.challenge_id)
            .field("challenge_nonce", &"<redacted>")
            .field("installation_id_digest", &"<redacted>")
            .field("anchor_id", &self.request.challenge.anchor_id)
            .field("expires_at", &"<monotonic>")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorAttestation {
    pub schema: String,
    pub anchor_id: String,
    pub anchor_sequence: i64,
    pub challenge_id: String,
    pub challenge_digest: String,
    pub installation_id_digest: String,
    pub checkpoint: HashedComputePluginAuthorityRollbackCheckpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginSignedRollbackAnchorAttestation {
    pub schema: String,
    pub attestation: ComputePluginRollbackAnchorAttestation,
    pub canonicalization: String,
    pub attestation_digest_algorithm: String,
    pub attestation_digest: String,
    pub signature: ComputePluginSignature,
}

/// Implementations resolve only immutable witness keys pinned into a trusted node release for the
/// exact anchor ID. AppData and environment variables are never implicit root-key fallbacks.
pub(in crate::node_agent_compute_plugin_host) trait ComputePluginRollbackAnchorKeyResolver {
    fn resolve_rollback_anchor_key(
        &self,
        anchor_id: &str,
        signing_key_id: &str,
    ) -> Result<Option<ComputePluginEd25519PublicKey>>;
}

pub(in crate::node_agent_compute_plugin_host) struct VerifiedComputePluginRollbackAnchor {
    attestation: ComputePluginRollbackAnchorAttestation,
    attestation_digest: String,
    signing_key_fingerprint: String,
    verified_at: Instant,
}

impl VerifiedComputePluginRollbackAnchor {
    pub(super) fn attestation(&self) -> &ComputePluginRollbackAnchorAttestation {
        &self.attestation
    }

    pub(super) fn attestation_digest(&self) -> &str {
        &self.attestation_digest
    }

    pub(super) fn signing_key_fingerprint(&self) -> &str {
        &self.signing_key_fingerprint
    }

    pub(super) fn verified_at(&self) -> Instant {
        self.verified_at
    }
}

impl fmt::Debug for VerifiedComputePluginRollbackAnchor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedComputePluginRollbackAnchor")
            .field("anchor_id", &self.attestation.anchor_id)
            .field("anchor_sequence", &self.attestation.anchor_sequence)
            .field("attestation_digest", &"<redacted>")
            .field("signing_key_fingerprint", &"<redacted>")
            .field("verified_at", &"<monotonic>")
            .finish()
    }
}

pub(in crate::node_agent_compute_plugin_host) fn begin_rollback_anchor_challenge(
    installation: &ComputePluginInstallationIdentity,
    anchor_id: &str,
) -> Result<ComputePluginRollbackAnchorChallenge> {
    if !is_sha256(installation.digest()) {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_INSTALLATION_INVALID");
    }
    validate_opaque_identifier("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_ID", anchor_id, 160)?;
    let random = SystemRandom::new();
    let mut challenge_id = [0_u8; RANDOM_CHALLENGE_ID_BYTES];
    let mut challenge_nonce = [0_u8; RANDOM_CHALLENGE_NONCE_BYTES];
    random
        .fill(&mut challenge_id)
        .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_CHALLENGE_RANDOM"))?;
    random
        .fill(&mut challenge_nonce)
        .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_NONCE_RANDOM"))?;

    let challenge = ComputePluginRollbackAnchorChallengePayload {
        schema: ROLLBACK_ANCHOR_CHALLENGE_PAYLOAD_SCHEMA.to_string(),
        challenge_id: hex::encode(challenge_id),
        challenge_nonce: hex::encode(challenge_nonce),
        installation_id_digest: installation.digest().to_string(),
        anchor_id: anchor_id.to_string(),
    };
    let challenge_digest = jcs_sha256_hex(&challenge)?;
    let expires_at = Instant::now()
        .checked_add(ROLLBACK_ANCHOR_CHALLENGE_LIFETIME)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_DEADLINE_OVERFLOW"))?;
    Ok(ComputePluginRollbackAnchorChallenge {
        request: ComputePluginRollbackAnchorChallengeRequest {
            schema: ROLLBACK_ANCHOR_CHALLENGE_REQUEST_SCHEMA.to_string(),
            challenge,
            canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
            challenge_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
            challenge_digest,
        },
        expires_at,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn verify_rollback_anchor_attestation(
    challenge: ComputePluginRollbackAnchorChallenge,
    signed: ComputePluginSignedRollbackAnchorAttestation,
    resolver: &dyn ComputePluginRollbackAnchorKeyResolver,
) -> Result<VerifiedComputePluginRollbackAnchor> {
    challenge.ensure_live(Instant::now())?;
    validate_challenge(&challenge.request)?;
    validate_attestation_binding(&challenge.request, &signed)?;
    let key = resolver
        .resolve_rollback_anchor_key(
            &signed.attestation.anchor_id,
            &signed.signature.signing_key_id,
        )?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_KEY_UNTRUSTED"))?;
    challenge.ensure_live(Instant::now())?;
    verify_jcs_ed25519(
        &signed.attestation,
        &signed.canonicalization,
        &signed.attestation_digest_algorithm,
        &signed.attestation_digest,
        &signed.signature,
        ROLLBACK_ANCHOR_SIGNATURE_DOMAIN,
        &key,
    )?;
    let verified_at = Instant::now();
    challenge.ensure_live(verified_at)?;
    Ok(VerifiedComputePluginRollbackAnchor {
        attestation: signed.attestation,
        attestation_digest: signed.attestation_digest,
        signing_key_fingerprint: key.fingerprint(),
        verified_at,
    })
}

pub(super) fn validate_challenge(
    request: &ComputePluginRollbackAnchorChallengeRequest,
) -> Result<()> {
    if request.schema != ROLLBACK_ANCHOR_CHALLENGE_REQUEST_SCHEMA
        || request.challenge.schema != ROLLBACK_ANCHOR_CHALLENGE_PAYLOAD_SCHEMA
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
        || jcs_sha256_hex(&request.challenge)? != request.challenge_digest
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_CHALLENGE_INVALID");
    }
    validate_opaque_identifier(
        "COMPUTE_PLUGIN_ROLLBACK_ANCHOR_ID",
        &request.challenge.anchor_id,
        160,
    )
}

fn validate_attestation_binding(
    request: &ComputePluginRollbackAnchorChallengeRequest,
    signed: &ComputePluginSignedRollbackAnchorAttestation,
) -> Result<()> {
    let attestation = &signed.attestation;
    if signed.schema != SIGNED_ROLLBACK_ANCHOR_ATTESTATION_SCHEMA
        || attestation.schema != ROLLBACK_ANCHOR_ATTESTATION_SCHEMA
        || signed.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || signed.attestation_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || signed.signature.algorithm != COMPUTE_PLUGIN_SIGNATURE_ALGORITHM
        || !is_sha256(&signed.attestation_digest)
        || attestation.anchor_sequence <= 0
        || attestation.anchor_id != request.challenge.anchor_id
        || attestation.challenge_id != request.challenge.challenge_id
        || attestation.challenge_digest != request.challenge_digest
        || attestation.installation_id_digest != request.challenge.installation_id_digest
        || attestation.checkpoint.checkpoint.installation_id_digest
            != request.challenge.installation_id_digest
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_ATTESTATION_BINDING_INVALID");
    }
    validate_opaque_identifier(
        "COMPUTE_PLUGIN_ROLLBACK_ANCHOR_ID",
        &attestation.anchor_id,
        160,
    )?;
    validate_opaque_identifier(
        "COMPUTE_PLUGIN_ROLLBACK_ANCHOR_SIGNING_KEY_ID",
        &signed.signature.signing_key_id,
        160,
    )?;
    validate_checkpoint_envelope(&attestation.checkpoint)
}

fn is_lower_hex(value: &str, expected_characters: usize) -> bool {
    value.len() == expected_characters
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
