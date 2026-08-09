use std::{
    fmt,
    time::{Duration, Instant},
};

use anyhow::{bail, Result};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};

use super::{validate_opaque_identifier, ComputePluginRollbackAnchorKeyResolver};
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginInstallationIdentity,
    install_plan_admission_validation::is_identifier,
    local_authority_schema::COMPUTE_PLUGIN_LOCAL_AUTHORITY_SCHEMA_VERSION,
    manifest_validation::is_sha256,
    plugin_manifest::{
        ComputePluginSignature, COMPUTE_PLUGIN_DIGEST_ALGORITHM,
        COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION, COMPUTE_PLUGIN_SIGNATURE_ALGORITHM,
    },
    signed_artifact_verification::{jcs_sha256_hex, verify_jcs_ed25519},
};

pub(in crate::node_agent_compute_plugin_host) const COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_V2_SCHEMA: &str =
    "elon.compute_plugin.authority_rollback_checkpoint.v2";
pub(in crate::node_agent_compute_plugin_host) const HASHED_COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_V2_SCHEMA: &str =
    "elon.compute_plugin.hashed_authority_rollback_checkpoint.v2";
const ROLLBACK_ANCHOR_CHALLENGE_PAYLOAD_V2_SCHEMA: &str =
    "elon.compute_plugin.rollback_anchor_challenge.v2";
const ROLLBACK_ANCHOR_CHALLENGE_REQUEST_V2_SCHEMA: &str =
    "elon.compute_plugin.rollback_anchor_challenge_request.v2";
const ROLLBACK_ANCHOR_ATTESTATION_V2_SCHEMA: &str =
    "elon.compute_plugin.rollback_anchor_attestation.v2";
const SIGNED_ROLLBACK_ANCHOR_ATTESTATION_V2_SCHEMA: &str =
    "elon.compute_plugin.signed_rollback_anchor_attestation.v2";
const ROLLBACK_ANCHOR_V2_SIGNATURE_DOMAIN: &str = "ELON-COMPUTE-PLUGIN-ROLLBACK-ANCHOR-V2";
pub(super) const ROLLBACK_ANCHOR_STARTUP_WITNESS_V2_SCHEMA: &str =
    "elon.compute_plugin.rollback_anchor_startup_witness.v2";
pub(super) const HASHED_ROLLBACK_ANCHOR_STARTUP_WITNESS_V2_SCHEMA: &str =
    "elon.compute_plugin.hashed_rollback_anchor_startup_witness.v2";
const ROLLBACK_ANCHOR_CHALLENGE_LIFETIME: Duration = Duration::from_secs(60);
const RANDOM_CHALLENGE_ID_BYTES: usize = 16;
const RANDOM_CHALLENGE_NONCE_BYTES: usize = 32;

/// Shared by the future local checkpoint producer and the V2 anchor protocol. V1 remains a
/// separate wire type and cannot be upgraded by adding fields to its canonical payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginAuthorityRollbackCheckpointV2 {
    pub schema: String,
    pub authority_schema_version: i64,
    pub installation_id_digest: String,
    pub state_revision: i64,
    pub inventory_revision: i64,
    pub inventory_digest: String,
    pub desired_policy_revision: i64,
    pub sharing_enabled: bool,
    pub sharing_authorization_ref_digest: Option<String>,
    pub sharing_authorization_revision: Option<i64>,
    pub sharing_authorization_digest: Option<String>,
    pub node_profile_digest: String,
    pub manifest_catalog_revision: i64,
    pub manifest_catalog_digest: String,
    /// Digest of the pre-existing catalog binding receipt. The receipt never includes this
    /// checkpoint digest, so checkpoint -> receipt remains an acyclic evidence edge.
    pub manifest_catalog_binding_receipt_digest: String,
    pub target_id: String,
    pub host_api_protocol_id: String,
    pub host_api_revision: i64,
    pub active_bundle_revision: Option<i64>,
    pub publisher_keyring_revision: Option<i64>,
    pub publisher_keyring_digest: Option<String>,
    pub control_keyring_revision: Option<i64>,
    pub control_keyring_digest: Option<String>,
    pub authority_epoch: i64,
    pub process_owner_epoch: i64,
    pub trusted_time_high_water_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginAuthorityRollbackCheckpointV2
{
    pub schema: String,
    pub checkpoint: ComputePluginAuthorityRollbackCheckpointV2,
    pub canonicalization: String,
    pub checkpoint_digest_algorithm: String,
    pub checkpoint_digest: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorChallengePayloadV2 {
    pub schema: String,
    pub challenge_id: String,
    pub challenge_nonce: String,
    pub installation_id_digest: String,
    pub anchor_id: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorChallengeRequestV2 {
    pub schema: String,
    pub challenge: ComputePluginRollbackAnchorChallengePayloadV2,
    pub canonicalization: String,
    pub challenge_digest_algorithm: String,
    pub challenge_digest: String,
}

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorChallengeV2 {
    request: ComputePluginRollbackAnchorChallengeRequestV2,
    expires_at: Instant,
}

impl ComputePluginRollbackAnchorChallengeV2 {
    pub(in crate::node_agent_compute_plugin_host) fn request(
        &self,
    ) -> &ComputePluginRollbackAnchorChallengeRequestV2 {
        &self.request
    }

    fn ensure_live(&self, now: Instant) -> Result<()> {
        if now >= self.expires_at {
            bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_CHALLENGE_EXPIRED");
        }
        Ok(())
    }
}

impl fmt::Debug for ComputePluginRollbackAnchorChallengeV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginRollbackAnchorChallengeV2")
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
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorAttestationV2 {
    pub schema: String,
    pub anchor_id: String,
    pub anchor_sequence: i64,
    pub challenge_id: String,
    pub challenge_digest: String,
    pub installation_id_digest: String,
    pub checkpoint: HashedComputePluginAuthorityRollbackCheckpointV2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginSignedRollbackAnchorAttestationV2
{
    pub schema: String,
    pub attestation: ComputePluginRollbackAnchorAttestationV2,
    pub canonicalization: String,
    pub attestation_digest_algorithm: String,
    pub attestation_digest: String,
    pub signature: ComputePluginSignature,
}

pub(in crate::node_agent_compute_plugin_host) struct VerifiedComputePluginRollbackAnchorV2 {
    attestation: ComputePluginRollbackAnchorAttestationV2,
    attestation_digest: String,
    signing_key_fingerprint: String,
    verified_at: Instant,
}

impl VerifiedComputePluginRollbackAnchorV2 {
    pub(super) fn attestation(&self) -> &ComputePluginRollbackAnchorAttestationV2 {
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

impl fmt::Debug for VerifiedComputePluginRollbackAnchorV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedComputePluginRollbackAnchorV2")
            .field("anchor_id", &self.attestation.anchor_id)
            .field("anchor_sequence", &self.attestation.anchor_sequence)
            .field("attestation_digest", &"<redacted>")
            .field("signing_key_fingerprint", &"<redacted>")
            .field("verified_at", &"<monotonic>")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct ComputePluginRollbackAnchorStartupWitnessV2 {
    pub(super) schema: String,
    pub(super) anchor_id: String,
    pub(super) anchor_sequence: i64,
    pub(super) checkpoint_digest: String,
    pub(super) attestation_digest: String,
    pub(super) signing_key_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct HashedComputePluginRollbackAnchorStartupWitnessV2 {
    pub(super) schema: String,
    pub(super) witness: ComputePluginRollbackAnchorStartupWitnessV2,
    pub(super) canonicalization: String,
    pub(super) witness_digest_algorithm: String,
    pub(super) witness_digest: String,
}

pub(in crate::node_agent_compute_plugin_host) fn begin_rollback_anchor_challenge_v2(
    installation: &ComputePluginInstallationIdentity,
    anchor_id: &str,
) -> Result<ComputePluginRollbackAnchorChallengeV2> {
    if !is_sha256(installation.digest()) {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_INSTALLATION_INVALID");
    }
    validate_opaque_identifier("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_ID", anchor_id, 160)?;
    let random = SystemRandom::new();
    let mut challenge_id = [0_u8; RANDOM_CHALLENGE_ID_BYTES];
    let mut challenge_nonce = [0_u8; RANDOM_CHALLENGE_NONCE_BYTES];
    random
        .fill(&mut challenge_id)
        .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_CHALLENGE_RANDOM"))?;
    random
        .fill(&mut challenge_nonce)
        .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_NONCE_RANDOM"))?;
    let challenge = ComputePluginRollbackAnchorChallengePayloadV2 {
        schema: ROLLBACK_ANCHOR_CHALLENGE_PAYLOAD_V2_SCHEMA.to_string(),
        challenge_id: hex::encode(challenge_id),
        challenge_nonce: hex::encode(challenge_nonce),
        installation_id_digest: installation.digest().to_string(),
        anchor_id: anchor_id.to_string(),
    };
    let challenge_digest = jcs_sha256_hex(&challenge)?;
    let expires_at = Instant::now()
        .checked_add(ROLLBACK_ANCHOR_CHALLENGE_LIFETIME)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_DEADLINE_OVERFLOW"))?;
    Ok(ComputePluginRollbackAnchorChallengeV2 {
        request: ComputePluginRollbackAnchorChallengeRequestV2 {
            schema: ROLLBACK_ANCHOR_CHALLENGE_REQUEST_V2_SCHEMA.to_string(),
            challenge,
            canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
            challenge_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
            challenge_digest,
        },
        expires_at,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn verify_rollback_anchor_attestation_v2(
    challenge: ComputePluginRollbackAnchorChallengeV2,
    signed: ComputePluginSignedRollbackAnchorAttestationV2,
    resolver: &dyn ComputePluginRollbackAnchorKeyResolver,
) -> Result<VerifiedComputePluginRollbackAnchorV2> {
    challenge.ensure_live(Instant::now())?;
    validate_challenge_v2(challenge.request())?;
    validate_attestation_binding_v2(challenge.request(), &signed)?;
    let key = resolver
        .resolve_rollback_anchor_key(
            &signed.attestation.anchor_id,
            &signed.signature.signing_key_id,
        )?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_KEY_UNTRUSTED"))?;
    challenge.ensure_live(Instant::now())?;
    verify_jcs_ed25519(
        &signed.attestation,
        &signed.canonicalization,
        &signed.attestation_digest_algorithm,
        &signed.attestation_digest,
        &signed.signature,
        ROLLBACK_ANCHOR_V2_SIGNATURE_DOMAIN,
        &key,
    )?;
    let verified_at = Instant::now();
    challenge.ensure_live(verified_at)?;
    Ok(VerifiedComputePluginRollbackAnchorV2 {
        attestation: signed.attestation,
        attestation_digest: signed.attestation_digest,
        signing_key_fingerprint: key.fingerprint(),
        verified_at,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn validate_checkpoint_envelope_v2(
    envelope: &HashedComputePluginAuthorityRollbackCheckpointV2,
) -> Result<()> {
    let checkpoint = &envelope.checkpoint;
    if envelope.schema != HASHED_COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_V2_SCHEMA
        || envelope.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || envelope.checkpoint_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || !is_sha256(&envelope.checkpoint_digest)
        || checkpoint.schema != COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_V2_SCHEMA
        || checkpoint.authority_schema_version != COMPUTE_PLUGIN_LOCAL_AUTHORITY_SCHEMA_VERSION
        || !is_sha256(&checkpoint.installation_id_digest)
        || checkpoint.state_revision < 0
        || checkpoint.inventory_revision < 0
        || !is_sha256(&checkpoint.inventory_digest)
        || checkpoint.desired_policy_revision < 0
        || !is_sha256(&checkpoint.node_profile_digest)
        || checkpoint.manifest_catalog_revision <= 0
        || !is_sha256(&checkpoint.manifest_catalog_digest)
        || !is_sha256(&checkpoint.manifest_catalog_binding_receipt_digest)
        || !is_identifier(&checkpoint.target_id)
        || !is_identifier(&checkpoint.host_api_protocol_id)
        || !(1..=i64::from(u32::MAX)).contains(&checkpoint.host_api_revision)
        || checkpoint.authority_epoch <= 0
        || checkpoint.process_owner_epoch <= 0
        || checkpoint.trusted_time_high_water_ms <= 0
        || checkpoint.updated_at_ms != checkpoint.trusted_time_high_water_ms
        || !sharing_binding_is_valid_v2(checkpoint)
        || !keyring_binding_is_valid_v2(checkpoint)
        || jcs_sha256_hex(checkpoint)? != envelope.checkpoint_digest
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_CHECKPOINT_INVALID");
    }
    Ok(())
}

pub(super) fn build_startup_witness_v2(
    anchor_id: String,
    anchor_sequence: i64,
    checkpoint_digest: String,
    attestation_digest: String,
    signing_key_fingerprint: String,
) -> Result<HashedComputePluginRollbackAnchorStartupWitnessV2> {
    validate_opaque_identifier("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_ID", &anchor_id, 160)?;
    if anchor_sequence <= 0
        || !is_sha256(&checkpoint_digest)
        || !is_sha256(&attestation_digest)
        || !is_sha256(&signing_key_fingerprint)
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_WITNESS_INVALID");
    }
    let witness = ComputePluginRollbackAnchorStartupWitnessV2 {
        schema: ROLLBACK_ANCHOR_STARTUP_WITNESS_V2_SCHEMA.to_string(),
        anchor_id,
        anchor_sequence,
        checkpoint_digest,
        attestation_digest,
        signing_key_fingerprint,
    };
    let witness_digest = jcs_sha256_hex(&witness)?;
    Ok(HashedComputePluginRollbackAnchorStartupWitnessV2 {
        schema: HASHED_ROLLBACK_ANCHOR_STARTUP_WITNESS_V2_SCHEMA.to_string(),
        witness,
        canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
        witness_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
        witness_digest,
    })
}

fn validate_challenge_v2(request: &ComputePluginRollbackAnchorChallengeRequestV2) -> Result<()> {
    if request.schema != ROLLBACK_ANCHOR_CHALLENGE_REQUEST_V2_SCHEMA
        || request.challenge.schema != ROLLBACK_ANCHOR_CHALLENGE_PAYLOAD_V2_SCHEMA
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
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_CHALLENGE_INVALID");
    }
    validate_opaque_identifier(
        "COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_ID",
        &request.challenge.anchor_id,
        160,
    )
}

fn validate_attestation_binding_v2(
    request: &ComputePluginRollbackAnchorChallengeRequestV2,
    signed: &ComputePluginSignedRollbackAnchorAttestationV2,
) -> Result<()> {
    let attestation = &signed.attestation;
    if signed.schema != SIGNED_ROLLBACK_ANCHOR_ATTESTATION_V2_SCHEMA
        || attestation.schema != ROLLBACK_ANCHOR_ATTESTATION_V2_SCHEMA
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
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_ATTESTATION_BINDING_INVALID");
    }
    validate_opaque_identifier(
        "COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_ID",
        &attestation.anchor_id,
        160,
    )?;
    validate_opaque_identifier(
        "COMPUTE_PLUGIN_ROLLBACK_ANCHOR_V2_SIGNING_KEY_ID",
        &signed.signature.signing_key_id,
        160,
    )?;
    validate_checkpoint_envelope_v2(&attestation.checkpoint)
}

fn sharing_binding_is_valid_v2(checkpoint: &ComputePluginAuthorityRollbackCheckpointV2) -> bool {
    match (
        checkpoint.sharing_authorization_ref_digest.as_deref(),
        checkpoint.sharing_authorization_revision,
        checkpoint.sharing_authorization_digest.as_deref(),
    ) {
        (None, None, None) => !checkpoint.sharing_enabled,
        (Some(reference_digest), Some(revision), Some(authorization_digest)) => {
            checkpoint.sharing_enabled
                && is_sha256(reference_digest)
                && revision > 0
                && revision == checkpoint.desired_policy_revision
                && is_sha256(authorization_digest)
        }
        _ => false,
    }
}

fn keyring_binding_is_valid_v2(checkpoint: &ComputePluginAuthorityRollbackCheckpointV2) -> bool {
    match (
        checkpoint.active_bundle_revision,
        checkpoint.publisher_keyring_revision,
        checkpoint.publisher_keyring_digest.as_deref(),
        checkpoint.control_keyring_revision,
        checkpoint.control_keyring_digest.as_deref(),
    ) {
        (
            Some(bundle),
            Some(publisher_revision),
            Some(publisher_digest),
            Some(control_revision),
            Some(control_digest),
        ) => {
            bundle > 0
                && publisher_revision > 0
                && control_revision > 0
                && is_sha256(publisher_digest)
                && is_sha256(control_digest)
        }
        _ => false,
    }
}

fn is_lower_hex(value: &str, expected_characters: usize) -> bool {
    value.len() == expected_characters
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
