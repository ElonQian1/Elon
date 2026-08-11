use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{
    StoredApplicationEnvelope, StoredReviewEnvelope, StoredSnapshotBindingEnvelope,
};

const MAX_PLATFORM_REFERENCE_PRICE_CURVE_JSON_BYTES: usize = 1024 * 1024;
const REVIEW_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-PLATFORM-REFERENCE-PRICE-CURVE-REVIEW-V1";
const APPLICATION_DIGEST_DOMAIN: &[u8] =
    b"ELON-COMPUTE-PLATFORM-REFERENCE-PRICE-CURVE-APPLICATION-V1";
const SNAPSHOT_BINDING_DIGEST_DOMAIN: &[u8] =
    b"ELON-COMPUTE-PLATFORM-REFERENCE-PRICE-CURVE-SNAPSHOT-BINDING-V1";
const SNAPSHOT_BINDING_SET_DIGEST_DOMAIN: &[u8] =
    b"ELON-COMPUTE-PLATFORM-REFERENCE-PRICE-CURVE-SNAPSHOT-BINDING-SET-V1";

pub(super) fn canonical_review_json_and_digest(
    envelope: &StoredReviewEnvelope,
) -> Result<(String, String)> {
    envelope_json_and_digest(
        REVIEW_DIGEST_DOMAIN,
        envelope,
        "review_digest",
        &envelope.review_digest,
    )
}

pub(super) fn canonical_application_json_and_digest(
    envelope: &StoredApplicationEnvelope,
) -> Result<(String, String)> {
    envelope_json_and_digest(
        APPLICATION_DIGEST_DOMAIN,
        envelope,
        "application_digest",
        &envelope.application_digest,
    )
}

pub(super) fn canonical_snapshot_binding_json_and_digest(
    envelope: &StoredSnapshotBindingEnvelope,
) -> Result<(String, String)> {
    envelope_json_and_digest(
        SNAPSHOT_BINDING_DIGEST_DOMAIN,
        envelope,
        "binding_digest",
        &envelope.binding_digest,
    )
}

pub(super) fn canonical_snapshot_binding_set_digest(binding_digests: &[String]) -> Result<String> {
    domain_digest(SNAPSHOT_BINDING_SET_DIGEST_DOMAIN, binding_digests)
}

pub(super) fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, MAX_PLATFORM_REFERENCE_PRICE_CURVE_JSON_BYTES)
        .map(|(json, _)| json)
}

fn envelope_json_and_digest<E: Serialize>(
    domain: &[u8],
    envelope: &E,
    digest_field: &str,
    stored_digest: &str,
) -> Result<(String, String)> {
    let value = serde_json::to_value(envelope)?;
    let object = value.as_object().ok_or_else(|| {
        anyhow::anyhow!("platform reference price curve envelope is not an object")
    })?;
    let mut projection = object.clone();
    if projection
        .insert(
            digest_field.to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("platform reference price curve envelope lacks {digest_field}");
    }
    let computed = domain_digest(domain, &projection)?;
    let json = canonical_json(envelope)?;
    if !stored_digest.is_empty() && stored_digest != computed {
        bail!("platform reference price curve envelope digest mismatch");
    }
    Ok((json, computed))
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let json = canonical_json(value)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
