use std::{fmt, time::Instant};

use anyhow::{bail, Result};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};

use super::{
    absence::{
        ComputePluginRollbackAnchorAbsenceEnrollmentParts,
        VerifiedComputePluginRollbackAnchorAbsence,
    },
    validate_checkpoint_envelope, validate_opaque_identifier,
    ComputePluginRollbackAnchorKeyResolver,
};
use crate::node_agent_compute_plugin_host::{
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

const ROLLBACK_ANCHOR_ENROLLMENT_PAYLOAD_SCHEMA: &str =
    "elon.compute_plugin.rollback_anchor_enrollment.v1";
const ROLLBACK_ANCHOR_ENROLLMENT_REQUEST_SCHEMA: &str =
    "elon.compute_plugin.rollback_anchor_enrollment_request.v1";
const ROLLBACK_ANCHOR_ENROLLMENT_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.rollback_anchor_enrollment_receipt.v1";
const SIGNED_ROLLBACK_ANCHOR_ENROLLMENT_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.signed_rollback_anchor_enrollment_receipt.v1";
const ROLLBACK_ANCHOR_ENROLLMENT_SIGNATURE_DOMAIN: &str =
    "ELON-COMPUTE-PLUGIN-ROLLBACK-ANCHOR-ENROLLMENT-V1";
const FIRST_ROLLBACK_ANCHOR_SEQUENCE: i64 = 1;
const RANDOM_ENROLLMENT_ID_BYTES: usize = 16;
const RANDOM_ENROLLMENT_NONCE_BYTES: usize = 32;
const ABSENCE_CHALLENGE_ID_CHARACTERS: usize = 32;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorEnrollmentPayload {
    pub schema: String,
    pub enrollment_id: String,
    pub enrollment_nonce: String,
    pub installation_id_digest: String,
    pub anchor_id: String,
    pub absence_challenge_id: String,
    pub absence_challenge_digest: String,
    pub absence_attestation_digest: String,
    pub initial_checkpoint: HashedComputePluginAuthorityRollbackCheckpoint,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorEnrollmentRequest {
    pub schema: String,
    pub enrollment: ComputePluginRollbackAnchorEnrollmentPayload,
    pub canonicalization: String,
    pub enrollment_digest_algorithm: String,
    pub enrollment_digest: String,
}

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorEnrollmentChallenge
{
    request: ComputePluginRollbackAnchorEnrollmentRequest,
    expires_at: Instant,
}

impl ComputePluginRollbackAnchorEnrollmentChallenge {
    pub(in crate::node_agent_compute_plugin_host) fn request(
        &self,
    ) -> &ComputePluginRollbackAnchorEnrollmentRequest {
        &self.request
    }

    fn ensure_live(&self, now: Instant) -> Result<()> {
        if now >= self.expires_at {
            bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_ENROLLMENT_EXPIRED");
        }
        Ok(())
    }
}

impl fmt::Debug for ComputePluginRollbackAnchorEnrollmentChallenge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginRollbackAnchorEnrollmentChallenge")
            .field("enrollment_id", &self.request.enrollment.enrollment_id)
            .field("enrollment_nonce", &"<redacted>")
            .field("installation_id_digest", &"<redacted>")
            .field("anchor_id", &self.request.enrollment.anchor_id)
            .field("absence_attestation_digest", &"<redacted>")
            .field("initial_checkpoint_digest", &"<redacted>")
            .field("expires_at", &"<monotonic>")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorEnrollmentReceipt {
    pub schema: String,
    pub enrollment_id: String,
    pub enrollment_digest: String,
    pub installation_id_digest: String,
    pub anchor_id: String,
    pub absence_attestation_digest: String,
    pub anchor_sequence: i64,
    pub checkpoint_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginSignedRollbackAnchorEnrollmentReceipt
{
    pub schema: String,
    pub receipt: ComputePluginRollbackAnchorEnrollmentReceipt,
    pub canonicalization: String,
    pub receipt_digest_algorithm: String,
    pub receipt_digest: String,
    pub signature: ComputePluginSignature,
}

pub(in crate::node_agent_compute_plugin_host) struct ConfirmedComputePluginRollbackAnchorEnrollment
{
    anchor_id: String,
    anchor_sequence: i64,
    checkpoint: HashedComputePluginAuthorityRollbackCheckpoint,
    receipt_digest: String,
    signing_key_fingerprint: String,
    verified_at: Instant,
}

impl fmt::Debug for ConfirmedComputePluginRollbackAnchorEnrollment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConfirmedComputePluginRollbackAnchorEnrollment")
            .field("anchor_id", &self.anchor_id)
            .field("anchor_sequence", &self.anchor_sequence)
            .field("checkpoint_digest", &"<redacted>")
            .field("receipt_digest", &"<redacted>")
            .field("signing_key_fingerprint", &"<redacted>")
            .field("verified_at", &"<monotonic>")
            .finish()
    }
}

/// Freshness and content binding are local guarantees only. The witness must authenticate the
/// installation and atomically consume the issued absence challenge before accepting this first
/// anchor CAS.
pub(in crate::node_agent_compute_plugin_host) fn begin_rollback_anchor_enrollment(
    checkpoint: HashedComputePluginAuthorityRollbackCheckpoint,
    absence: VerifiedComputePluginRollbackAnchorAbsence,
) -> Result<ComputePluginRollbackAnchorEnrollmentChallenge> {
    let now = Instant::now();
    let absence = absence.into_enrollment_parts();
    if now >= absence.expires_at {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_ABSENCE_EXPIRED");
    }
    validate_enrollment_inputs(&checkpoint, &absence)?;

    let random = SystemRandom::new();
    let mut enrollment_id = [0_u8; RANDOM_ENROLLMENT_ID_BYTES];
    let mut enrollment_nonce = [0_u8; RANDOM_ENROLLMENT_NONCE_BYTES];
    random
        .fill(&mut enrollment_id)
        .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_ENROLLMENT_ID_RANDOM"))?;
    random
        .fill(&mut enrollment_nonce)
        .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_ENROLLMENT_NONCE_RANDOM"))?;

    let enrollment = ComputePluginRollbackAnchorEnrollmentPayload {
        schema: ROLLBACK_ANCHOR_ENROLLMENT_PAYLOAD_SCHEMA.to_string(),
        enrollment_id: hex::encode(enrollment_id),
        enrollment_nonce: hex::encode(enrollment_nonce),
        installation_id_digest: absence.installation_id_digest,
        anchor_id: absence.anchor_id,
        absence_challenge_id: absence.challenge_id,
        absence_challenge_digest: absence.challenge_digest,
        absence_attestation_digest: absence.absence_attestation_digest,
        initial_checkpoint: checkpoint,
    };
    let enrollment_digest = jcs_sha256_hex(&enrollment)?;
    Ok(ComputePluginRollbackAnchorEnrollmentChallenge {
        request: ComputePluginRollbackAnchorEnrollmentRequest {
            schema: ROLLBACK_ANCHOR_ENROLLMENT_REQUEST_SCHEMA.to_string(),
            enrollment,
            canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
            enrollment_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
            enrollment_digest,
        },
        expires_at: absence.expires_at,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn verify_rollback_anchor_enrollment_receipt(
    challenge: ComputePluginRollbackAnchorEnrollmentChallenge,
    signed: ComputePluginSignedRollbackAnchorEnrollmentReceipt,
    resolver: &dyn ComputePluginRollbackAnchorKeyResolver,
) -> Result<ConfirmedComputePluginRollbackAnchorEnrollment> {
    challenge.ensure_live(Instant::now())?;
    validate_enrollment_request(&challenge.request)?;
    validate_enrollment_receipt_binding(&challenge.request, &signed)?;
    let key = resolver
        .resolve_rollback_anchor_key(&signed.receipt.anchor_id, &signed.signature.signing_key_id)?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_KEY_UNTRUSTED"))?;
    challenge.ensure_live(Instant::now())?;
    verify_enrollment_receipt_signature(&signed, &key)?;
    let verified_at = Instant::now();
    challenge.ensure_live(verified_at)?;

    Ok(ConfirmedComputePluginRollbackAnchorEnrollment {
        anchor_id: signed.receipt.anchor_id,
        anchor_sequence: signed.receipt.anchor_sequence,
        checkpoint: challenge.request.enrollment.initial_checkpoint,
        receipt_digest: signed.receipt_digest,
        signing_key_fingerprint: key.fingerprint(),
        verified_at,
    })
}

fn validate_enrollment_inputs(
    checkpoint: &HashedComputePluginAuthorityRollbackCheckpoint,
    absence: &ComputePluginRollbackAnchorAbsenceEnrollmentParts,
) -> Result<()> {
    validate_checkpoint_envelope(checkpoint)?;
    if checkpoint.checkpoint.installation_id_digest != absence.installation_id_digest
        || !is_sha256(&absence.challenge_digest)
        || !is_sha256(&absence.absence_attestation_digest)
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_ENROLLMENT_INPUT_INVALID");
    }
    validate_opaque_identifier("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_ID", &absence.anchor_id, 160)
}

fn validate_enrollment_request(
    request: &ComputePluginRollbackAnchorEnrollmentRequest,
) -> Result<()> {
    let enrollment = &request.enrollment;
    if request.schema != ROLLBACK_ANCHOR_ENROLLMENT_REQUEST_SCHEMA
        || enrollment.schema != ROLLBACK_ANCHOR_ENROLLMENT_PAYLOAD_SCHEMA
        || request.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || request.enrollment_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || !is_lower_hex(&enrollment.enrollment_id, RANDOM_ENROLLMENT_ID_BYTES * 2)
        || !is_lower_hex(
            &enrollment.enrollment_nonce,
            RANDOM_ENROLLMENT_NONCE_BYTES * 2,
        )
        || !is_sha256(&enrollment.installation_id_digest)
        || !is_lower_hex(
            &enrollment.absence_challenge_id,
            ABSENCE_CHALLENGE_ID_CHARACTERS,
        )
        || !is_sha256(&enrollment.absence_challenge_digest)
        || !is_sha256(&enrollment.absence_attestation_digest)
        || enrollment
            .initial_checkpoint
            .checkpoint
            .installation_id_digest
            != enrollment.installation_id_digest
        || jcs_sha256_hex(enrollment)? != request.enrollment_digest
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_ENROLLMENT_REQUEST_INVALID");
    }
    validate_opaque_identifier(
        "COMPUTE_PLUGIN_ROLLBACK_ANCHOR_ID",
        &enrollment.anchor_id,
        160,
    )?;
    validate_checkpoint_envelope(&enrollment.initial_checkpoint)
}

fn validate_enrollment_receipt_binding(
    request: &ComputePluginRollbackAnchorEnrollmentRequest,
    signed: &ComputePluginSignedRollbackAnchorEnrollmentReceipt,
) -> Result<()> {
    let enrollment = &request.enrollment;
    let receipt = &signed.receipt;
    if signed.schema != SIGNED_ROLLBACK_ANCHOR_ENROLLMENT_RECEIPT_SCHEMA
        || receipt.schema != ROLLBACK_ANCHOR_ENROLLMENT_RECEIPT_SCHEMA
        || signed.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || signed.receipt_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || signed.signature.algorithm != COMPUTE_PLUGIN_SIGNATURE_ALGORITHM
        || !is_sha256(&signed.receipt_digest)
        || receipt.enrollment_id != enrollment.enrollment_id
        || receipt.enrollment_digest != request.enrollment_digest
        || receipt.installation_id_digest != enrollment.installation_id_digest
        || receipt.anchor_id != enrollment.anchor_id
        || receipt.absence_attestation_digest != enrollment.absence_attestation_digest
        || receipt.anchor_sequence != FIRST_ROLLBACK_ANCHOR_SEQUENCE
        || receipt.checkpoint_digest != enrollment.initial_checkpoint.checkpoint_digest
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_ENROLLMENT_RECEIPT_BINDING_INVALID");
    }
    validate_opaque_identifier(
        "COMPUTE_PLUGIN_ROLLBACK_ANCHOR_SIGNING_KEY_ID",
        &signed.signature.signing_key_id,
        160,
    )
}

fn verify_enrollment_receipt_signature(
    signed: &ComputePluginSignedRollbackAnchorEnrollmentReceipt,
    key: &ComputePluginEd25519PublicKey,
) -> Result<()> {
    verify_jcs_ed25519(
        &signed.receipt,
        &signed.canonicalization,
        &signed.receipt_digest_algorithm,
        &signed.receipt_digest,
        &signed.signature,
        ROLLBACK_ANCHOR_ENROLLMENT_SIGNATURE_DOMAIN,
        key,
    )
}

fn is_lower_hex(value: &str, expected_characters: usize) -> bool {
    value.len() == expected_characters
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
