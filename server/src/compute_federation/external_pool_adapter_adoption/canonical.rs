use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{
    ExternalPoolAdapterAdoptionReceipt, ExternalPoolAdapterAdoptionTerminalReceipt,
};

const MAX_JSON_BYTES: usize = 512 * 1024;
const ADOPTION_MATERIAL_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ADOPTION-MATERIAL-V1";
const ADOPTION_RECEIPT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ADOPTION-RECEIPT-V1";
const TERMINAL_MATERIAL_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ADOPTION-TERMINAL-MATERIAL-V1";
const TERMINAL_RECEIPT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ADOPTION-TERMINAL-RECEIPT-V1";

pub(crate) fn adoption_material_digest<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    domain_digest(ADOPTION_MATERIAL_DOMAIN, value)
}

pub(crate) fn adoption_terminal_material_digest<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    domain_digest(TERMINAL_MATERIAL_DOMAIN, value)
}

pub(crate) fn canonical_adoption_receipt_json_and_digest(
    receipt: &ExternalPoolAdapterAdoptionReceipt,
) -> Result<(String, String)> {
    receipt_json_and_digest(receipt, "adoption_receipt_digest", ADOPTION_RECEIPT_DOMAIN)
}

pub(crate) fn canonical_adoption_terminal_receipt_json_and_digest(
    receipt: &ExternalPoolAdapterAdoptionTerminalReceipt,
) -> Result<(String, String)> {
    receipt_json_and_digest(receipt, "terminal_receipt_digest", TERMINAL_RECEIPT_DOMAIN)
}

fn receipt_json_and_digest<T: Serialize>(
    receipt: &T,
    digest_field: &str,
    domain: &[u8],
) -> Result<(String, String)> {
    let value = serde_json::to_value(receipt)?;
    let mut projection = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("adoption receipt must be an object"))?
        .clone();
    if projection
        .insert(
            digest_field.to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("adoption receipt lacks digest field");
    }
    Ok((
        canonical_json(receipt)?,
        domain_digest(domain, &projection)?,
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
