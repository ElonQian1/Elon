use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::*;

const MAX_JSON_BYTES: usize = 1024 * 1024;
const POLICY_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-SUPERVISOR-SESSION-POLICY-V1";
const MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-SUPERVISOR-SESSION-POLICY-COMPANION-MATERIAL-V1";
const RECEIPT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-SUPERVISOR-SESSION-POLICY-COMPANION-RECEIPT-V1";
const REVOCATION_MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-SUPERVISOR-SESSION-POLICY-COMPANION-REVOCATION-MATERIAL-V1";
const REVOCATION_RECEIPT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-SUPERVISOR-SESSION-POLICY-COMPANION-REVOCATION-RECEIPT-V1";

pub(crate) fn supervisor_session_policy_digest(
    policy: &ExternalPoolAdapterSupervisorSessionPolicy,
) -> Result<String> {
    domain_digest(POLICY_DOMAIN, policy)
}
pub(crate) fn supervisor_session_companion_material_digest<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    domain_digest(MATERIAL_DOMAIN, value)
}
pub(crate) fn supervisor_session_companion_revocation_material_digest<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    domain_digest(REVOCATION_MATERIAL_DOMAIN, value)
}
pub(crate) fn canonical_supervisor_session_companion_json_and_digest(
    receipt: &ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
) -> Result<(String, String)> {
    receipt_digest(receipt, "companion_digest", RECEIPT_DOMAIN)
}
pub(crate) fn canonical_supervisor_session_companion_revocation_json_and_digest(
    receipt: &ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationReceipt,
) -> Result<(String, String)> {
    receipt_digest(receipt, "revocation_digest", REVOCATION_RECEIPT_DOMAIN)
}

fn receipt_digest<T: Serialize>(value: &T, field: &str, domain: &[u8]) -> Result<(String, String)> {
    let object = serde_json::to_value(value)?;
    let mut projection = object
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("supervisor session receipt must be an object"))?
        .clone();
    if projection
        .insert(field.into(), serde_json::Value::String(String::new()))
        .is_none()
    {
        bail!("supervisor session receipt lacks digest field");
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
