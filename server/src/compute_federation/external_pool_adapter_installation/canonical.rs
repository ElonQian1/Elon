use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::{ExternalPoolAdapterInstallationBinding, ExternalPoolAdapterInstallationReceipt};

const MAX_INSTALLATION_JSON_BYTES: usize = 1024 * 1024;
const CONTENT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-INSTALLATION-CONTENT-V1";
const MATERIAL_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-INSTALLATION-MATERIAL-V1";
const RECEIPT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-INSTALLATION-RECEIPT-V1";

pub(crate) fn installation_content_digest<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    domain_digest(CONTENT_DOMAIN, value)
}

pub(crate) fn installation_material_digest<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    domain_digest(MATERIAL_DOMAIN, value)
}

pub(crate) fn canonical_external_pool_adapter_installation_receipt_json_and_digest(
    receipt: &ExternalPoolAdapterInstallationReceipt,
) -> Result<(String, String)> {
    let value = serde_json::to_value(receipt)?;
    let mut projection = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("installation receipt must be an object"))?
        .clone();
    if projection
        .insert(
            "installation_receipt_digest".to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("installation receipt lacks digest field");
    }
    Ok((
        canonical_json(receipt)?,
        domain_digest(RECEIPT_DOMAIN, &projection)?,
    ))
}

pub(crate) fn binding_content_digest(
    binding: &ExternalPoolAdapterInstallationBinding,
) -> Result<String> {
    #[derive(Serialize)]
    struct Content<'a> {
        archive_sha256: &'a str,
        archive_size_bytes: u64,
        manifest_digest: &'a str,
        entry_inventory_digest: &'a str,
        runtime_kind: &'a str,
        entrypoint_path: &'a str,
        files: &'a [super::InstalledExternalPoolAdapterFile],
    }
    installation_content_digest(&Content {
        archive_sha256: &binding.archive_sha256,
        archive_size_bytes: binding.archive_size_bytes,
        manifest_digest: &binding.manifest_digest,
        entry_inventory_digest: &binding.entry_inventory_digest,
        runtime_kind: &binding.runtime_kind,
        entrypoint_path: &binding.entrypoint_path,
        files: &binding.installed_files,
    })
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, MAX_INSTALLATION_JSON_BYTES)
        .map(|(json, _)| json)
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let json = canonical_json(value)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
