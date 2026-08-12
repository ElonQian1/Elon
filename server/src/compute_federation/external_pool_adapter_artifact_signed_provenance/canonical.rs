use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{
    ExternalPoolAdapterArtifactSignatureBinding,
    ExternalPoolAdapterArtifactSignatureChallengeReceipt,
    ExternalPoolAdapterArtifactSignedProvenanceMaterial,
    ExternalPoolAdapterArtifactSignedProvenanceReceipt, ARTIFACT_SIGNATURE_CHALLENGE_SCHEMA,
    ARTIFACT_SIGNED_PROVENANCE_CANONICALIZATION, ARTIFACT_SIGNED_PROVENANCE_DIGEST_ALGORITHM,
    ARTIFACT_SIGNED_PROVENANCE_SIGNATURE_ALGORITHM,
};

const MAX_PROVENANCE_JSON_BYTES: usize = 256 * 1024;
const SIGNATURE_MESSAGE_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-SIGNATURE-MESSAGE-V1";
const CANDIDATE_REF_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-CANDIDATE-ARTIFACT-REF-V1";
const RECEIPT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-SIGNED-PROVENANCE-RECEIPT-V1";
const MATERIAL_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-SIGNED-PROVENANCE-MATERIAL-V1";

pub(crate) fn candidate_artifact_ref_digest(value: &str) -> String {
    domain_digest_bytes(CANDIDATE_REF_DOMAIN, value.as_bytes())
}

pub(crate) fn signature_challenge(
    binding: ExternalPoolAdapterArtifactSignatureBinding,
) -> Result<ExternalPoolAdapterArtifactSignatureChallengeReceipt> {
    let binding_json = canonical_json(&binding)?;
    let mut message = Vec::with_capacity(SIGNATURE_MESSAGE_DOMAIN.len() + 1 + binding_json.len());
    message.extend_from_slice(SIGNATURE_MESSAGE_DOMAIN);
    message.push(0);
    message.extend_from_slice(binding_json.as_bytes());
    Ok(ExternalPoolAdapterArtifactSignatureChallengeReceipt {
        schema: ARTIFACT_SIGNATURE_CHALLENGE_SCHEMA,
        canonicalization: ARTIFACT_SIGNED_PROVENANCE_CANONICALIZATION,
        digest_algorithm: ARTIFACT_SIGNED_PROVENANCE_DIGEST_ALGORITHM,
        signature_algorithm: ARTIFACT_SIGNED_PROVENANCE_SIGNATURE_ALGORITHM,
        signature_message_digest: hex::encode(Sha256::digest(&message)),
        signature_message_base64: STANDARD.encode(message),
        binding,
    })
}

pub(crate) fn verification_material_digest(
    material: &ExternalPoolAdapterArtifactSignedProvenanceMaterial,
) -> Result<String> {
    domain_digest(MATERIAL_DOMAIN, material)
}

pub(crate) fn canonical_signed_provenance_receipt_json_and_digest(
    receipt: &ExternalPoolAdapterArtifactSignedProvenanceReceipt,
) -> Result<(String, String)> {
    let value = serde_json::to_value(receipt)?;
    let mut projection = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("signed-provenance receipt must be an object"))?
        .clone();
    if projection
        .insert(
            "provenance_receipt_digest".to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("signed-provenance receipt lacks digest field");
    }
    Ok((
        canonical_json(receipt)?,
        domain_digest(RECEIPT_DOMAIN, &projection)?,
    ))
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, MAX_PROVENANCE_JSON_BYTES)
        .map(|(json, _)| json)
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    Ok(domain_digest_bytes(
        domain,
        canonical_json(value)?.as_bytes(),
    ))
}

fn domain_digest_bytes(domain: &[u8], value: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(value);
    hex::encode(digest.finalize())
}
