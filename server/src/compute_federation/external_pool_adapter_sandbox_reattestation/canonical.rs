use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::*;

const MAX_JSON_BYTES: usize = 1024 * 1024;
const CHALLENGE_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-SANDBOX-REATTESTATION-CHALLENGE-V1";
const MATERIAL_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-SANDBOX-REATTESTATION-MATERIAL-V1";
const RECEIPT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-SANDBOX-REATTESTATION-RECEIPT-V1";
const REVOCATION_MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-SANDBOX-REATTESTATION-REVOCATION-MATERIAL-V1";
const REVOCATION_RECEIPT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-SANDBOX-REATTESTATION-REVOCATION-RECEIPT-V1";

pub(crate) fn sandbox_reattestation_challenge(
    binding: ExternalPoolAdapterSandboxReattestationBinding,
) -> Result<ExternalPoolAdapterSandboxReattestationChallenge> {
    let json = canonical_json(&binding)?;
    let mut message = Vec::with_capacity(CHALLENGE_DOMAIN.len() + 1 + json.len());
    message.extend_from_slice(CHALLENGE_DOMAIN);
    message.push(0);
    message.extend_from_slice(json.as_bytes());
    Ok(ExternalPoolAdapterSandboxReattestationChallenge {
        schema: SANDBOX_REATTESTATION_CHALLENGE_SCHEMA,
        canonicalization: SANDBOX_REATTESTATION_CANONICALIZATION,
        digest_algorithm: SANDBOX_REATTESTATION_DIGEST_ALGORITHM,
        signature_algorithm: SANDBOX_REATTESTATION_SIGNATURE_ALGORITHM,
        signature_message_base64: STANDARD.encode(&message),
        signature_message_digest: hex::encode(Sha256::digest(&message)),
        binding,
    })
}

pub(crate) fn sandbox_reattestation_material_digest<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    domain_digest(MATERIAL_DOMAIN, value)
}

pub(crate) fn sandbox_reattestation_receipt_json_and_digest(
    value: &ExternalPoolAdapterSandboxReattestationReceipt,
) -> Result<(String, String)> {
    receipt_digest(value, "reattestation_receipt_digest", RECEIPT_DOMAIN)
}

pub(crate) fn sandbox_reattestation_revocation_material_digest<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    domain_digest(REVOCATION_MATERIAL_DOMAIN, value)
}

pub(crate) fn sandbox_reattestation_revocation_receipt_json_and_digest(
    value: &ExternalPoolAdapterSandboxReattestationRevocationReceipt,
) -> Result<(String, String)> {
    receipt_digest(
        value,
        "revocation_receipt_digest",
        REVOCATION_RECEIPT_DOMAIN,
    )
}

fn receipt_digest<T: Serialize>(value: &T, field: &str, domain: &[u8]) -> Result<(String, String)> {
    let json = canonical_json(value)?;
    let mut projection = serde_json::to_value(value)?
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("sandbox re-attestation receipt must be an object"))?
        .clone();
    if projection
        .insert(field.to_string(), serde_json::Value::String(String::new()))
        .is_none()
    {
        bail!("sandbox re-attestation receipt lacks digest field");
    }
    Ok((json, domain_digest(domain, &projection)?))
}

pub(crate) fn canonical_sandbox_reattestation_json<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    canonical_json(value)
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, MAX_JSON_BYTES).map(|(json, _)| json)
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let json = canonical_json(value)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
