use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{
    ExternalPoolAdapterArtifactSbom, ExternalPoolAdapterArtifactSecurityInspection,
    ExternalPoolAdapterArtifactSecurityReceipt,
};

const MAX_SECURITY_JSON_BYTES: usize = 1024 * 1024;
const SBOM_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-SBOM-V1";
const COMPONENT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-SBOM-COMPONENTS-V1";
const LICENSE_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-SBOM-LICENSES-V1";
const FILE_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-STATIC-SCANNED-FILES-V1";
const RULE_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-STATIC-RULE-SET-V1";
const INSPECTION_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-STATIC-SECURITY-INSPECTION-V1";
const MATERIAL_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-STATIC-SECURITY-MATERIAL-V1";
const RECEIPT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-STATIC-SECURITY-RECEIPT-V1";

pub(crate) fn canonical_sbom(sbom: &ExternalPoolAdapterArtifactSbom) -> Result<(String, String)> {
    let json = canonical_json(sbom)?;
    Ok((
        json.clone(),
        domain_digest_bytes(SBOM_DOMAIN, json.as_bytes()),
    ))
}

pub(super) fn component_inventory_digest<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    domain_digest(COMPONENT_DOMAIN, value)
}

pub(super) fn license_inventory_digest<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    domain_digest(LICENSE_DOMAIN, value)
}

pub(super) fn scanned_file_inventory_digest<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    domain_digest(FILE_DOMAIN, value)
}

pub(crate) fn scanner_rule_set_digest<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    domain_digest(RULE_DOMAIN, value)
}

pub(crate) fn security_inspection_digest(
    inspection: &ExternalPoolAdapterArtifactSecurityInspection,
) -> Result<String> {
    #[derive(Serialize)]
    struct Projection<'a> {
        archive_sha256: &'a str,
        archive_size_bytes: u64,
        package_receipt_digest: &'a str,
        package_inspection_digest: &'a str,
        manifest_digest: &'a str,
        sbom_digest: &'a str,
        component_inventory_digest: &'a str,
        component_count: u64,
        license_inventory_digest: &'a str,
        license_count: u64,
        scanned_file_inventory_digest: &'a str,
        scanned_file_count: u64,
        scanner_rule_set_id: &'a str,
        scanner_rule_set_digest: &'a str,
        finding_count: u64,
    }
    domain_digest(
        INSPECTION_DOMAIN,
        &Projection {
            archive_sha256: &inspection.archive_sha256,
            archive_size_bytes: inspection.archive_size_bytes,
            package_receipt_digest: &inspection.package_receipt_digest,
            package_inspection_digest: &inspection.package_inspection_digest,
            manifest_digest: &inspection.manifest_digest,
            sbom_digest: &inspection.sbom_digest,
            component_inventory_digest: &inspection.component_inventory_digest,
            component_count: inspection.component_count,
            license_inventory_digest: &inspection.license_inventory_digest,
            license_count: inspection.license_count,
            scanned_file_inventory_digest: &inspection.scanned_file_inventory_digest,
            scanned_file_count: inspection.scanned_file_count,
            scanner_rule_set_id: &inspection.scanner_rule_set_id,
            scanner_rule_set_digest: &inspection.scanner_rule_set_digest,
            finding_count: inspection.finding_count,
        },
    )
}

pub(crate) fn security_material_digest<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    domain_digest(MATERIAL_DOMAIN, value)
}

pub(crate) fn canonical_artifact_security_receipt_json_and_digest(
    receipt: &ExternalPoolAdapterArtifactSecurityReceipt,
) -> Result<(String, String)> {
    let value = serde_json::to_value(receipt)?;
    let mut projection = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("security receipt must be an object"))?
        .clone();
    if projection
        .insert(
            "security_receipt_digest".to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("security receipt lacks digest field");
    }
    Ok((
        canonical_json(receipt)?,
        domain_digest(RECEIPT_DOMAIN, &projection)?,
    ))
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, MAX_SECURITY_JSON_BYTES).map(|(json, _)| json)
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
