use anyhow::{bail, ensure, Result};
use chrono::{DateTime, SecondsFormat};

use super::{
    canonical::{
        canonical_user_node_provider_binding_material_digest,
        canonical_user_node_provider_binding_receipt_json_and_digest,
        canonical_user_node_provider_binding_request_digest, derive_user_node_provider_binding_id,
    },
    types::*,
};

impl UserNodeProviderBindingMaterial {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        provider_id: String,
        provider_genesis_digest: String,
        node_id: String,
        owner_user_id: String,
        installation_identity_digest: String,
        endpoint_installation_binding_digest: String,
        source_endpoint_credential_id: String,
        source_endpoint_credential_revision: i64,
        source_endpoint_credential_digest: String,
        source_consent_receipt_id: String,
        source_consent_policy_revision: i64,
        source_consent_policy_digest: String,
        source_authorization_ref: String,
        source_authorization_revision: i64,
        source_authorization_digest: String,
        idempotency_scope: String,
        idempotency_key: String,
        recorded_at: String,
    ) -> Result<Self> {
        let binding_id = derive_user_node_provider_binding_id(
            &provider_id,
            &node_id,
            &provider_genesis_digest,
            &installation_identity_digest,
            &endpoint_installation_binding_digest,
        )?;
        let request_digest = canonical_user_node_provider_binding_request_digest(
            &provider_id,
            &node_id,
            &owner_user_id,
            USER_NODE_PROVIDER_BINDING_CONFIRMATION,
            &idempotency_scope,
            &idempotency_key,
        )?;
        let material = Self {
            binding_id,
            provider_id,
            provider_genesis_policy_revision: USER_NODE_PROVIDER_BINDING_GENESIS_POLICY_REVISION,
            provider_genesis_digest,
            node_id,
            owner_user_id,
            installation_identity_digest,
            endpoint_installation_binding_digest,
            source_endpoint_credential_id,
            source_endpoint_credential_revision,
            source_endpoint_credential_digest,
            source_consent_receipt_id,
            source_consent_policy_revision,
            source_consent_policy_digest,
            source_authorization_ref,
            source_authorization_revision,
            source_authorization_digest,
            confirmation: USER_NODE_PROVIDER_BINDING_CONFIRMATION.to_string(),
            idempotency_scope,
            idempotency_key,
            request_digest,
            bound_at: recorded_at.clone(),
            recorded_at,
            binding_effect: USER_NODE_PROVIDER_BINDING_IDENTITY_EFFECT.to_string(),
            provider_effect: USER_NODE_PROVIDER_BINDING_NO_EFFECT.to_string(),
            capacity_effect: USER_NODE_PROVIDER_BINDING_NO_EFFECT.to_string(),
            offer_effect: USER_NODE_PROVIDER_BINDING_NO_EFFECT.to_string(),
            readiness_effect: USER_NODE_PROVIDER_BINDING_NO_EFFECT.to_string(),
            route_effect: USER_NODE_PROVIDER_BINDING_NO_EFFECT.to_string(),
            execution_effect: USER_NODE_PROVIDER_BINDING_NO_EFFECT.to_string(),
            settlement_effect: USER_NODE_PROVIDER_BINDING_NO_EFFECT.to_string(),
        };
        validate_material(&material)?;
        Ok(material)
    }
}

pub(crate) fn build_user_node_provider_binding_receipt(
    material: UserNodeProviderBindingMaterial,
) -> Result<UserNodeProviderBindingReceiptV1> {
    validate_material(&material)?;
    let binding_material_digest = canonical_user_node_provider_binding_material_digest(&material)?;
    let mut receipt = UserNodeProviderBindingReceiptV1 {
        schema: USER_NODE_PROVIDER_BINDING_SCHEMA_V1.to_string(),
        binding_digest: String::new(),
        binding_material_digest,
        canonicalization: USER_NODE_PROVIDER_BINDING_CANONICALIZATION.to_string(),
        digest_algorithm: USER_NODE_PROVIDER_BINDING_DIGEST_ALGORITHM.to_string(),
        binding: material,
    };
    receipt.binding_digest =
        canonical_user_node_provider_binding_receipt_json_and_digest(&receipt)?.1;
    validate_user_node_provider_binding_receipt(&receipt)?;
    Ok(receipt)
}

pub(crate) fn validate_user_node_provider_binding_receipt(
    receipt: &UserNodeProviderBindingReceiptV1,
) -> Result<()> {
    ensure!(
        receipt.schema() == USER_NODE_PROVIDER_BINDING_SCHEMA_V1
            && receipt.canonicalization() == USER_NODE_PROVIDER_BINDING_CANONICALIZATION
            && receipt.digest_algorithm() == USER_NODE_PROVIDER_BINDING_DIGEST_ALGORITHM,
        "user-node Provider binding receipt metadata is unsupported"
    );
    digest(receipt.binding_digest())?;
    digest(receipt.binding_material_digest())?;
    validate_material(receipt.binding())?;
    let material_digest = canonical_user_node_provider_binding_material_digest(receipt.binding())?;
    let (_, binding_digest) =
        canonical_user_node_provider_binding_receipt_json_and_digest(receipt)?;
    ensure!(
        material_digest == receipt.binding_material_digest()
            && binding_digest == receipt.binding_digest(),
        "user-node Provider binding receipt digest mismatch"
    );
    Ok(())
}

pub(crate) fn user_node_provider_binding_json_is_canonical(value: &str) -> bool {
    user_node_provider_binding_receipt_from_json(value).is_ok()
}

pub(crate) fn user_node_provider_binding_receipt_from_json(
    value: &str,
) -> Result<UserNodeProviderBindingReceiptV1> {
    let receipt = serde_json::from_str::<UserNodeProviderBindingReceiptV1>(value)?;
    validate_user_node_provider_binding_receipt(&receipt)?;
    ensure!(
        receipt.binding_json()? == value,
        "user-node Provider binding JSON is not canonical"
    );
    Ok(receipt)
}

fn validate_material(material: &UserNodeProviderBindingMaterial) -> Result<()> {
    for value in [
        material.provider_id(),
        material.node_id(),
        material.owner_user_id(),
        material.source_endpoint_credential_id(),
        material.source_consent_receipt_id(),
        material.source_authorization_ref(),
    ] {
        identifier(value, 160)?;
    }
    identifier(material.idempotency_scope(), 200)?;
    identifier(material.idempotency_key(), 160)?;
    for value in [
        material.binding_id(),
        material.provider_genesis_digest(),
        material.installation_identity_digest(),
        material.endpoint_installation_binding_digest(),
        material.source_endpoint_credential_digest(),
        material.source_consent_policy_digest(),
        material.source_authorization_digest(),
        material.request_digest(),
    ] {
        digest(value)?;
    }
    ensure!(
        material.provider_genesis_policy_revision()
            == USER_NODE_PROVIDER_BINDING_GENESIS_POLICY_REVISION
            && positive_revision(material.source_endpoint_credential_revision())
            && positive_revision(material.source_consent_policy_revision())
            && material.source_authorization_revision()
                == material.source_consent_policy_revision(),
        "user-node Provider binding revision mismatch"
    );
    canonical_nanos(material.recorded_at())?;
    ensure!(
        material.bound_at() == material.recorded_at()
            && material.confirmation() == USER_NODE_PROVIDER_BINDING_CONFIRMATION
            && material.binding_effect() == USER_NODE_PROVIDER_BINDING_IDENTITY_EFFECT
            && [
                material.provider_effect(),
                material.capacity_effect(),
                material.offer_effect(),
                material.readiness_effect(),
                material.route_effect(),
                material.execution_effect(),
                material.settlement_effect(),
            ]
            .into_iter()
            .all(|effect| effect == USER_NODE_PROVIDER_BINDING_NO_EFFECT),
        "user-node Provider binding timestamp or effects are invalid"
    );
    let binding_id = derive_user_node_provider_binding_id(
        material.provider_id(),
        material.node_id(),
        material.provider_genesis_digest(),
        material.installation_identity_digest(),
        material.endpoint_installation_binding_digest(),
    )?;
    let request_digest = canonical_user_node_provider_binding_request_digest(
        material.provider_id(),
        material.node_id(),
        material.owner_user_id(),
        material.confirmation(),
        material.idempotency_scope(),
        material.idempotency_key(),
    )?;
    ensure!(
        binding_id == material.binding_id() && request_digest == material.request_digest(),
        "user-node Provider binding identity or request digest mismatch"
    );
    Ok(())
}

fn identifier(value: &str, max: usize) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("user-node Provider binding identifier is invalid");
    }
    Ok(())
}

fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("user-node Provider binding digest is invalid");
    }
    Ok(())
}

fn positive_revision(value: i64) -> bool {
    (1..=9_007_199_254_740_991).contains(&value)
}

fn canonical_nanos(value: &str) -> Result<()> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value {
        bail!("user-node Provider binding timestamp is not canonical UTC nanoseconds");
    }
    Ok(())
}
