use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::*;

const POLICY_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-RUNTIME-READINESS-POLICY-V1";
const MATERIAL_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-RUNTIME-READINESS-MATERIAL-V1";
const RECEIPT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-RUNTIME-READINESS-RECEIPT-V1";
const REVOCATION_MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-RUNTIME-READINESS-REVOCATION-MATERIAL-V1";
const REVOCATION_RECEIPT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-RUNTIME-READINESS-REVOCATION-RECEIPT-V1";

pub(crate) fn provider_runtime_readiness_policy_digest(
    value: &ExternalPoolAdapterProviderRuntimeReadinessPolicy,
) -> Result<String> {
    domain_digest(POLICY_DOMAIN, value)
}

pub(crate) fn provider_runtime_readiness_material_digest<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    domain_digest(MATERIAL_DOMAIN, value)
}

pub(crate) fn provider_runtime_readiness_revocation_material_digest<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    domain_digest(REVOCATION_MATERIAL_DOMAIN, value)
}

pub(crate) fn canonical_provider_runtime_readiness_receipt_json_and_digest(
    receipt: &ExternalPoolAdapterProviderRuntimeReadinessReceipt,
) -> Result<(String, String)> {
    receipt_digest(
        receipt,
        "readiness_receipt_digest",
        RECEIPT_DOMAIN,
        "readiness receipt",
    )
}

pub(crate) fn canonical_provider_runtime_readiness_revocation_json_and_digest(
    receipt: &ExternalPoolAdapterProviderRuntimeReadinessRevocationReceipt,
) -> Result<(String, String)> {
    receipt_digest(
        receipt,
        "revocation_receipt_digest",
        REVOCATION_RECEIPT_DOMAIN,
        "readiness revocation receipt",
    )
}

fn receipt_digest<T: Serialize>(
    value: &T,
    digest_field: &str,
    domain: &[u8],
    kind: &str,
) -> Result<(String, String)> {
    let object = serde_json::to_value(value)?;
    let mut projection = object
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("{kind} must be an object"))?
        .clone();
    if projection
        .insert(
            digest_field.into(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("{kind} lacks its digest field")
    }
    Ok((canonical_json(value)?, domain_digest(domain, &projection)?))
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(
        value,
        PROVIDER_RUNTIME_READINESS_MAX_RECEIPT_JSON_BYTES,
    )
    .map(|item| item.0)
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(canonical_json(value)?.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
