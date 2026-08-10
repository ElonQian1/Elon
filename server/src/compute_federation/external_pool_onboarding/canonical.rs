use anyhow::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{
    ComputeExternalPoolOnboardingRequest, ComputeExternalPoolOnboardingRequestEnvelope,
};

const MAX_EXTERNAL_POOL_ONBOARDING_JSON_BYTES: usize = 512 * 1024;
const REQUEST_ENVELOPE_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-EXTERNAL-POOL-ONBOARDING-REQUEST-V1";
const REQUEST_MATERIAL_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-EXTERNAL-POOL-ONBOARDING-MATERIAL-V1";

/// Returns the full RFC 8785/JCS envelope JSON and a digest that excludes `request_digest`.
pub(crate) fn canonical_external_pool_onboarding_request_json_and_digest(
    envelope: &ComputeExternalPoolOnboardingRequestEnvelope,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct DigestProjection<'a> {
        schema: &'a str,
        request_id: &'a str,
        canonicalization: &'a str,
        digest_algorithm: &'a str,
        request: &'a ComputeExternalPoolOnboardingRequest,
    }
    let projection = DigestProjection {
        schema: &envelope.schema,
        request_id: &envelope.request_id,
        canonicalization: &envelope.canonicalization,
        digest_algorithm: &envelope.digest_algorithm,
        request: &envelope.request,
    };
    let digest = domain_digest(REQUEST_ENVELOPE_DIGEST_DOMAIN, &projection)?;
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(
        envelope,
        MAX_EXTERNAL_POOL_ONBOARDING_JSON_BYTES,
    )?;
    Ok((json, digest))
}

/// Stable material digest for exact idempotency comparison; it is not an approval receipt.
pub(crate) fn canonical_external_pool_onboarding_request_material_digest(
    request: &ComputeExternalPoolOnboardingRequest,
) -> Result<String> {
    domain_digest(REQUEST_MATERIAL_DIGEST_DOMAIN, request)
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let (json, _) =
        canonical_compute_plugin_ijson_and_sha256(value, MAX_EXTERNAL_POOL_ONBOARDING_JSON_BYTES)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
