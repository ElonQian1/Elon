use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::{
    ExternalPoolAdapterRuntimeLaunchPolicy, ExternalPoolAdapterRuntimeLaunchProfileReceipt,
    ExternalPoolAdapterRuntimeLaunchProfileRevocationReceipt,
};

const MAX_JSON_BYTES: usize = 1024 * 1024;
const POLICY_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-LAUNCH-POLICY-V1";
const PROFILE_MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-LAUNCH-PROFILE-MATERIAL-V1";
const PROFILE_RECEIPT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-LAUNCH-PROFILE-RECEIPT-V1";
const REVOCATION_MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-LAUNCH-PROFILE-REVOCATION-MATERIAL-V1";
const REVOCATION_RECEIPT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-LAUNCH-PROFILE-REVOCATION-RECEIPT-V1";
const ENTRYPOINT_PATH_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ENTRYPOINT-PATH-V1";

pub(crate) fn runtime_launch_policy_digest(
    policy: &ExternalPoolAdapterRuntimeLaunchPolicy,
) -> Result<String> {
    domain_digest(POLICY_DOMAIN, policy)
}

pub(crate) fn runtime_launch_profile_material_digest<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    domain_digest(PROFILE_MATERIAL_DOMAIN, value)
}

pub(crate) fn runtime_launch_profile_revocation_material_digest<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    domain_digest(REVOCATION_MATERIAL_DOMAIN, value)
}

pub(crate) fn runtime_launch_entrypoint_path_digest(path: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(ENTRYPOINT_PATH_DOMAIN);
    digest.update([0]);
    digest.update(path.as_bytes());
    hex::encode(digest.finalize())
}

pub(crate) fn canonical_runtime_launch_profile_json_and_digest(
    receipt: &ExternalPoolAdapterRuntimeLaunchProfileReceipt,
) -> Result<(String, String)> {
    receipt_digest(receipt, "profile_digest", PROFILE_RECEIPT_DOMAIN)
}

pub(crate) fn canonical_runtime_launch_profile_revocation_json_and_digest(
    receipt: &ExternalPoolAdapterRuntimeLaunchProfileRevocationReceipt,
) -> Result<(String, String)> {
    receipt_digest(receipt, "revocation_digest", REVOCATION_RECEIPT_DOMAIN)
}

fn receipt_digest<T: Serialize>(value: &T, field: &str, domain: &[u8]) -> Result<(String, String)> {
    let object = serde_json::to_value(value)?;
    let mut projection = object
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("runtime launch receipt must be an object"))?
        .clone();
    if projection
        .insert(field.into(), serde_json::Value::String(String::new()))
        .is_none()
    {
        bail!("runtime launch receipt lacks digest field");
    }
    Ok((canonical_json(value)?, domain_digest(domain, &projection)?))
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, MAX_JSON_BYTES).map(|(json, _)| json)
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(canonical_json(value)?.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
