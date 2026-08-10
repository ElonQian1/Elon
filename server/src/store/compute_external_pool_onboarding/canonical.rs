use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{StoredApplicationEnvelope, StoredReviewEnvelope};

const MAX_ONBOARDING_JSON_BYTES: usize = 512 * 1024;
const REVIEW_DOMAIN: &[u8] = b"ELON-COMPUTE-EXTERNAL-POOL-ONBOARDING-REVIEW-V1";
const APPLICATION_DOMAIN: &[u8] = b"ELON-COMPUTE-EXTERNAL-POOL-ONBOARDING-APPLICATION-V1";

pub(super) fn canonical_review_json_and_digest(
    envelope: &StoredReviewEnvelope,
) -> Result<(String, String)> {
    envelope_json_and_digest(REVIEW_DOMAIN, envelope, &envelope.review_digest)
}

pub(super) fn canonical_application_json_and_digest(
    envelope: &StoredApplicationEnvelope,
) -> Result<(String, String)> {
    envelope_json_and_digest(APPLICATION_DOMAIN, envelope, &envelope.application_digest)
}

fn envelope_json_and_digest<E: Serialize>(
    domain: &[u8],
    envelope: &E,
    stored_digest: &str,
) -> Result<(String, String)> {
    let value = serde_json::to_value(envelope)?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("onboarding envelope is not an object"))?;
    let mut projection = object.clone();
    if projection
        .insert(
            if domain == REVIEW_DOMAIN {
                "review_digest".to_string()
            } else {
                "application_digest".to_string()
            },
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("onboarding envelope lacks its digest field");
    }
    let canonical_projection = canonical_json(&projection)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(canonical_projection.as_bytes());
    let computed = hex::encode(digest.finalize());
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(envelope, MAX_ONBOARDING_JSON_BYTES)?;
    if !stored_digest.is_empty() && stored_digest != computed {
        bail!("onboarding envelope digest mismatch");
    }
    Ok((json, computed))
}

fn canonical_json<T: Serialize>(value: &T) -> Result<String> {
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(value, MAX_ONBOARDING_JSON_BYTES)?;
    Ok(json)
}
