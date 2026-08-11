use anyhow::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{
    ComputeExternalPoolAdapterReleaseCapability, ComputeExternalPoolAdapterReleaseIntent,
    ComputeExternalPoolAdapterReleaseRequest, ComputeExternalPoolAdapterReleaseRequestEnvelope,
};

const MAX_EXTERNAL_POOL_ADAPTER_RELEASE_JSON_BYTES: usize = 512 * 1024;
const REQUEST_ENVELOPE_DIGEST_DOMAIN: &[u8] =
    b"ELON-COMPUTE-EXTERNAL-POOL-ADAPTER-RELEASE-REQUEST-V1";
const REQUEST_MATERIAL_DIGEST_DOMAIN: &[u8] =
    b"ELON-COMPUTE-EXTERNAL-POOL-ADAPTER-RELEASE-MATERIAL-V1";
const CAPABILITY_SET_DIGEST_DOMAIN: &[u8] =
    b"ELON-COMPUTE-EXTERNAL-POOL-ADAPTER-RELEASE-CAPABILITY-SET-V1";

/// Returns full RFC 8785/JCS JSON and the domain-separated digest excluding `request_digest`.
pub(crate) fn canonical_external_pool_adapter_release_request_json_and_digest(
    envelope: &ComputeExternalPoolAdapterReleaseRequestEnvelope,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct DigestProjection<'a> {
        schema: &'a str,
        request_id: &'a str,
        request_material_digest: &'a str,
        canonicalization: &'a str,
        digest_algorithm: &'a str,
        request: &'a ComputeExternalPoolAdapterReleaseRequest,
    }

    let projection = DigestProjection {
        schema: &envelope.schema,
        request_id: &envelope.request_id,
        request_material_digest: &envelope.request_material_digest,
        canonicalization: &envelope.canonicalization,
        digest_algorithm: &envelope.digest_algorithm,
        request: &envelope.request,
    };
    let digest = domain_digest(REQUEST_ENVELOPE_DIGEST_DOMAIN, &projection)?;
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(
        envelope,
        MAX_EXTERNAL_POOL_ADAPTER_RELEASE_JSON_BYTES,
    )?;
    Ok((json, digest))
}

/// Stable exact-material digest for idempotency; it is not a review or admission receipt.
pub(crate) fn canonical_external_pool_adapter_release_request_material_digest(
    request: &ComputeExternalPoolAdapterReleaseRequest,
) -> Result<String> {
    #[derive(Serialize)]
    struct MaterialProjection<'a> {
        submitted_by_admin_user_id: &'a str,
        release: &'a ComputeExternalPoolAdapterReleaseIntent,
        idempotency_key: &'a str,
        confirmation: &'a str,
        submission_note: &'a str,
    }

    domain_digest(
        REQUEST_MATERIAL_DIGEST_DOMAIN,
        &MaterialProjection {
            submitted_by_admin_user_id: &request.submitted_by_admin_user_id,
            release: &request.release,
            idempotency_key: &request.idempotency_key,
            confirmation: &request.confirmation,
            submission_note: &request.submission_note,
        },
    )
}

/// Canonical binding of the declared ordered capability list; it is not conformance evidence.
pub(crate) fn canonical_external_pool_adapter_release_capability_set_digest(
    capabilities: &[ComputeExternalPoolAdapterReleaseCapability],
) -> Result<String> {
    domain_digest(CAPABILITY_SET_DIGEST_DOMAIN, capabilities)
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(
        value,
        MAX_EXTERNAL_POOL_ADAPTER_RELEASE_JSON_BYTES,
    )?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
