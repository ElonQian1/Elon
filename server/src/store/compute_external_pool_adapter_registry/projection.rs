use anyhow::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};

const ROUTE_ADAPTER_PROJECTION_ID_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-REGISTRY-ROUTE-PROJECTION-ID-V1";
const ROUTE_ADAPTER_PROJECTION_ID_PREFIX: &str = "external_pool_provider_route_adapter_";

#[derive(Serialize)]
struct RouteAdapterProjectionIdentity<'a> {
    provider_id: &'a str,
    provider_policy_revision: i64,
    provider_digest: &'a str,
    registry_release_id: &'a str,
    registry_release_digest: &'a str,
    installation_receipt_id: &'a str,
    installation_receipt_digest: &'a str,
}

pub(super) fn route_adapter_projection_id(
    provider_id: &str,
    provider_policy_revision: i64,
    provider_digest: &str,
    registry_release_id: &str,
    registry_release_digest: &str,
    installation_receipt_id: &str,
    installation_receipt_digest: &str,
) -> Result<String> {
    let identity = RouteAdapterProjectionIdentity {
        provider_id,
        provider_policy_revision,
        provider_digest,
        registry_release_id,
        registry_release_digest,
        installation_receipt_id,
        installation_receipt_digest,
    };
    let canonical =
        crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256(
            &identity,
            16 * 1024,
        )?
        .0;
    let mut digest = Sha256::new();
    digest.update(ROUTE_ADAPTER_PROJECTION_ID_DOMAIN);
    digest.update([0]);
    digest.update(canonical.as_bytes());
    Ok(format!(
        "{ROUTE_ADAPTER_PROJECTION_ID_PREFIX}{}",
        hex::encode(digest.finalize())
    ))
}
