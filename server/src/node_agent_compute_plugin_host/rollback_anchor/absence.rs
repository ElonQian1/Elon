use std::{fmt, time::Instant};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use super::{
    attestation::{
        validate_challenge, ComputePluginRollbackAnchorChallenge,
        ComputePluginRollbackAnchorKeyResolver,
    },
    validate_opaque_identifier,
};
use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256,
    plugin_manifest::{
        ComputePluginSignature, COMPUTE_PLUGIN_DIGEST_ALGORITHM,
        COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION, COMPUTE_PLUGIN_SIGNATURE_ALGORITHM,
    },
    signed_artifact_verification::verify_jcs_ed25519,
};

const ROLLBACK_ANCHOR_ABSENCE_ATTESTATION_SCHEMA: &str =
    "elon.compute_plugin.rollback_anchor_absence_attestation.v1";
const SIGNED_ROLLBACK_ANCHOR_ABSENCE_ATTESTATION_SCHEMA: &str =
    "elon.compute_plugin.signed_rollback_anchor_absence_attestation.v1";
const ROLLBACK_ANCHOR_ABSENCE_SIGNATURE_DOMAIN: &str =
    "ELON-COMPUTE-PLUGIN-ROLLBACK-ANCHOR-ABSENCE-V1";
const ROLLBACK_ANCHOR_ABSENT_STATE: &str = "absent";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorAbsenceAttestation {
    pub schema: String,
    pub anchor_id: String,
    pub anchor_state: String,
    pub challenge_id: String,
    pub challenge_digest: String,
    pub installation_id_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginSignedRollbackAnchorAbsenceAttestation
{
    pub schema: String,
    pub attestation: ComputePluginRollbackAnchorAbsenceAttestation,
    pub canonicalization: String,
    pub attestation_digest_algorithm: String,
    pub attestation_digest: String,
    pub signature: ComputePluginSignature,
}

pub(in crate::node_agent_compute_plugin_host) struct VerifiedComputePluginRollbackAnchorAbsence {
    signed: ComputePluginSignedRollbackAnchorAbsenceAttestation,
    signing_key_fingerprint: String,
    expires_at: Instant,
    verified_at: Instant,
}

pub(super) struct ComputePluginRollbackAnchorAbsenceEnrollmentParts {
    pub(super) anchor_id: String,
    pub(super) challenge_id: String,
    pub(super) challenge_digest: String,
    pub(super) installation_id_digest: String,
    pub(super) absence_attestation_digest: String,
    pub(super) expires_at: Instant,
}

impl VerifiedComputePluginRollbackAnchorAbsence {
    pub(super) fn into_enrollment_parts(self) -> ComputePluginRollbackAnchorAbsenceEnrollmentParts {
        ComputePluginRollbackAnchorAbsenceEnrollmentParts {
            anchor_id: self.signed.attestation.anchor_id,
            challenge_id: self.signed.attestation.challenge_id,
            challenge_digest: self.signed.attestation.challenge_digest,
            installation_id_digest: self.signed.attestation.installation_id_digest,
            absence_attestation_digest: self.signed.attestation_digest,
            expires_at: self.expires_at,
        }
    }
}

impl fmt::Debug for VerifiedComputePluginRollbackAnchorAbsence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedComputePluginRollbackAnchorAbsence")
            .field("anchor_id", &self.signed.attestation.anchor_id)
            .field("installation_id_digest", &"<redacted>")
            .field("attestation_digest", &"<redacted>")
            .field("signing_key_fingerprint", &"<redacted>")
            .field("expires_at", &"<monotonic>")
            .field("verified_at", &"<monotonic>")
            .finish()
    }
}

pub(in crate::node_agent_compute_plugin_host) fn verify_rollback_anchor_absence_attestation(
    challenge: ComputePluginRollbackAnchorChallenge,
    signed: ComputePluginSignedRollbackAnchorAbsenceAttestation,
    resolver: &dyn ComputePluginRollbackAnchorKeyResolver,
) -> Result<VerifiedComputePluginRollbackAnchorAbsence> {
    challenge.ensure_live(Instant::now())?;
    validate_challenge(challenge.request())?;
    validate_absence_binding(challenge.request(), &signed)?;
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
        ROLLBACK_ANCHOR_ABSENCE_SIGNATURE_DOMAIN,
        &key,
    )?;
    let verified_at = Instant::now();
    challenge.ensure_live(verified_at)?;

    Ok(VerifiedComputePluginRollbackAnchorAbsence {
        signed,
        signing_key_fingerprint: key.fingerprint(),
        expires_at: challenge.expires_at(),
        verified_at,
    })
}

fn validate_absence_binding(
    request: &super::ComputePluginRollbackAnchorChallengeRequest,
    signed: &ComputePluginSignedRollbackAnchorAbsenceAttestation,
) -> Result<()> {
    let attestation = &signed.attestation;
    if signed.schema != SIGNED_ROLLBACK_ANCHOR_ABSENCE_ATTESTATION_SCHEMA
        || attestation.schema != ROLLBACK_ANCHOR_ABSENCE_ATTESTATION_SCHEMA
        || attestation.anchor_state != ROLLBACK_ANCHOR_ABSENT_STATE
        || signed.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || signed.attestation_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || signed.signature.algorithm != COMPUTE_PLUGIN_SIGNATURE_ALGORITHM
        || !is_sha256(&signed.attestation_digest)
        || attestation.anchor_id != request.challenge.anchor_id
        || attestation.challenge_id != request.challenge.challenge_id
        || attestation.challenge_digest != request.challenge_digest
        || attestation.installation_id_digest != request.challenge.installation_id_digest
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_ABSENCE_BINDING_INVALID");
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
    )
}
