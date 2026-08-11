use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::external_pool_adapter_release::{
        canonical_external_pool_adapter_release_capability_set_digest,
        ComputeExternalPoolAdapterReleaseCapability,
    },
    compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256,
};

use super::types::{StoredAdmissionEnvelope, StoredReviewEnvelope};

const MAX_ADAPTER_RELEASE_JSON_BYTES: usize = 512 * 1024;
const REVIEW_DOMAIN: &[u8] = b"ELON-COMPUTE-EXTERNAL-POOL-ADAPTER-RELEASE-REVIEW-V1";
const ADMISSION_DOMAIN: &[u8] = b"ELON-COMPUTE-EXTERNAL-POOL-ADAPTER-RELEASE-ADMISSION-V1";

pub(super) fn canonical_review_json_and_digest(
    envelope: &StoredReviewEnvelope,
) -> Result<(String, String)> {
    envelope_json_and_digest(
        REVIEW_DOMAIN,
        envelope,
        "review_digest",
        &envelope.review_digest,
    )
}

pub(super) fn canonical_admission_json_and_digest(
    envelope: &StoredAdmissionEnvelope,
) -> Result<(String, String)> {
    envelope_json_and_digest(
        ADMISSION_DOMAIN,
        envelope,
        "admission_digest",
        &envelope.admission_digest,
    )
}

pub(super) fn canonical_capabilities_json_and_digest(
    capabilities: &[ComputeExternalPoolAdapterReleaseCapability],
) -> Result<(String, String)> {
    let (json, _) =
        canonical_compute_plugin_ijson_and_sha256(capabilities, MAX_ADAPTER_RELEASE_JSON_BYTES)?;
    let digest = canonical_external_pool_adapter_release_capability_set_digest(capabilities)?;
    Ok((json, digest))
}

pub(super) fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, MAX_ADAPTER_RELEASE_JSON_BYTES)
        .map(|(json, _)| json)
}

fn envelope_json_and_digest<E: Serialize>(
    domain: &[u8],
    envelope: &E,
    digest_field: &str,
    stored_digest: &str,
) -> Result<(String, String)> {
    let value = serde_json::to_value(envelope)?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Adapter release envelope is not an object"))?;
    let mut projection = object.clone();
    if projection
        .insert(
            digest_field.to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("Adapter release envelope lacks {digest_field}");
    }
    let projection_json = canonical_json(&projection)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(projection_json.as_bytes());
    let computed = hex::encode(digest.finalize());
    let json = canonical_json(envelope)?;
    if !stored_digest.is_empty() && stored_digest != computed {
        bail!("Adapter release envelope digest mismatch");
    }
    Ok((json, computed))
}
