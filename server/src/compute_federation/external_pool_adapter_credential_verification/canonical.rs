use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::*;

const MAX_JSON_BYTES: usize = 512 * 1024;
const CHALLENGE_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-CREDENTIAL-VERIFICATION-V1";
const LOCATOR_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-NON-BEARER-CREDENTIAL-LOCATOR-V1";
const MATERIAL_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-CREDENTIAL-VERIFICATION-MATERIAL-V1";
const RECEIPT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-CREDENTIAL-VERIFICATION-RECEIPT-V1";

pub(crate) fn credential_locator_commitment(value: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(LOCATOR_DOMAIN);
    digest.update([0]);
    digest.update(value.as_bytes());
    hex::encode(digest.finalize())
}

pub(crate) fn credential_ref_scheme(value: &str) -> Result<&'static str> {
    if value.starts_with("vault-ref:") {
        Ok("vault_ref")
    } else if value.starts_with("gateway-ref:") {
        Ok("gateway_ref")
    } else {
        bail!("credential locator scheme is unsupported")
    }
}

pub(crate) fn credential_verification_challenge(
    binding: ExternalPoolAdapterCredentialVerificationBinding,
) -> Result<ExternalPoolAdapterCredentialVerificationChallenge> {
    let json = canonical_json(&binding)?;
    let mut message = Vec::with_capacity(CHALLENGE_DOMAIN.len() + 1 + json.len());
    message.extend_from_slice(CHALLENGE_DOMAIN);
    message.push(0);
    message.extend_from_slice(json.as_bytes());
    Ok(ExternalPoolAdapterCredentialVerificationChallenge {
        schema: CREDENTIAL_VERIFICATION_CHALLENGE_SCHEMA,
        canonicalization: CREDENTIAL_VERIFICATION_CANONICALIZATION,
        digest_algorithm: CREDENTIAL_VERIFICATION_DIGEST_ALGORITHM,
        signature_algorithm: CREDENTIAL_VERIFICATION_SIGNATURE_ALGORITHM,
        signature_message_base64: STANDARD.encode(&message),
        signature_message_digest: hex::encode(Sha256::digest(&message)),
        binding,
    })
}

pub(crate) fn credential_verification_material_digest<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    domain_digest(MATERIAL_DOMAIN, value)
}

pub(crate) fn canonical_credential_verification_receipt_json_and_digest(
    receipt: &ExternalPoolAdapterCredentialVerificationReceipt,
) -> Result<(String, String)> {
    let value = serde_json::to_value(receipt)?;
    let mut projection = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("credential verification receipt must be an object"))?
        .clone();
    if projection
        .insert(
            "credential_verification_receipt_digest".to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("credential verification receipt lacks digest field");
    }
    Ok((
        canonical_json(receipt)?,
        domain_digest(RECEIPT_DOMAIN, &projection)?,
    ))
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
