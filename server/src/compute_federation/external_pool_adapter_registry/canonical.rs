use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::{
    ExternalPoolAdapterRegistryProviderBindingReceipt, ExternalPoolAdapterRegistryReleaseReceipt,
};

const MAX_REGISTRY_JSON_BYTES: usize = 1024 * 1024;
const RELEASE_MATERIAL_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-REGISTRY-RELEASE-MATERIAL-V1";
const RELEASE_RECEIPT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-REGISTRY-RELEASE-RECEIPT-V1";
const BINDING_MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-REGISTRY-PROVIDER-BINDING-MATERIAL-V1";
const BINDING_RECEIPT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-REGISTRY-PROVIDER-BINDING-RECEIPT-V1";

pub(crate) fn registry_release_material_digest<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    domain_digest(RELEASE_MATERIAL_DOMAIN, value)
}

pub(crate) fn registry_provider_binding_material_digest<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    domain_digest(BINDING_MATERIAL_DOMAIN, value)
}

pub(crate) fn canonical_registry_release_receipt_json_and_digest(
    receipt: &ExternalPoolAdapterRegistryReleaseReceipt,
) -> Result<(String, String)> {
    receipt_digest(receipt, "registry_release_digest", RELEASE_RECEIPT_DOMAIN)
}

pub(crate) fn canonical_registry_provider_binding_receipt_json_and_digest(
    receipt: &ExternalPoolAdapterRegistryProviderBindingReceipt,
) -> Result<(String, String)> {
    receipt_digest(receipt, "provider_binding_digest", BINDING_RECEIPT_DOMAIN)
}

fn receipt_digest<T: Serialize>(value: &T, field: &str, domain: &[u8]) -> Result<(String, String)> {
    let object = serde_json::to_value(value)?;
    let mut projection = object
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("registry receipt must be an object"))?
        .clone();
    if projection
        .insert(field.to_string(), serde_json::Value::String(String::new()))
        .is_none()
    {
        bail!("registry receipt lacks digest field");
    }
    Ok((canonical_json(value)?, domain_digest(domain, &projection)?))
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, MAX_REGISTRY_JSON_BYTES).map(|(json, _)| json)
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(canonical_json(value)?.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
