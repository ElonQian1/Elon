use anyhow::{ensure, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::*;

const RECEIPT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ROUTE-RENEWAL-RECEIPT-V1";
const ID_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ROUTE-RENEWAL-ID-V1";
const POLICY_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ROUTE-RENEWAL-POLICY-V1";
const IDEMPOTENCY_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ROUTE-RENEWAL-IDEMPOTENCY-V1";

pub(crate) fn external_pool_adapter_route_renewal_policy() -> ExternalPoolAdapterRouteRenewalPolicy
{
    ExternalPoolAdapterRouteRenewalPolicy {
        renew_before_seconds: ROUTE_RENEWAL_RENEW_BEFORE_SECONDS,
        fresh_max_seconds: ROUTE_RENEWAL_FRESH_MAX_SECONDS,
        cleanup_max_seconds: ROUTE_RENEWAL_CLEANUP_MAX_SECONDS,
    }
}

pub(crate) fn canonical_external_pool_adapter_route_renewal_policy_digest() -> Result<String> {
    domain_digest(POLICY_DOMAIN, &external_pool_adapter_route_renewal_policy())
}

pub(crate) fn canonical_external_pool_adapter_route_renewal_idempotency_json_and_digest(
    material: &ExternalPoolAdapterRouteRenewalIdempotencyMaterial,
) -> Result<(String, String)> {
    material_json_and_digest(IDEMPOTENCY_DOMAIN, material)
}

pub(crate) fn derive_external_pool_adapter_route_renewal_receipt_id(
    idempotency_digest: &str,
) -> Result<String> {
    ensure!(
        is_digest(idempotency_digest),
        "V278 idempotency digest is malformed"
    );
    let digest = domain_digest(ID_DOMAIN, idempotency_digest)?;
    Ok(format!("external_pool_route_renewal_{digest}"))
}

pub(crate) fn derive_external_pool_adapter_route_renewal_leaf_id(
    receipt_id: &str,
    leaf: &str,
) -> Result<String> {
    ensure!(
        !receipt_id.is_empty() && !leaf.is_empty(),
        "V278 leaf identity is empty"
    );
    let digest = domain_digest(ID_DOMAIN, &(receipt_id, leaf))?;
    Ok(format!("v278_{leaf}_{digest}"))
}

pub(crate) fn canonical_external_pool_adapter_route_renewal_receipt_json_and_digest(
    receipt: &ExternalPoolAdapterRouteRenewalReceipt,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct Projection<'a> {
        schema: &'a str,
        route_renewal_receipt_id: &'a str,
        canonicalization: &'a str,
        digest_algorithm: &'a str,
        renewal: &'a ExternalPoolAdapterRouteRenewalMaterial,
    }
    let projection = Projection {
        schema: &receipt.schema,
        route_renewal_receipt_id: &receipt.route_renewal_receipt_id,
        canonicalization: &receipt.canonicalization,
        digest_algorithm: &receipt.digest_algorithm,
        renewal: &receipt.renewal,
    };
    Ok((
        canonical_json(receipt)?,
        domain_digest(RECEIPT_DOMAIN, &projection)?,
    ))
}

pub(crate) fn build_external_pool_adapter_route_renewal_receipt_from_material(
    route_renewal_receipt_id: String,
    renewal: ExternalPoolAdapterRouteRenewalMaterial,
) -> Result<ExternalPoolAdapterRouteRenewalReceipt> {
    let mut receipt = ExternalPoolAdapterRouteRenewalReceipt {
        schema: ROUTE_RENEWAL_RECEIPT_SCHEMA.into(),
        route_renewal_receipt_id,
        route_renewal_receipt_digest: String::new(),
        canonicalization: ROUTE_RENEWAL_CANONICALIZATION.into(),
        digest_algorithm: ROUTE_RENEWAL_DIGEST_ALGORITHM.into(),
        renewal,
    };
    receipt.route_renewal_receipt_digest =
        canonical_external_pool_adapter_route_renewal_receipt_json_and_digest(&receipt)?.1;
    validate_external_pool_adapter_route_renewal_receipt(&receipt)?;
    Ok(receipt)
}

pub(crate) fn canonical_external_pool_adapter_route_renewal_capabilities_json(
    capabilities: &[crate::compute_federation::route_authority::ComputeRouteCapabilityBinding],
) -> Result<String> {
    canonical_json(capabilities)
}

pub(crate) fn route_renewal_json_is_canonical(json: &str) -> bool {
    if json.len() > ROUTE_RENEWAL_MAX_JSON_BYTES {
        return false;
    }
    serde_json::from_str::<ExternalPoolAdapterRouteRenewalReceipt>(json).is_ok_and(|receipt| {
        validate_external_pool_adapter_route_renewal_receipt(&receipt).is_ok()
            && canonical_external_pool_adapter_route_renewal_receipt_json_and_digest(&receipt)
                .is_ok_and(|(canonical, _)| canonical == json)
    })
}

fn material_json_and_digest<T: Serialize>(domain: &[u8], value: &T) -> Result<(String, String)> {
    let json = canonical_json(value)?;
    let digest = domain_digest_from_json(domain, &json);
    Ok((json, digest))
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    Ok(domain_digest_from_json(domain, &canonical_json(value)?))
}

fn domain_digest_from_json(domain: &[u8], json: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    hex::encode(digest.finalize())
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, ROUTE_RENEWAL_MAX_JSON_BYTES)
        .map(|(json, _)| json)
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
