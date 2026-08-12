use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{
    ExternalPoolAdapterArtifactManifest, ExternalPoolAdapterArtifactPackageInspection,
    ExternalPoolAdapterArtifactPackageReceipt,
};

const MAX_PACKAGE_JSON_BYTES: usize = 512 * 1024;
const MANIFEST_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-MANIFEST-V1";
const INVENTORY_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-INVENTORY-V1";
const INSPECTION_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-INSPECTION-V1";
const MATERIAL_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-PACKAGE-MATERIAL-V1";
const RECEIPT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-PACKAGE-RECEIPT-V1";

pub(super) fn canonical_manifest(
    manifest: &ExternalPoolAdapterArtifactManifest,
) -> Result<(String, String)> {
    let json = canonical_json(manifest)?;
    Ok((
        json.clone(),
        domain_digest_bytes(MANIFEST_DOMAIN, json.as_bytes()),
    ))
}

pub(super) fn inventory_digest<T: Serialize + ?Sized>(inventory: &T) -> Result<String> {
    domain_digest(INVENTORY_DOMAIN, inventory)
}

pub(crate) fn package_inspection_digest(
    inspection: &ExternalPoolAdapterArtifactPackageInspection,
) -> Result<String> {
    #[derive(Serialize)]
    struct Projection<'a> {
        archive_sha256: &'a str,
        archive_size_bytes: u64,
        manifest_digest: &'a str,
        entry_inventory_digest: &'a str,
        entry_count: u64,
        total_uncompressed_bytes: u64,
    }
    domain_digest(
        INSPECTION_DOMAIN,
        &Projection {
            archive_sha256: &inspection.archive_sha256,
            archive_size_bytes: inspection.archive_size_bytes,
            manifest_digest: &inspection.manifest_digest,
            entry_inventory_digest: &inspection.entry_inventory_digest,
            entry_count: inspection.entry_count,
            total_uncompressed_bytes: inspection.total_uncompressed_bytes,
        },
    )
}

pub(crate) fn package_material_digest<T: Serialize + ?Sized>(material: &T) -> Result<String> {
    domain_digest(MATERIAL_DOMAIN, material)
}

pub(crate) fn canonical_artifact_package_receipt_json_and_digest(
    receipt: &ExternalPoolAdapterArtifactPackageReceipt,
) -> Result<(String, String)> {
    let value = serde_json::to_value(receipt)?;
    let mut projection = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Artifact package receipt must be an object"))?
        .clone();
    if projection
        .insert(
            "package_receipt_digest".to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("Artifact package receipt lacks digest field");
    }
    Ok((
        canonical_json(receipt)?,
        domain_digest(RECEIPT_DOMAIN, &projection)?,
    ))
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, MAX_PACKAGE_JSON_BYTES).map(|(json, _)| json)
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
