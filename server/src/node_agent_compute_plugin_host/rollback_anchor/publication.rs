use std::{
    fmt,
    time::{Duration, Instant},
};

use anyhow::{bail, Result};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};

use super::{
    comparison::{
        ComputePluginRollbackAnchorPublicationParts, ComputePluginRollbackAnchorPublishRequired,
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

const ROLLBACK_ANCHOR_PUBLICATION_PAYLOAD_SCHEMA: &str =
    "elon.compute_plugin.rollback_anchor_publication.v1";
const ROLLBACK_ANCHOR_PUBLICATION_REQUEST_SCHEMA: &str =
    "elon.compute_plugin.rollback_anchor_publication_request.v1";
const ROLLBACK_ANCHOR_PUBLICATION_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.rollback_anchor_publication_receipt.v1";
const SIGNED_ROLLBACK_ANCHOR_PUBLICATION_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.signed_rollback_anchor_publication_receipt.v1";
const ROLLBACK_ANCHOR_PUBLICATION_SIGNATURE_DOMAIN: &str =
    "ELON-COMPUTE-PLUGIN-ROLLBACK-ANCHOR-PUBLICATION-V1";
const ROLLBACK_ANCHOR_PUBLICATION_LIFETIME: Duration = Duration::from_secs(60);
const RANDOM_PUBLICATION_ID_BYTES: usize = 16;
const RANDOM_PUBLICATION_NONCE_BYTES: usize = 32;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorPublicationPayload {
    pub schema: String,
    pub publication_id: String,
    pub publication_nonce: String,
    pub installation_id_digest: String,
    pub anchor_id: String,
    pub expected_previous_anchor_sequence: i64,
    pub expected_previous_checkpoint_digest: String,
    pub proposed_checkpoint: HashedComputePluginAuthorityRollbackCheckpoint,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorPublicationRequest {
    pub schema: String,
    pub publication: ComputePluginRollbackAnchorPublicationPayload,
    pub canonicalization: String,
    pub publication_digest_algorithm: String,
    pub publication_digest: String,
}

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorPublicationChallenge
{
    request: ComputePluginRollbackAnchorPublicationRequest,
    expires_at: Instant,
}

impl ComputePluginRollbackAnchorPublicationChallenge {
    pub(in crate::node_agent_compute_plugin_host) fn request(
        &self,
    ) -> &ComputePluginRollbackAnchorPublicationRequest {
        &self.request
    }

    fn ensure_live(&self, now: Instant) -> Result<()> {
        if now >= self.expires_at {
            bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_PUBLICATION_EXPIRED");
        }
        Ok(())
    }
}

impl fmt::Debug for ComputePluginRollbackAnchorPublicationChallenge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginRollbackAnchorPublicationChallenge")
            .field("publication_id", &self.request.publication.publication_id)
            .field("publication_nonce", &"<redacted>")
            .field("installation_id_digest", &"<redacted>")
            .field("anchor_id", &self.request.publication.anchor_id)
            .field(
                "expected_previous_anchor_sequence",
                &self.request.publication.expected_previous_anchor_sequence,
            )
            .field("proposed_checkpoint_digest", &"<redacted>")
            .field("expires_at", &"<monotonic>")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginRollbackAnchorPublicationReceipt {
    pub schema: String,
    pub publication_id: String,
    pub publication_digest: String,
    pub installation_id_digest: String,
    pub anchor_id: String,
    pub previous_anchor_sequence: i64,
    pub previous_checkpoint_digest: String,
    pub anchor_sequence: i64,
    pub checkpoint_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginSignedRollbackAnchorPublicationReceipt
{
    pub schema: String,
    pub receipt: ComputePluginRollbackAnchorPublicationReceipt,
    pub canonicalization: String,
    pub receipt_digest_algorithm: String,
    pub receipt_digest: String,
    pub signature: ComputePluginSignature,
}

pub(in crate::node_agent_compute_plugin_host) struct ConfirmedComputePluginRollbackAnchorPublication
{
    anchor_id: String,
    anchor_sequence: i64,
    checkpoint: HashedComputePluginAuthorityRollbackCheckpoint,
    receipt_digest: String,
    signing_key_fingerprint: String,
    verified_at: Instant,
}

impl ConfirmedComputePluginRollbackAnchorPublication {
    pub(super) fn checkpoint(&self) -> &HashedComputePluginAuthorityRollbackCheckpoint {
        &self.checkpoint
    }
}

impl fmt::Debug for ConfirmedComputePluginRollbackAnchorPublication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConfirmedComputePluginRollbackAnchorPublication")
            .field("anchor_id", &self.anchor_id)
            .field("anchor_sequence", &self.anchor_sequence)
            .field("checkpoint_digest", &"<redacted>")
            .field("receipt_digest", &"<redacted>")
            .field("signing_key_fingerprint", &"<redacted>")
            .field("verified_at", &"<monotonic>")
            .finish()
    }
}

/// The request binds freshness and content only. A future transport must authenticate the node's
/// installation before the witness performs the predecessor CAS and signs a receipt.
pub(in crate::node_agent_compute_plugin_host) fn begin_rollback_anchor_publication(
    required: ComputePluginRollbackAnchorPublishRequired,
) -> Result<ComputePluginRollbackAnchorPublicationChallenge> {
    let parts = required.into_publication_parts();
    validate_publication_parts(&parts)?;

    let random = SystemRandom::new();
    let mut publication_id = [0_u8; RANDOM_PUBLICATION_ID_BYTES];
    let mut publication_nonce = [0_u8; RANDOM_PUBLICATION_NONCE_BYTES];
    random
        .fill(&mut publication_id)
        .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_PUBLICATION_ID_RANDOM"))?;
    random
        .fill(&mut publication_nonce)
        .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_PUBLICATION_NONCE_RANDOM"))?;

    let publication = ComputePluginRollbackAnchorPublicationPayload {
        schema: ROLLBACK_ANCHOR_PUBLICATION_PAYLOAD_SCHEMA.to_string(),
        publication_id: hex::encode(publication_id),
        publication_nonce: hex::encode(publication_nonce),
        installation_id_digest: parts
            .local_checkpoint
            .checkpoint
            .installation_id_digest
            .clone(),
        anchor_id: parts.anchor_id,
        expected_previous_anchor_sequence: parts.anchor_sequence,
        expected_previous_checkpoint_digest: parts.anchored_checkpoint_digest,
        proposed_checkpoint: parts.local_checkpoint,
    };
    let publication_digest = jcs_sha256_hex(&publication)?;
    let expires_at = Instant::now()
        .checked_add(ROLLBACK_ANCHOR_PUBLICATION_LIFETIME)
        .ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_PUBLICATION_DEADLINE_OVERFLOW")
        })?;
    Ok(ComputePluginRollbackAnchorPublicationChallenge {
        request: ComputePluginRollbackAnchorPublicationRequest {
            schema: ROLLBACK_ANCHOR_PUBLICATION_REQUEST_SCHEMA.to_string(),
            publication,
            canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
            publication_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
            publication_digest,
        },
        expires_at,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn verify_rollback_anchor_publication_receipt(
    challenge: ComputePluginRollbackAnchorPublicationChallenge,
    signed: ComputePluginSignedRollbackAnchorPublicationReceipt,
    resolver: &dyn ComputePluginRollbackAnchorKeyResolver,
) -> Result<ConfirmedComputePluginRollbackAnchorPublication> {
    challenge.ensure_live(Instant::now())?;
    validate_publication_request(&challenge.request)?;
    validate_receipt_binding(&challenge.request, &signed)?;
    let key = resolver
        .resolve_rollback_anchor_key(&signed.receipt.anchor_id, &signed.signature.signing_key_id)?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_KEY_UNTRUSTED"))?;
    challenge.ensure_live(Instant::now())?;
    verify_publication_receipt_signature(&signed, &key)?;
    let verified_at = Instant::now();
    challenge.ensure_live(verified_at)?;

    Ok(ConfirmedComputePluginRollbackAnchorPublication {
        anchor_id: signed.receipt.anchor_id,
        anchor_sequence: signed.receipt.anchor_sequence,
        checkpoint: challenge.request.publication.proposed_checkpoint,
        receipt_digest: signed.receipt_digest,
        signing_key_fingerprint: key.fingerprint(),
        verified_at,
    })
}

fn validate_publication_parts(parts: &ComputePluginRollbackAnchorPublicationParts) -> Result<()> {
    validate_opaque_identifier("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_ID", &parts.anchor_id, 160)?;
    if parts.anchor_sequence <= 0
        || parts.anchor_sequence == i64::MAX
        || !is_sha256(&parts.anchored_checkpoint_digest)
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_PUBLICATION_PREDECESSOR_INVALID");
    }
    validate_checkpoint_envelope(&parts.local_checkpoint)
}

fn validate_publication_request(
    request: &ComputePluginRollbackAnchorPublicationRequest,
) -> Result<()> {
    let publication = &request.publication;
    if request.schema != ROLLBACK_ANCHOR_PUBLICATION_REQUEST_SCHEMA
        || publication.schema != ROLLBACK_ANCHOR_PUBLICATION_PAYLOAD_SCHEMA
        || request.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || request.publication_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || !is_lower_hex(&publication.publication_id, RANDOM_PUBLICATION_ID_BYTES * 2)
        || !is_lower_hex(
            &publication.publication_nonce,
            RANDOM_PUBLICATION_NONCE_BYTES * 2,
        )
        || !is_sha256(&publication.installation_id_digest)
        || publication.expected_previous_anchor_sequence <= 0
        || !is_sha256(&publication.expected_previous_checkpoint_digest)
        || publication
            .proposed_checkpoint
            .checkpoint
            .installation_id_digest
            != publication.installation_id_digest
        || jcs_sha256_hex(publication)? != request.publication_digest
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_PUBLICATION_REQUEST_INVALID");
    }
    validate_opaque_identifier(
        "COMPUTE_PLUGIN_ROLLBACK_ANCHOR_ID",
        &publication.anchor_id,
        160,
    )?;
    validate_checkpoint_envelope(&publication.proposed_checkpoint)
}

fn validate_receipt_binding(
    request: &ComputePluginRollbackAnchorPublicationRequest,
    signed: &ComputePluginSignedRollbackAnchorPublicationReceipt,
) -> Result<()> {
    let publication = &request.publication;
    let receipt = &signed.receipt;
    let expected_anchor_sequence = publication
        .expected_previous_anchor_sequence
        .checked_add(1)
        .ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_PUBLICATION_SEQUENCE_OVERFLOW")
        })?;
    if signed.schema != SIGNED_ROLLBACK_ANCHOR_PUBLICATION_RECEIPT_SCHEMA
        || receipt.schema != ROLLBACK_ANCHOR_PUBLICATION_RECEIPT_SCHEMA
        || signed.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || signed.receipt_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || signed.signature.algorithm != COMPUTE_PLUGIN_SIGNATURE_ALGORITHM
        || !is_sha256(&signed.receipt_digest)
        || receipt.publication_id != publication.publication_id
        || receipt.publication_digest != request.publication_digest
        || receipt.installation_id_digest != publication.installation_id_digest
        || receipt.anchor_id != publication.anchor_id
        || receipt.previous_anchor_sequence != publication.expected_previous_anchor_sequence
        || receipt.previous_checkpoint_digest != publication.expected_previous_checkpoint_digest
        || receipt.anchor_sequence != expected_anchor_sequence
        || receipt.checkpoint_digest != publication.proposed_checkpoint.checkpoint_digest
    {
        bail!("COMPUTE_PLUGIN_ROLLBACK_ANCHOR_PUBLICATION_RECEIPT_BINDING_INVALID");
    }
    validate_opaque_identifier(
        "COMPUTE_PLUGIN_ROLLBACK_ANCHOR_SIGNING_KEY_ID",
        &signed.signature.signing_key_id,
        160,
    )
}

fn verify_publication_receipt_signature(
    signed: &ComputePluginSignedRollbackAnchorPublicationReceipt,
    key: &ComputePluginEd25519PublicKey,
) -> Result<()> {
    verify_jcs_ed25519(
        &signed.receipt,
        &signed.canonicalization,
        &signed.receipt_digest_algorithm,
        &signed.receipt_digest,
        &signed.signature,
        ROLLBACK_ANCHOR_PUBLICATION_SIGNATURE_DOMAIN,
        key,
    )
}

fn is_lower_hex(value: &str, expected_characters: usize) -> bool {
    value.len() == expected_characters
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
