use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{
    ExternalPoolAdapterArtifactSigningKeyActivation,
    ExternalPoolAdapterArtifactSigningKeyActivationReceipt,
    ExternalPoolAdapterArtifactSigningKeyRecord, ExternalPoolAdapterArtifactSigningKeyRegistration,
    ExternalPoolAdapterArtifactSigningKeyRevocation,
    ExternalPoolAdapterArtifactSigningKeyRevocationReceipt,
};

const MAX_SIGNING_KEY_JSON_BYTES: usize = 128 * 1024;
const RECORD_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-SIGNING-KEY-RECORD-V1";
const REGISTRATION_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-SIGNING-KEY-REGISTRATION-MATERIAL-V1";
const ACTIVATION_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-SIGNING-KEY-ACTIVATION-RECEIPT-V1";
const ACTIVATION_MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-SIGNING-KEY-ACTIVATION-MATERIAL-V1";
const REVOCATION_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-SIGNING-KEY-REVOCATION-RECEIPT-V1";
const REVOCATION_MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-SIGNING-KEY-REVOCATION-MATERIAL-V1";

pub(crate) fn canonical_signing_key_record_json_and_digest(
    record: &ExternalPoolAdapterArtifactSigningKeyRecord,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(record, "key_record_digest", RECORD_DOMAIN)
}

pub(crate) fn signing_key_registration_material_digest(
    registration: &ExternalPoolAdapterArtifactSigningKeyRegistration,
) -> Result<String> {
    domain_digest(REGISTRATION_DOMAIN, registration)
}

pub(crate) fn canonical_signing_key_activation_json_and_digest(
    receipt: &ExternalPoolAdapterArtifactSigningKeyActivationReceipt,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(receipt, "activation_receipt_digest", ACTIVATION_DOMAIN)
}

pub(crate) fn signing_key_activation_material_digest(
    activation: &ExternalPoolAdapterArtifactSigningKeyActivation,
) -> Result<String> {
    domain_digest(ACTIVATION_MATERIAL_DOMAIN, activation)
}

pub(crate) fn canonical_signing_key_revocation_json_and_digest(
    receipt: &ExternalPoolAdapterArtifactSigningKeyRevocationReceipt,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(receipt, "revocation_receipt_digest", REVOCATION_DOMAIN)
}

pub(crate) fn signing_key_revocation_material_digest(
    revocation: &ExternalPoolAdapterArtifactSigningKeyRevocation,
) -> Result<String> {
    domain_digest(REVOCATION_MATERIAL_DOMAIN, revocation)
}

fn canonical_envelope_json_and_digest<T: Serialize>(
    envelope: &T,
    digest_field: &str,
    domain: &[u8],
) -> Result<(String, String)> {
    let value = serde_json::to_value(envelope)?;
    let mut projection = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("signing-key receipt must be an object"))?
        .clone();
    if projection
        .insert(
            digest_field.to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("signing-key receipt lacks digest field");
    }
    let digest = domain_digest(domain, &projection)?;
    let json = canonical_json(envelope)?;
    Ok((json, digest))
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, MAX_SIGNING_KEY_JSON_BYTES)
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
