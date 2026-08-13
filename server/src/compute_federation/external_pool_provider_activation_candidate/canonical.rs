use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::{
    ExternalPoolProviderActivationCandidateReceipt,
    ExternalPoolProviderActivationDelegationReceipt,
    ExternalPoolProviderActivationDelegationRevocationReceipt,
};

const MAX_JSON_BYTES: usize = 1024 * 1024;
const DELEGATION_MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-PROVIDER-ACTIVATION-DELEGATION-MATERIAL-V1";
const DELEGATION_RECEIPT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-PROVIDER-ACTIVATION-DELEGATION-RECEIPT-V1";
const CANDIDATE_MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-PROVIDER-ACTIVATION-CANDIDATE-MATERIAL-V1";
const CANDIDATE_RECEIPT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-PROVIDER-ACTIVATION-CANDIDATE-RECEIPT-V1";
const REVOCATION_MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-PROVIDER-ACTIVATION-DELEGATION-REVOCATION-MATERIAL-V1";
const REVOCATION_RECEIPT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-PROVIDER-ACTIVATION-DELEGATION-REVOCATION-RECEIPT-V1";
const SERVICE_ACTOR_ID_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-PROVIDER-ACTIVATION-SERVICE-ACTOR-ID-V1";
const COMPATIBILITY_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-PROVIDER-ACTIVATION-LOGICAL-PROJECTION-V1";

pub(crate) fn activation_delegation_material_digest<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    domain_digest(DELEGATION_MATERIAL_DOMAIN, value)
}

pub(crate) fn activation_candidate_material_digest<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    domain_digest(CANDIDATE_MATERIAL_DOMAIN, value)
}

pub(crate) fn activation_delegation_revocation_material_digest<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    domain_digest(REVOCATION_MATERIAL_DOMAIN, value)
}

pub(crate) fn canonical_activation_delegation_json_and_digest(
    receipt: &ExternalPoolProviderActivationDelegationReceipt,
) -> Result<(String, String)> {
    receipt_digest(receipt, "delegation_digest", DELEGATION_RECEIPT_DOMAIN)
}

pub(crate) fn canonical_activation_candidate_json_and_digest(
    receipt: &ExternalPoolProviderActivationCandidateReceipt,
) -> Result<(String, String)> {
    receipt_digest(receipt, "candidate_digest", CANDIDATE_RECEIPT_DOMAIN)
}

pub(crate) fn canonical_activation_delegation_revocation_json_and_digest(
    receipt: &ExternalPoolProviderActivationDelegationRevocationReceipt,
) -> Result<(String, String)> {
    receipt_digest(receipt, "revocation_digest", REVOCATION_RECEIPT_DOMAIN)
}

pub(crate) fn external_pool_activation_service_actor_id(
    provider_id: &str,
    provider_binding_id: &str,
    provider_binding_digest: &str,
    route_adapter_projection_id: &str,
) -> Result<String> {
    #[derive(Serialize)]
    struct Identity<'a> {
        provider_id: &'a str,
        provider_binding_id: &'a str,
        provider_binding_digest: &'a str,
        route_adapter_projection_id: &'a str,
    }
    Ok(format!(
        "external_pool_platform_dispatch_service_{}",
        domain_digest(
            SERVICE_ACTOR_ID_DOMAIN,
            &Identity {
                provider_id,
                provider_binding_id,
                provider_binding_digest,
                route_adapter_projection_id,
            },
        )?
    ))
}

pub(crate) fn logical_projection_compatibility_digest(
    provider_binding_id: &str,
    provider_binding_digest: &str,
    registry_release_id: &str,
    registry_release_digest: &str,
    logical_adapter_binding_digest: &str,
    route_adapter_projection_id: &str,
) -> Result<String> {
    #[derive(Serialize)]
    struct Identity<'a> {
        provider_binding_id: &'a str,
        provider_binding_digest: &'a str,
        registry_release_id: &'a str,
        registry_release_digest: &'a str,
        logical_adapter_binding_digest: &'a str,
        route_adapter_projection_id: &'a str,
    }
    domain_digest(
        COMPATIBILITY_DOMAIN,
        &Identity {
            provider_binding_id,
            provider_binding_digest,
            registry_release_id,
            registry_release_digest,
            logical_adapter_binding_digest,
            route_adapter_projection_id,
        },
    )
}

fn receipt_digest<T: Serialize>(value: &T, field: &str, domain: &[u8]) -> Result<(String, String)> {
    let object = serde_json::to_value(value)?;
    let mut projection = object
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("activation receipt must be an object"))?
        .clone();
    if projection
        .insert(field.into(), serde_json::Value::String(String::new()))
        .is_none()
    {
        bail!("activation receipt lacks digest field");
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
