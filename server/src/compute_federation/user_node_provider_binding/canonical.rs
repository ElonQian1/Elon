use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{
    UserNodeProviderBindingMaterial, UserNodeProviderBindingReceiptV1,
    USER_NODE_PROVIDER_BINDING_ID_DOMAIN_V1, USER_NODE_PROVIDER_BINDING_MATERIAL_DOMAIN_V1,
    USER_NODE_PROVIDER_BINDING_MAX_JSON_BYTES, USER_NODE_PROVIDER_BINDING_RECEIPT_DOMAIN_V1,
    USER_NODE_PROVIDER_BINDING_REQUEST_DOMAIN_V1,
};

#[derive(Serialize)]
struct BindingIdentityMaterial<'a> {
    provider_id: &'a str,
    node_id: &'a str,
    provider_genesis_digest: &'a str,
    installation_identity_digest: &'a str,
    endpoint_installation_binding_digest: &'a str,
}

#[derive(Serialize)]
struct BindingRequestMaterial<'a> {
    provider_id: &'a str,
    node_id: &'a str,
    owner_user_id: &'a str,
    confirmation: &'a str,
    idempotency_scope: &'a str,
    idempotency_key: &'a str,
}

pub(super) fn derive_user_node_provider_binding_id(
    provider_id: &str,
    node_id: &str,
    provider_genesis_digest: &str,
    installation_identity_digest: &str,
    endpoint_installation_binding_digest: &str,
) -> Result<String> {
    domain_digest(
        USER_NODE_PROVIDER_BINDING_ID_DOMAIN_V1,
        &BindingIdentityMaterial {
            provider_id,
            node_id,
            provider_genesis_digest,
            installation_identity_digest,
            endpoint_installation_binding_digest,
        },
    )
}

pub(crate) fn canonical_user_node_provider_binding_request_digest(
    provider_id: &str,
    node_id: &str,
    owner_user_id: &str,
    confirmation: &str,
    idempotency_scope: &str,
    idempotency_key: &str,
) -> Result<String> {
    domain_digest(
        USER_NODE_PROVIDER_BINDING_REQUEST_DOMAIN_V1,
        &BindingRequestMaterial {
            provider_id,
            node_id,
            owner_user_id,
            confirmation,
            idempotency_scope,
            idempotency_key,
        },
    )
}

pub(super) fn canonical_user_node_provider_binding_material_digest(
    material: &UserNodeProviderBindingMaterial,
) -> Result<String> {
    domain_digest(USER_NODE_PROVIDER_BINDING_MATERIAL_DOMAIN_V1, material)
}

pub(crate) fn canonical_user_node_provider_binding_receipt_json_and_digest(
    receipt: &UserNodeProviderBindingReceiptV1,
) -> Result<(String, String)> {
    let value = serde_json::to_value(receipt)?;
    let mut projection = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("user-node binding receipt must be an object"))?
        .clone();
    if projection
        .insert(
            "binding_digest".to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("user-node binding receipt lacks binding_digest");
    }
    let digest = domain_digest(USER_NODE_PROVIDER_BINDING_RECEIPT_DOMAIN_V1, &projection)?;
    let json = canonical_json(receipt)?;
    Ok((json, digest))
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, USER_NODE_PROVIDER_BINDING_MAX_JSON_BYTES)
        .map(|(json, _)| json)
}

fn domain_digest<T: Serialize + ?Sized>(domain: &str, value: &T) -> Result<String> {
    let json = canonical_json(value)?;
    let mut digest = Sha256::new();
    digest.update(domain.as_bytes());
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
