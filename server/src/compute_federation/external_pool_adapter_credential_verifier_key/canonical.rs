use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::*;

const MAX_JSON_BYTES: usize = 128 * 1024;
const RECORD_DOMAIN: &[u8] = b"ELON-CREDENTIAL-VERIFIER-KEY-RECORD-V1";
const REGISTRATION_DOMAIN: &[u8] = b"ELON-CREDENTIAL-VERIFIER-KEY-REGISTRATION-V1";
const REVOCATION_DOMAIN: &[u8] = b"ELON-CREDENTIAL-VERIFIER-KEY-REVOCATION-V1";
const REVOCATION_RECEIPT_DOMAIN: &[u8] = b"ELON-CREDENTIAL-VERIFIER-KEY-REVOCATION-RECEIPT-V1";

pub(crate) fn record_json_and_digest(
    value: &CredentialVerifierKeyRecord,
) -> Result<(String, String)> {
    envelope(value, "key_record_digest", RECORD_DOMAIN)
}

pub(crate) fn registration_digest<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    domain_digest(REGISTRATION_DOMAIN, value)
}

pub(crate) fn revocation_json_and_digest(
    value: &CredentialVerifierKeyRevocationReceipt,
) -> Result<(String, String)> {
    envelope(
        value,
        "revocation_receipt_digest",
        REVOCATION_RECEIPT_DOMAIN,
    )
}

pub(crate) fn revocation_digest<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    domain_digest(REVOCATION_DOMAIN, value)
}

fn envelope<T: Serialize>(value: &T, field: &str, domain: &[u8]) -> Result<(String, String)> {
    let raw = serde_json::to_value(value)?;
    let mut projection = raw
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("key receipt is not an object"))?
        .clone();
    if projection
        .insert(field.into(), serde_json::Value::String(String::new()))
        .is_none()
    {
        bail!("key receipt lacks digest field");
    }
    Ok((canonical_json(value)?, domain_digest(domain, &projection)?))
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, MAX_JSON_BYTES).map(|item| item.0)
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(canonical_json(value)?.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
